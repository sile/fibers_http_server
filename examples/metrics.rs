extern crate fibers_global;
extern crate fibers_http_server;
extern crate futures;
extern crate slog;
#[macro_use]
extern crate trackable;

use fibers_http_server::metrics::MetricsHandler;
use fibers_http_server::ServerBuilder;
use trackable::result::MainResult;

fn main() -> MainResult {
    let addr = "0.0.0.0:9090".parse().unwrap();
    let mut builder = ServerBuilder::new(addr);
    track!(builder.add_handler(MetricsHandler))?;

    let server = builder.finish(fibers_global::handle());
    track!(fibers_global::execute(server))?;
    Ok(())
}
