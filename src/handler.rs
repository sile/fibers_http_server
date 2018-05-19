use bytecodec::EncodeExt;
use bytecodec::io::{IoDecodeExt, ReadBuf};
use bytecodec::marker::Never;
use factory::{DefaultFactory, Factory};
use futures::{self, Future, Poll};
use httpcodec::{BodyDecode, BodyEncode, ResponseEncoder};
use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

use response::ResEncoder;
use {Error, Req, Res, Result};

/// `HandleRequest` allows for handling HTTP requests.
pub trait HandleRequest: Sized + Send + Sync + 'static {
    /// The method that the handler can handle.
    const METHOD: &'static str;

    /// The request path that the handler can handle.
    ///
    /// `*` and `**` in the path have the special meanings as follows:
    /// - `*` matches any path segment (i.e., regarded as a wildcard)
    /// - `**` matches all remaining parts of a path
    const PATH: &'static str;

    /// The type of the request bodies.
    type ReqBody: Send + 'static;

    /// The type of the response bodies.
    type ResBody: Send + 'static;

    /// Request body decoder.
    type Decoder: BodyDecode<Item = Self::ReqBody> + Send + 'static;

    /// Response body encoder.
    type Encoder: BodyEncode<Item = Self::ResBody> + Send + 'static;

    /// `Future` that represents reply to a request.
    type Reply: Future<Item = Res<Self::ResBody>, Error = Never> + Send + 'static;

    /// Handles the head part of a request.
    ///
    /// If a `Some(..)` value is returned, the invocation of `handle_request` method will be skipped.
    ///
    /// The default implementation always returns `None`.
    #[allow(unused_variables)]
    fn handle_request_head(&self, req: &Req<()>) -> Option<Res<Self::ResBody>> {
        None
    }

    /// Handles a request.
    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply;

    /// Handles an error occurred while decoding the body of a request.
    ///
    /// The default implementation always returns `None`
    /// (i.e., the default error response will be returned to the HTTP client).
    #[allow(unused_variables)]
    fn handle_decoding_error(&self, req: Req<()>, error: &Error) -> Option<Res<Self::ResBody>> {
        None
    }
}

/// Options for a request handler.
#[derive(Debug)]
pub struct HandlerOptions<H, D, E> {
    _handler: PhantomData<H>,
    decoder_factory: D,
    encoder_factory: E,
}
#[cfg_attr(feature = "cargo-clippy", allow(new_without_default_derive))]
impl<H> HandlerOptions<H, (), ()> {
    /// Makes a new `HandlerOptions` instance.
    pub fn new() -> Self {
        HandlerOptions {
            _handler: PhantomData,
            decoder_factory: (),
            encoder_factory: (),
        }
    }
}
impl<H, D, E> HandlerOptions<H, D, E>
where
    H: HandleRequest,
{
    /// Specifies the decoder factory that the handler will use.
    pub fn decoder<F>(self, decoder_factory: F) -> HandlerOptions<H, F, E>
    where
        F: Factory<Item = H::Decoder>,
    {
        HandlerOptions {
            _handler: self._handler,
            decoder_factory,
            encoder_factory: self.encoder_factory,
        }
    }

    /// Specifies that the handler will use the default decoder factory.
    pub fn default_decoder(self) -> HandlerOptions<H, DefaultFactory<H::Decoder>, E>
    where
        H::Decoder: Default,
    {
        self.decoder(Default::default())
    }

    /// Specifies the encoder factory that the handler will use.
    pub fn encoder<F>(self, encoder_factory: F) -> HandlerOptions<H, D, F>
    where
        F: Factory<Item = H::Encoder>,
    {
        HandlerOptions {
            _handler: self._handler,
            decoder_factory: self.decoder_factory,
            encoder_factory,
        }
    }

    /// Specifies that the handler will use the default encoder factory.
    pub fn default_encoder(self) -> HandlerOptions<H, D, DefaultFactory<H::Encoder>>
    where
        H::Encoder: Default,
    {
        self.encoder(Default::default())
    }
}
impl<H> Default for HandlerOptions<H, DefaultFactory<H::Decoder>, DefaultFactory<H::Encoder>>
where
    H: HandleRequest,
    H::Decoder: Default,
    H::Encoder: Default,
{
    fn default() -> Self {
        HandlerOptions::new().default_decoder().default_encoder()
    }
}

pub trait HandleInput {
    fn init(&mut self, req: Req<()>) -> Result<()>;

    fn handle_input(&mut self, buf: &mut ReadBuf<Vec<u8>>) -> Result<Option<BoxReply>>;

    fn is_closed(&self) -> bool;
}

struct InputHandler<H: HandleRequest> {
    req_handler: Arc<H>,
    req_head: Option<Req<()>>,
    res: Option<Res<H::ResBody>>,
    decoder: H::Decoder,
    encoder: Option<H::Encoder>,
    is_closed: bool,
}
impl<H: HandleRequest> HandleInput for InputHandler<H> {
    fn init(&mut self, req: Req<()>) -> Result<()> {
        if let Some(res) = self.req_handler.handle_request_head(&req) {
            self.res = Some(res);
            self.is_closed = true;
        } else if let Err(e) = self.decoder.initialize(&req.header()) {
            let e = track!(Error::from(e));
            if let Some(res) = self.req_handler.handle_decoding_error(req, &e) {
                self.res = Some(res);
                self.is_closed = true;
            } else {
                return Err(e);
            }
        } else {
            self.req_head = Some(req);
        }
        Ok(())
    }

    fn handle_input(&mut self, buf: &mut ReadBuf<Vec<u8>>) -> Result<Option<BoxReply>> {
        if let Some(res) = self.res.take() {
            let encoder = self.encoder.take().expect("Never fails");
            return Ok(Some(BoxReply::new::<_, H>(futures::finished(res), encoder)));
        }

        match self.decoder.decode_from_read_buf(buf) {
            Err(e) => {
                let e = track!(Error::from(e));
                let req = self.req_head.take().expect("Never fails");
                if let Some(res) = self.req_handler.handle_decoding_error(req, &e) {
                    self.is_closed = true;
                    self.res = Some(res);
                    return self.handle_input(buf);
                } else {
                    Err(e)
                }
            }
            Ok(None) => Ok(None),
            Ok(Some(body)) => {
                let req = self.req_head
                    .take()
                    .expect("Never fails")
                    .map_body(|()| body);
                let reply = self.req_handler.handle_request(req);
                let encoder = self.encoder.take().expect("Never fails");
                Ok(Some(BoxReply::new::<_, H>(reply, encoder)))
            }
        }
    }

    fn is_closed(&self) -> bool {
        self.is_closed
    }
}

pub struct RequestHandlerInstance(Box<HandleInput + Send + 'static>);
impl HandleInput for RequestHandlerInstance {
    fn init(&mut self, req: Req<()>) -> Result<()> {
        self.0.init(req)
    }

    fn handle_input(&mut self, buf: &mut ReadBuf<Vec<u8>>) -> Result<Option<BoxReply>> {
        self.0.handle_input(buf)
    }

    fn is_closed(&self) -> bool {
        self.0.is_closed()
    }
}
impl fmt::Debug for RequestHandlerInstance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RequestHandlerInstance(_)")
    }
}

pub struct RequestHandlerFactory {
    inner: Box<Fn() -> RequestHandlerInstance + Send + Sync + 'static>,
}
impl RequestHandlerFactory {
    pub fn new<H, D, E>(req_handler: H, options: HandlerOptions<H, D, E>) -> Self
    where
        H: HandleRequest,
        D: Factory<Item = H::Decoder> + Send + Sync + 'static,
        E: Factory<Item = H::Encoder> + Send + Sync + 'static,
    {
        let req_handler = Arc::new(req_handler);
        let f = move || {
            let handler = InputHandler {
                req_handler: Arc::clone(&req_handler),
                req_head: None,
                res: None,
                decoder: options.decoder_factory.create(),
                encoder: Some(options.encoder_factory.create()),
                is_closed: false,
            };
            RequestHandlerInstance(Box::new(handler))
        };
        RequestHandlerFactory { inner: Box::new(f) }
    }

    pub fn create(&self) -> RequestHandlerInstance {
        (self.inner)()
    }
}
impl fmt::Debug for RequestHandlerFactory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RequestHandlerFactory {{ .. }}")
    }
}

/// An alias of the typical `Future` that can be used as the result of `HandleRequest::handle_request` method.
pub type Reply<T> = Box<Future<Item = Res<T>, Error = Never> + Send + 'static>;

pub struct BoxReply(Box<Future<Item = ResEncoder, Error = Never> + Send + 'static>);
impl BoxReply {
    fn new<F, H>(reply: F, encoder: H::Encoder) -> Self
    where
        F: Future<Item = Res<H::ResBody>, Error = Never> + Send + 'static,
        H: HandleRequest,
    {
        let future = reply.and_then(move |res| {
            let body_encoder = Box::new(encoder);
            let encoder = ResponseEncoder::new(body_encoder).last_item(res.0);
            futures::finished(ResEncoder::new(encoder))
        });
        BoxReply(Box::new(future))
    }
}
impl Future for BoxReply {
    type Item = ResEncoder;
    type Error = Never;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}
impl fmt::Debug for BoxReply {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BoxReply(_)")
    }
}
