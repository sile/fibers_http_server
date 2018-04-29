use std::result::Result as StdResult;
use std::sync::Arc;
use factory::Factory;
use url::Url;

use {ErrorKind, HandleRequest, HandlerOptions, Req, Result, Status};
use handler::{BoxStreamHandler, StreamHandlerFactory};

type Method = &'static str;

#[derive(Debug, Clone)]
pub struct Dispatcher {
    trie: Arc<Trie>,
}
impl Dispatcher {
    pub fn dispatch(&self, req: &Req<()>) -> StdResult<BoxStreamHandler, Status> {
        self.trie.dispatch(req.method(), req.url())
    }
}

#[derive(Debug)]
pub struct DispatcherBuilder {
    trie: Trie,
}
impl DispatcherBuilder {
    pub fn new() -> Self {
        DispatcherBuilder {
            trie: Trie::default(),
        }
    }

    pub fn register_handler<H, D, E>(
        &mut self,
        handler: H,
        options: HandlerOptions<H, D, E>,
    ) -> Result<()>
    where
        H: HandleRequest,
        D: Factory<Item = H::Decoder> + Send + Sync + 'static,
        E: Factory<Item = H::Encoder> + Send + Sync + 'static,
    {
        let method = H::METHOD;
        let path = track!(Path::parse(H::PATH))?;
        let handler = StreamHandlerFactory::new(handler, options);
        track!(self.trie.register(method, path, handler); method, H::PATH)?;
        Ok(())
    }

    pub fn finish(self) -> Dispatcher {
        Dispatcher {
            trie: Arc::new(self.trie),
        }
    }
}

#[derive(Debug, Default)]
struct Trie(TrieNode);
impl Trie {
    fn register(
        &mut self,
        method: Method,
        path: Path,
        handler: StreamHandlerFactory,
    ) -> Result<()> {
        let mut node = &mut self.0;
        let mut segments = path.0.into_iter().peekable();
        while let Some(segment) = segments.next() {
            match segment {
                Segment::Val(v) => {
                    let mut i = 0;
                    while i < node.segments.len() {
                        match node.segments[i].0 {
                            Segment::Any | Segment::AllTheRest => {
                                track_panic!(ErrorKind::InvalidInput)
                            }
                            Segment::Val(w) if v == w => {
                                break;
                            }
                            Segment::Val(_) => {
                                i += 1;
                            }
                        }
                    }
                    if i == node.segments.len() {
                        node.segments.push((segment, Box::new(TrieNode::default())));
                    }
                    node = &mut { node }.segments[i].1;
                }
                Segment::Any => {
                    if node.segments.is_empty() {
                        node.segments.push((segment, Box::new(TrieNode::default())));
                    } else {
                        track_assert_eq!(node.segments[0].0, Segment::Any, ErrorKind::InvalidInput);
                    }
                    node = &mut { node }.segments[0].1;
                }
                Segment::AllTheRest => {
                    if node.segments.is_empty() {
                        node.segments.push((segment, Box::new(TrieNode::default())));
                    } else {
                        track_assert_eq!(
                            node.segments[0].0,
                            Segment::AllTheRest,
                            ErrorKind::InvalidInput
                        );
                    }
                    node = &mut { node }.segments[0].1;
                    track_assert_eq!(segments.peek(), None, ErrorKind::InvalidInput);
                }
            }
        }
        track_assert!(
            node.handlers.iter().find(|x| x.0 == method).is_none(),
            ErrorKind::InvalidInput
        );
        node.handlers.push((method, handler));

        Ok(())
    }

    fn dispatch(&self, method: &str, url: &Url) -> StdResult<BoxStreamHandler, Status> {
        let mut node = &self.0;
        'root: for actual in url.path_segments().expect("Never fails") {
            for expected in &node.segments {
                match *expected {
                    (Segment::Any, ref next) => {
                        node = next;
                        break;
                    }
                    (Segment::AllTheRest, _) => {
                        break 'root;
                    }
                    (Segment::Val(v), ref next) => {
                        if v == actual {
                            node = next;
                            break;
                        } else {
                            return Err(Status::NotFound);
                        }
                    }
                }
            }
        }
        for handler in &node.handlers {
            if handler.0 == method {
                return Ok(handler.1.create());
            }
        }
        Err(Status::MethodNotAllowed)
    }
}

#[derive(Debug, Default)]
struct TrieNode {
    segments: Vec<(Segment, Box<TrieNode>)>,
    handlers: Vec<(Method, StreamHandlerFactory)>,
}
impl TrieNode {}

#[derive(Debug)]
struct Path(Vec<Segment>);
impl Path {
    fn parse(path: &'static str) -> Result<Path> {
        track_assert!(!path.is_empty(), ErrorKind::InvalidInput);
        track_assert_eq!(path.chars().nth(0), Some('/'), ErrorKind::InvalidInput; path);
        let mut segments = Vec::new();
        let mut is_last = false;
        for segment in path.split('/').skip(1) {
            track_assert!(
                !is_last,
                ErrorKind::InvalidInput,
                "'**' is allowed to be located only at the end of a path"; path
            );
            match segment {
                "*" => {
                    segments.push(Segment::Any);
                }
                "**" => {
                    segments.push(Segment::AllTheRest);
                    is_last = true;
                }
                _ => {
                    segments.push(Segment::Val(segment));
                }
            }
        }
        Ok(Path(segments))
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Segment {
    Val(&'static str),
    Any,
    AllTheRest,
}
