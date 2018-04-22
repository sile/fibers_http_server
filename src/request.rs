use bytecodec::Decode;
use httpcodec::Request;

use Result;

pub trait HandleRequest {
    type Body;
    fn handle_header(&self, request: &Request<()>) {}
    fn handle_request(&self, request: Request<Self::Body>);
    fn body_decoder(&self) -> Box<Decode<Item = Self::Body> + Send + 'static>;
}

pub trait DispatchRequest {
    fn dispatch_request(&self, request: &Request<()>) -> RequestHandler;
}

#[derive(Debug)]
pub struct RequestHandler;
