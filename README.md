fibers_http_server
==================

[![fibers_http_server](https://img.shields.io/crates/v/fibers_http_server.svg)](https://crates.io/crates/fibers_http_server)
[![Documentation](https://docs.rs/fibers_http_server/badge.svg)](https://docs.rs/fibers_http_server)
[![Build Status](https://travis-ci.org/sile/fibers_http_server.svg?branch=master)](https://travis-ci.org/sile/fibers_http_server)
[![Code Coverage](https://codecov.io/gh/sile/fibers_http_server/branch/master/graph/badge.svg)](https://codecov.io/gh/sile/fibers_http_server/branch/master)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A tiny HTTP/1.1 server framework for Rust.

[Documentation](https://docs.rs/fibers_http_server)

Examples
---------

```rust
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;
use bytecodec::bytes::Utf8Encoder;
use bytecodec::value::NullDecoder;
use fibers::{Executor, Spawn, InPlaceExecutor};
use fibers_http_server::{HandleRequest, Reply, Req, Res, ServerBuilder, Status};
use futures::future::{ok, Future};
use httpcodec::{BodyDecoder, BodyEncoder};

// Request handler
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

let addr = "127.0.0.1:14758".parse().unwrap();

// HTTP server
thread::spawn(move || {
    let executor = InPlaceExecutor::new().unwrap();
    let mut builder = ServerBuilder::new(addr);
    builder.add_handler(Hello).unwrap();
    let server = builder.finish(executor.handle());
    executor.spawn(server.map_err(|e| panic!("{}", e)));
    executor.run().unwrap()
});
thread::sleep(Duration::from_millis(100));

// HTTP client
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
```
