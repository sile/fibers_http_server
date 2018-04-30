extern crate bytecodec;
extern crate fibers;
extern crate fibers_http_server;
extern crate futures;
extern crate httpcodec;
extern crate slog;
extern crate sloggers;
#[macro_use]
extern crate trackable;

use bytecodec::bytes::Utf8Encoder;
use bytecodec::value::NullDecoder;
use fibers::{Executor, Spawn, ThreadPoolExecutor};
use fibers_http_server::{HandleRequest, Reply, Req, Res, ServerBuilder, Status};
use fibers_http_server::metrics::MetricsHandler;
use futures::future::ok;
use httpcodec::{BodyDecoder, BodyEncoder};
use sloggers::Build;
use sloggers::terminal::TerminalLoggerBuilder;
use sloggers::types::Severity;

fn main() {
    let logger = track_try_unwrap!(TerminalLoggerBuilder::new().level(Severity::Debug).build());
    let mut executor = track_try_unwrap!(track_any_err!(ThreadPoolExecutor::new()));

    let addr = "0.0.0.0:3100".parse().unwrap();
    let mut builder = ServerBuilder::new(addr);
    builder.logger(logger);
    track_try_unwrap!(builder.add_handler(Hello));
    track_try_unwrap!(builder.add_handler(MetricsHandler));
    let server = builder.finish(executor.handle());

    let fiber = executor.spawn_monitor(server);
    let result = track_try_unwrap!(track_any_err!(executor.run_fiber(fiber)));
    track_try_unwrap!(track_any_err!(result));
}

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
