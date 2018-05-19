use factory::Factory;
use std::result::Result as StdResult;
use std::sync::Arc;
use url::Url;

use handler::{RequestHandlerFactory, RequestHandlerInstance};
use {ErrorKind, HandleRequest, HandlerOptions, Req, Result, Status};

type Method = &'static str;

#[derive(Debug, Clone)]
pub struct Dispatcher {
    trie: Arc<Trie>,
}
impl Dispatcher {
    pub fn dispatch(&self, req: &Req<()>) -> StdResult<RequestHandlerInstance, Status> {
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
        let handler = RequestHandlerFactory::new(handler, options);
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
        handler: RequestHandlerFactory,
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

    fn dispatch(&self, method: &str, url: &Url) -> StdResult<RequestHandlerInstance, Status> {
        let mut node = &self.0;
        'root: for actual in url.path_segments().expect("Never fails") {
            for expected in &node.segments {
                match *expected {
                    (Segment::Any, ref next) => {
                        node = next;
                        continue 'root;
                    }
                    (Segment::AllTheRest, ref next) => {
                        node = next;
                        break 'root;
                    }
                    (Segment::Val(v), ref next) => {
                        if v == actual {
                            node = next;
                            continue 'root;
                        }
                    }
                }
            }
            return Err(Status::NotFound);
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
    handlers: Vec<(Method, RequestHandlerFactory)>,
}

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

#[cfg(test)]
mod test {
    use bytecodec::value::NullDecoder;
    use futures::future::ok;
    use httpcodec::{BodyDecoder, NoBodyEncoder};
    use url::Url;

    use super::*;
    use {Reply, Req, Res, Status};

    macro_rules! define_handler {
        ($handler:ident, $method:expr, $path:expr) => {
            struct $handler;
            impl HandleRequest for $handler {
                const METHOD: &'static str = $method;
                const PATH: &'static str = $path;

                type ReqBody = ();
                type ResBody = ();
                type Decoder = BodyDecoder<NullDecoder>;
                type Encoder = NoBodyEncoder;
                type Reply = Reply<Self::ResBody>;

                fn handle_request(&self, _req: Req<Self::ReqBody>) -> Self::Reply {
                    Box::new(ok(Res::new(Status::Ok, ())))
                }
            }
        };
    }

    define_handler!(Handler0, "GET", "/");
    define_handler!(Handler1, "GET", "/foo/bar");
    define_handler!(Handler2, "PUT", "/foo/bar/");
    define_handler!(Handler3, "GET", "/aaa/*/bbb");
    define_handler!(Handler4, "GET", "/111/**");

    fn url(path: &str) -> Url {
        Url::parse(&format!("http://localhost{}", path)).unwrap()
    }

    #[test]
    fn dispatcher_works() {
        let mut builder = DispatcherBuilder::new();
        track_try_unwrap!(builder.register_handler(Handler0, Default::default()));
        track_try_unwrap!(builder.register_handler(Handler1, Default::default()));
        track_try_unwrap!(builder.register_handler(Handler2, Default::default()));
        track_try_unwrap!(builder.register_handler(Handler3, Default::default()));
        track_try_unwrap!(builder.register_handler(Handler4, Default::default()));

        let trie = builder.finish().trie;
        assert!(trie.dispatch("GET", &url("/")).is_ok());
        assert!(trie.dispatch("PUT", &url("/")).is_err());
        assert!(trie.dispatch("GET", &url("/f")).is_err());
        assert!(trie.dispatch("GET", &url("/foo/bar")).is_ok());
        assert!(trie.dispatch("GET", &url("/foo/bar/")).is_err());
        assert!(trie.dispatch("PUT", &url("/foo/bar/")).is_ok());
        assert!(trie.dispatch("GET", &url("/aaa/0/bbb")).is_ok());
        assert!(trie.dispatch("GET", &url("/aaa/012/bbb")).is_ok());
        assert!(trie.dispatch("GET", &url("/aaa/bbb")).is_err());
        assert!(trie.dispatch("GET", &url("/aaa/0/bbb/")).is_err());
        assert!(trie.dispatch("GET", &url("/111/")).is_ok());
        assert!(trie.dispatch("GET", &url("/111/222")).is_ok());
        assert!(trie.dispatch("GET", &url("/111/222/")).is_ok());
        assert!(trie.dispatch("GET", &url("/111/222/333")).is_ok());
    }
}
