extern crate bytecodec;
extern crate factory;
extern crate fibers;
extern crate fibers_tasque;
extern crate futures;
extern crate httpcodec;
extern crate prometrics;
#[macro_use]
extern crate slog;
#[macro_use]
extern crate trackable;
extern crate url;

pub use error::{Error, ErrorKind};
pub use handler::{HandleRequest, HandlerOptions, Reply};
pub use request::Req;
pub use response::Res;
pub use server::{Server, ServerBuilder};
pub use status::Status;

pub mod metrics;

mod bc; // TODO: delete
mod connection;
mod dispatcher;
mod error;
mod handler;
mod header;
mod request;
mod response;
mod server;
mod status;

// TODO: metrics_handler

/// This crate specific `Result` type.
pub type Result<T> = std::result::Result<T, Error>;
