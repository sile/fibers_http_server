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

    // TODO
    #[allow(unused_variables)]
    fn enable_async_decoding(&self, req: &Req<()>) -> bool {
        false
    }

    #[allow(unused_variables)]
    fn enable_async_encoding(&self, res: &Res<Self::ResBody>) -> bool {
        false
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
    pub fn done(response: Res<T::ResBody>) -> Self {
        unimplemented!()
    }
}

trait HandleStream {
    fn init(&mut self, req: Req<()>);
    fn read(&mut self, buf: &mut ReadBuf<Vec<u8>>) -> Result<Action>;
    fn write(&mut self, buf: &mut WriteBuf<Vec<u8>>) -> Result<Action>;
}

struct StreamHandler<H, D, E>
where
    H: HandleRequest,
{
    request_handler: Arc<H>,
    req_head: Option<Req<()>>,
    response: Option<Res<H::ResBody>>,
    decoder: D,
    encoder: E,
}
impl<H, D, E> HandleStream for StreamHandler<H, D, E>
where
    H: HandleRequest,
    D: Decode<Item = H::ReqBody>,
    E: Encode<Item = H::ResBody>,
{
    fn init(&mut self, req: Req<()>) {
        self.response = self.request_handler.handle_request_head(&req);
        self.req_head = Some(req);
    }

    fn read(&mut self, buf: &mut ReadBuf<Vec<u8>>) -> Result<Action> {
        if self.response.is_some() {
            return Ok(Action::NextPhase);
        }
        if let Some(body) = track!(self.decoder.decode_from_read_buf(buf))? {
            let req = self.req_head
                .take()
                .expect("Never fails")
                .map_body(|()| body);
            unimplemented!()
        } else {
            Ok(Action::Continue)
        }
    }

    fn write(&mut self, buf: &mut WriteBuf<Vec<u8>>) -> Result<Action> {
        unimplemented!()
    }
}

#[derive(Debug)]
pub enum Action {
    Continue,
    NextPhase,
    CloseStream,
}

pub struct StreamHandlerFactory {
    inner: Box<Fn() -> Box<HandleStream + Send + 'static> + Send + 'static>,
}
impl StreamHandlerFactory {
    pub fn new<H, D, E>(request_handler: H, options: HandlerOptions<H, D, E>) -> Self
    where
        H: HandleRequest,
        D: Factory<Item = H::Decoder> + Send + 'static,
        E: Factory<Item = H::Encoder> + Send + 'static,
    {
        let request_handler = Arc::new(request_handler);
        let f = move || {
            let request_handler = Arc::clone(&request_handler);
            let decoder = options.decoder_factory.create();
            let encoder = options.encoder_factory.create();
            let stream_handler = StreamHandler {
                request_handler,
                response: None,
                decoder,
                encoder,
            };
            let stream_handler: Box<HandleStream + Send + 'static> = Box::new(stream_handler);
            stream_handler
        };
        StreamHandlerFactory { inner: Box::new(f) }
    }
}
impl fmt::Debug for StreamHandlerFactory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "StreamHandlerFactory {{ .. }}")
    }
}
