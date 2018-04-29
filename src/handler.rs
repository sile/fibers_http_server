use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;
use bytecodec::{Encode, EncodeExt};
use bytecodec::io::{IoDecodeExt, ReadBuf};
use bytecodec::marker::Never;
use factory::{DefaultFactory, Factory};
use futures::{Async, Future, Poll};
use httpcodec::{BodyDecode, BodyEncode, ResponseEncoder};

use {Error, ErrorKind, Req, Res, Result};

pub trait HandleRequest: Sized + Send + Sync + 'static {
    const METHOD: &'static str;
    const PATH: &'static str;

    type ReqBody: Send + 'static;
    type ResBody: Send + 'static;
    type Decoder: BodyDecode<Item = Self::ReqBody> + Send + 'static;
    type Encoder: BodyEncode<Item = Self::ResBody> + Send + 'static;

    #[allow(unused_variables)]
    fn handle_request_head(&self, req: &Req<()>) -> Option<Res<Self::ResBody>> {
        None
    }

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Reply<Self>;

    #[allow(unused_variables)]
    fn handle_decoding_error(&self, error: &Error) -> Option<Res<Self::ResBody>> {
        None
    }
}

#[derive(Debug)]
pub struct HandlerOptions<H, D, E> {
    _handler: PhantomData<H>,
    decoder_factory: D,
    encoder_factory: E,
}
impl<H> HandlerOptions<H, (), ()> {
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

    pub fn default_decoder(self) -> HandlerOptions<H, DefaultFactory<H::Decoder>, E>
    where
        H::Decoder: Default,
    {
        self.decoder(Default::default())
    }

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

enum ReplyInner<T: HandleRequest> {
    Done(Res<T::ResBody>),
}
impl<T: HandleRequest> fmt::Debug for ReplyInner<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ReplyInner::Done(_) => write!(f, "Done(_)"),
        }
    }
}

#[derive(Debug)]
pub struct Reply<T: HandleRequest>(ReplyInner<T>);
impl<T: HandleRequest> Reply<T> {
    pub fn done(res: Res<T::ResBody>) -> Self {
        Reply(ReplyInner::Done(res))
    }

    pub fn boxed(self, encoder: T::Encoder) -> BoxReply {
        // TODO: handle HEAD request
        match self.0 {
            ReplyInner::Done(res) => {
                let body_encoder = Box::new(encoder);
                let mut encoder = ResponseEncoder::new(body_encoder);
                track!(encoder.start_encoding(res.0)).expect("TODO: Use Lazy or ...");
                BoxReply(BoxReplyInner::Done(Some(Box::new(encoder.last()))))
            }
        }
    }
}

enum BoxReplyInner {
    Done(Option<Box<Encode<Item = Never> + Send + 'static>>),
}
impl fmt::Debug for BoxReplyInner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            BoxReplyInner::Done(_) => write!(f, "Done(_)"),
        }
    }
}

#[derive(Debug)]
pub struct BoxReply(BoxReplyInner);
impl Future for BoxReply {
    type Item = Box<Encode<Item = Never> + Send + 'static>;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0 {
            BoxReplyInner::Done(ref mut x) => {
                Ok(Async::Ready(x.take().expect("Cannot poll BoxReply twice")))
            }
        }
    }
}

trait HandleStream {
    // TODO: rename
    fn init(&mut self, req: Req<()>) -> Result<()>;
    fn handle_request(&mut self, buf: &mut ReadBuf<Vec<u8>>) -> Result<Option<BoxReply>>;
    fn is_closed(&self) -> bool;
}

struct StreamHandler<H>
where
    H: HandleRequest,
{
    req_handler: Arc<H>,
    req_head: Option<Req<()>>,
    reply: Option<Reply<H>>,
    decoder: H::Decoder,
    encoder: Option<H::Encoder>,
    is_closed: bool,
}
impl<H> HandleStream for StreamHandler<H>
where
    H: HandleRequest,
{
    fn init(&mut self, req: Req<()>) -> Result<()> {
        if let Some(res) = self.req_handler.handle_request_head(&req) {
            self.reply = Some(Reply::done(res));
            self.is_closed = true;
        } else {
            if let Err(e) = self.decoder.initialize(&req.header()) {
                let e = track!(Error::from(e));
                if let Some(res) = self.req_handler.handle_decoding_error(&e) {
                    self.reply = Some(Reply::done(res));
                    self.is_closed = true;
                } else {
                    return Err(e);
                }
            } else {
                self.req_head = Some(req);
            }
        }
        Ok(())
    }

    fn handle_request(&mut self, buf: &mut ReadBuf<Vec<u8>>) -> Result<Option<BoxReply>> {
        if let Some(reply) = self.reply.take() {
            let encoder = track_assert_some!(self.encoder.take(), ErrorKind::Other);
            return Ok(Some(reply.boxed(encoder)));
        }

        match self.decoder.decode_from_read_buf(buf) {
            Err(e) => {
                let e = track!(Error::from(e));
                if let Some(res) = self.req_handler.handle_decoding_error(&e) {
                    self.is_closed = true;
                    self.reply = Some(Reply::done(res));
                    return self.handle_request(buf);
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
                // TODO: check 'connection: close' header
                self.reply = Some(self.req_handler.handle_request(req));
                return self.handle_request(buf);
            }
        }
    }

    fn is_closed(&self) -> bool {
        self.is_closed
    }
}

pub struct BoxStreamHandler(Box<HandleStream + Send + 'static>);
impl BoxStreamHandler {
    pub fn init(&mut self, req: Req<()>) -> Result<()> {
        self.0.init(req)
    }
    pub fn handle_request(&mut self, buf: &mut ReadBuf<Vec<u8>>) -> Result<Option<BoxReply>> {
        self.0.handle_request(buf)
    }
    pub fn is_closed(&self) -> bool {
        self.0.is_closed()
    }
}
impl fmt::Debug for BoxStreamHandler {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BoxStreamHandler(_)")
    }
}

pub struct StreamHandlerFactory {
    inner: Box<Fn() -> BoxStreamHandler + Send + Sync + 'static>,
}
impl StreamHandlerFactory {
    pub fn new<H, D, E>(req_handler: H, options: HandlerOptions<H, D, E>) -> Self
    where
        H: HandleRequest,
        D: Factory<Item = H::Decoder> + Send + Sync + 'static,
        E: Factory<Item = H::Encoder> + Send + Sync + 'static,
    {
        let req_handler = Arc::new(req_handler);
        let f = move || {
            let req_handler = Arc::clone(&req_handler);
            let decoder = options.decoder_factory.create();
            let encoder = options.encoder_factory.create();
            let stream_handler = StreamHandler {
                req_handler,
                req_head: None,
                reply: None,
                decoder,
                encoder: Some(encoder),
                is_closed: false,
            };
            BoxStreamHandler(Box::new(stream_handler))
        };
        StreamHandlerFactory { inner: Box::new(f) }
    }

    pub fn create(&self) -> BoxStreamHandler {
        (self.inner)()
    }
}
impl fmt::Debug for StreamHandlerFactory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "StreamHandlerFactory {{ .. }}")
    }
}
