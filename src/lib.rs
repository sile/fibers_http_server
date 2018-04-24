extern crate bytecodec;
extern crate factory;
extern crate fibers;
extern crate fibers_tasque;
extern crate futures;
extern crate httpcodec;
#[macro_use]
extern crate slog;
#[macro_use]
extern crate trackable;

pub use error::{Error, ErrorKind};
pub use handler::{HandleRequest, HandlerOptions, Reply};
pub use request::Req;
pub use response::Res;
pub use server::{Server, ServerBuilder};
pub use status::Status;

mod dispatcher;
mod error;
mod handler;
mod request;
mod response;
mod server;
mod status;

pub type Result<T> = std::result::Result<T, Error>;
