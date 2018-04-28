use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;
use bytecodec::{Decode, Encode};
use bytecodec::io::{IoDecodeExt, IoEncodeExt, ReadBuf, WriteBuf};
use factory::{DefaultFactory, Factory};
use httpcodec::{BodyDecode, BodyEncode};

use {Error, Req, Res, Result};

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

#[derive(Debug)]
pub struct Reply<T>(T);
impl<T: HandleRequest> Reply<T> {
    pub fn done(res: Res<T::ResBody>) -> Self {
        unimplemented!()
    }
}

trait HandleStream {
    fn init(&mut self, req: Req<()>) -> Result<()>;
    fn recv_request(&mut self, buf: &mut ReadBuf<Vec<u8>>) -> Result<Phase>;
    fn send_response(&mut self, buf: &mut WriteBuf<Vec<u8>>) -> Result<Phase>;
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
    encoder: H::Encoder,
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

    fn recv_request(&mut self, buf: &mut ReadBuf<Vec<u8>>) -> Result<Phase> {
        if self.reply.is_some() {
            return Ok(Phase::Send);
        }
        match self.decoder.decode_from_read_buf(buf) {
            Err(e) => {
                let e = track!(Error::from(e));
                if let Some(res) = self.req_handler.handle_decoding_error(&e) {
                    self.reply = Some(Reply::done(res));
                    self.is_closed = true;
                    Ok(Phase::Send)
                } else {
                    Err(e)
                }
            }
            Ok(None) => Ok(Phase::Recv),
            Ok(Some(body)) => {
                let req = self.req_head
                    .take()
                    .expect("Never fails")
                    .map_body(|()| body);
                self.reply = Some(self.req_handler.handle_request(req));
                Ok(Phase::Send)
            }
        }
    }

    fn send_response(&mut self, buf: &mut WriteBuf<Vec<u8>>) -> Result<Phase> {
        unimplemented!()
    }

    fn is_closed(&self) -> bool {
        self.is_closed
    }
}

#[derive(Debug)]
pub enum Phase {
    Recv,
    Send,
}

pub struct BoxStreamHandler(Box<HandleStream + Send + 'static>);
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
                encoder,
                is_closed: false,
            };
            BoxStreamHandler(Box::new(stream_handler))
        };
        StreamHandlerFactory { inner: Box::new(f) }
    }
}
impl fmt::Debug for StreamHandlerFactory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "StreamHandlerFactory {{ .. }}")
    }
}
