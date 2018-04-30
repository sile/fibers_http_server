//! A tiny HTTP/1.1 server framework.
//!
//! # Examples
//!
//! ```
//! # extern crate bytecodec;
//! # extern crate fibers;
//! # extern crate fibers_http_server;
//! # extern crate futures;
//! # extern crate httpcodec;
//! use std::io::{Read, Write};
//! use std::net::TcpStream;
//! use std::thread;
//! use std::time::Duration;
//! use bytecodec::bytes::Utf8Encoder;
//! use bytecodec::value::NullDecoder;
//! use fibers::{Executor, Spawn, InPlaceExecutor};
//! use fibers_http_server::{HandleRequest, Reply, Req, Res, ServerBuilder, Status};
//! use futures::future::{ok, Future};
//! use httpcodec::{BodyDecoder, BodyEncoder};
//!
//! // Request handler
//! struct Hello;
//! impl HandleRequest for Hello {
//!     const METHOD: &'static str = "GET";
//!     const PATH: &'static str = "/hello";
//!
//!     type ReqBody = ();
//!     type ResBody = String;
//!     type Decoder = BodyDecoder<NullDecoder>;
//!     type Encoder = BodyEncoder<Utf8Encoder>;
//!     type Reply = Reply<Self::ResBody>;
//!
//!     fn handle_request(&self, _req: Req<Self::ReqBody>) -> Self::Reply {
//!         Box::new(ok(Res::new(Status::Ok, "hello".to_owned())))
//!     }
//! }
//!
//! # fn main() {
//! let addr = "127.0.0.1:14758".parse().unwrap();
//!
//! // HTTP server
//! thread::spawn(move || {
//!     let executor = InPlaceExecutor::new().unwrap();
//!     let mut builder = ServerBuilder::new(addr);
//!     builder.add_handler(Hello).unwrap();
//!     let server = builder.finish(executor.handle());
//!     executor.spawn(server.map_err(|e| panic!("{}", e)));
//!     executor.run().unwrap()
//! });
//! thread::sleep(Duration::from_millis(100));
//!
//! // HTTP client
//! let mut client = TcpStream::connect(addr).unwrap();
//! client
//!     .write_all(b"GET /hello HTTP/1.1\r\nContent-Length: 0\r\n\r\n")
//!     .unwrap();
//! thread::sleep(Duration::from_millis(100));
//!
//! let mut buf = [0; 1024];
//! let size = client.read(&mut buf).unwrap();
//! assert_eq!(
//!     &buf[..size],
//!     b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello".as_ref()
//! );
//! # }
//! ```
#![warn(missing_docs)]
extern crate bytecodec;
extern crate factory;
extern crate fibers;
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

mod connection;
mod dispatcher;
mod error;
mod handler;
mod header;
mod request;
mod response;
mod server;
mod status;

/// This crate specific `Result` type.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod test {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::thread;
    use std::time::Duration;
    use bytecodec::bytes::Utf8Encoder;
    use bytecodec::value::NullDecoder;
    use fibers::{Executor, InPlaceExecutor, Spawn};
    use futures::future::{ok, Future};
    use httpcodec::{BodyDecoder, BodyEncoder};

    use super::*;

    struct Hello;
    impl HandleRequest for Hello {
        const METHOD: &'static str = "GET";
        const PATH: &'static str = "/hello";

        type ReqBody = ();
        type ResBody = String;
        type Decoder = BodyDecoder<NullDecoder>;
        type Encoder = BodyEncoder<Utf8Encoder>;
        type Reply = Reply<Self::ResBody>;

        fn handle_request(&self, _req: Req<Self::ReqBody>) -> Self::Reply {
            Box::new(ok(Res::new(Status::Ok, "hello".to_owned())))
        }
    }

    #[test]
    fn it_works() {
        let addr = "127.0.0.1:14757".parse().unwrap();
        thread::spawn(move || {
            let executor = InPlaceExecutor::new().unwrap();
            let mut builder = ServerBuilder::new(addr);
            builder.add_handler(Hello).unwrap();
            let server = builder.finish(executor.handle());
            executor.spawn(server.map_err(|e| panic!("{}", e)));
            executor.run().unwrap()
        });
        thread::sleep(Duration::from_millis(100));

        let mut client = TcpStream::connect(addr).unwrap();
        client
            .write_all(b"GET /hello HTTP/1.1\r\nContent-Length: 0\r\n\r\n")
            .unwrap();
        thread::sleep(Duration::from_millis(100));

        let mut buf = [0; 1024];
        let size = client.read(&mut buf).unwrap();
        assert_eq!(
            &buf[..size],
            b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello".as_ref()
        );
    }
}
