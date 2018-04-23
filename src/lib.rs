extern crate bytecodec;
extern crate fibers;
extern crate futures;
extern crate httpcodec;
#[macro_use]
extern crate trackable;

pub use error::{Error, ErrorKind};

pub mod request;
pub mod status;

mod error;

pub type Result<T> = std::result::Result<T, Error>;
