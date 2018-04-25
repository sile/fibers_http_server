use std::fmt;
use std::marker::PhantomData;
use bytecodec::{self, ByteCount, Decode, Encode, Eos};
use bytecodec::marker::Never;
use factory::{DefaultFactory, Factory};
use httpcodec::{BodyDecode, BodyEncode};

use {Error, Req, Res};

pub trait HandleRequest: Sized {
    const METHOD: &'static str;
    const PATH: &'static str;

    type ReqBody;
    type ResBody;
    type Decoder: BodyDecode<Item = Self::ReqBody>;
    type Encoder: BodyEncode<Item = Self::ResBody>;

    #[allow(unused_variables)]
    fn handle_request_head(&self, req: &Req<()>) -> Option<Res<Self::ResBody>> {
        None
    }

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Reply<Self>;

    #[allow(unused_variables)]
    fn handle_decoding_error(&self, error: &Error) -> Option<Res<Self::ResBody>> {
        None
    }

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
    // pub fn done(response: Response<T::ResBody>)
}

pub struct RequestHandler {}
impl RequestHandler {
    pub fn new<H, D, E>(handler: H, options: HandlerOptions<H, D, E>) -> Self
    where
        H: HandleRequest,
    {
        unimplemented!()
    }
}
impl Decode for RequestHandler {
    type Item = ();

    fn decode(&mut self, buf: &[u8], eos: Eos) -> bytecodec::Result<(usize, Option<Self::Item>)> {
        unimplemented!()
    }

    fn has_terminated(&self) -> bool {
        unimplemented!()
    }

    fn requiring_bytes(&self) -> ByteCount {
        unimplemented!()
    }
}
impl Encode for RequestHandler {
    type Item = Never;

    fn encode(&mut self, buf: &mut [u8], eos: Eos) -> bytecodec::Result<usize> {
        unimplemented!()
    }

    fn start_encoding(&mut self, item: Self::Item) -> bytecodec::Result<()> {
        unimplemented!()
    }

    fn is_idle(&self) -> bool {
        unimplemented!()
    }

    fn requiring_bytes(&self) -> ByteCount {
        unimplemented!()
    }
}
impl fmt::Debug for RequestHandler {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Requesthandler {{ .. }}")
    }
}
