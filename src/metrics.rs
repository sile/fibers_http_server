//! [Prometheus][prometheus] metrics.
//!
//! [prometheus]: https://prometheus.io/
use prometrics::metrics::{Counter, MetricBuilder};

/// HTTP server metrics.
#[derive(Debug, Clone)]
pub struct Metrics {
    pub(crate) connected_tcp_clients: Counter,
    pub(crate) disconnected_tcp_clients: Counter,
}
impl Metrics {
    pub(crate) fn new(mut builder: MetricBuilder) -> Self {
        // TODO: help
        builder.namespace("fibers_http_server");
        Metrics {
            connected_tcp_clients: builder
                .counter("connected_tcp_clients_total")
                .finish()
                .expect("Never fails"),
            disconnected_tcp_clients: builder
                .counter("disconnected_tcp_clients_total")
                .finish()
                .expect("Never fails"),
        }
    }
}
