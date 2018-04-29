//! [Prometheus][prometheus] metrics.
//!
//! [prometheus]: https://prometheus.io/
use prometrics::metrics::{Counter, MetricBuilder};

/// HTTP server metrics.
#[derive(Debug, Clone)]
pub struct Metrics {
    pub(crate) connected_tcp_clients: Counter,
    pub(crate) disconnected_tcp_clients: Counter,
    pub(crate) read_request_head_errors: Counter,
    pub(crate) parse_request_path_errors: Counter,
    pub(crate) dispatch_request_errors: Counter,
    pub(crate) initialize_handler_errors: Counter,
    pub(crate) write_response_errors: Counter,
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
            read_request_head_errors: builder
                .counter("errors_total")
                .label("phase", "read_request_head")
                .finish()
                .expect("Never fails"),
            parse_request_path_errors: builder
                .counter("errors_total")
                .label("phase", "parse_request_path")
                .finish()
                .expect("Never fails"),
            dispatch_request_errors: builder
                .counter("errors_total")
                .label("phase", "dispatch_request")
                .finish()
                .expect("Never fails"),
            initialize_handler_errors: builder
                .counter("errors_total")
                .label("phase", "initialize_handler")
                .finish()
                .expect("Never fails"),
            write_response_errors: builder
                .counter("errors_total")
                .label("phase", "write_response")
                .finish()
                .expect("Never fails"),
        }
    }
}
