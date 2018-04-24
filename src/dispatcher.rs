use std::sync::Arc;

use {ErrorKind, HandleRequest, HandlerOptions, Result};

// TODO: move
pub struct RequestHandler {}
impl RequestHandler {
    fn new<H, D, E>(handler: H, options: HandlerOptions<H, D, E>) -> Self
    where
        H: HandleRequest,
    {
        unimplemented!()
    }
}

#[derive(Debug, Clone)]
pub struct Dispatcher {
    trie: Arc<Trie>,
}

#[derive(Debug)]
pub struct DispatcherBuilder {
    trie: Trie,
}
impl DispatcherBuilder {
    pub fn new() -> Self {
        DispatcherBuilder { trie: Trie(None) }
    }

    pub fn register_handler<H, D, E>(
        &mut self,
        handler: H,
        options: HandlerOptions<H, D, E>,
    ) -> Result<()>
    where
        H: HandleRequest,
    {
        let method = H::METHOD;
        let path = track!(Path::parse(H::PATH))?;
        let handler = RequestHandler::new(handler, options);
        track!(self.trie.register(method, path, handler); method, H::PATH)?;
        Ok(())
    }

    pub fn finish(self) -> Dispatcher {
        Dispatcher {
            trie: Arc::new(self.trie),
        }
    }
}

type Method = &'static str;

#[derive(Debug)]
struct Trie(Option<TrieNode>);
impl Trie {
    fn register(&mut self, method: Method, path: Path, handler: RequestHandler) -> Result<()> {
        let mut segments = path.0.into_iter().peekable();
        if self.0.is_none() {
            let segment = track_assert_some!(segments.peek(), ErrorKind::Other);
            let root = match *segment {
                Segment::Val(v) => TrieNode::Val {
                    segments: vec![(v, TrieNodeVal::default())],
                },
                Segment::Any => TrieNode::Any {
                    child: None,
                    handlers: Vec::new(),
                },
                Segment::AllTheRest => TrieNode::AllTheRest {
                    handlers: Vec::new(),
                },
            };
            self.0 = Some(root);
        }
        panic!()
    }
}

#[derive(Debug, Default)]
struct TrieNodeVal {
    child: Option<Box<TrieNode>>,
    handlers: Vec<(Method, ())>,
}

#[derive(Debug)]
enum TrieNode {
    Val {
        segments: Vec<(&'static str, TrieNodeVal)>,
    },
    Any {
        child: Option<Box<TrieNode>>,
        handlers: Vec<(Method, ())>,
    },
    AllTheRest {
        handlers: Vec<(Method, ())>,
    },
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
        for segment in path.split('/') {
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

#[derive(Debug)]
enum Segment {
    Val(&'static str),
    Any,
    AllTheRest,
}
