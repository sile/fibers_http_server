//! [Prometheus][prometheus] metrics.
//!
//! [prometheus]: https://prometheus.io/
use prometrics::metrics::{Counter, MetricBuilder};

/// HTTP server metrics.
#[derive(Debug, Clone)]
pub struct ServerMetrics {
    pub(crate) connected_tcp_clients: Counter,
    pub(crate) disconnected_tcp_clients: Counter,
    pub(crate) read_request_head_errors: Counter,
    pub(crate) parse_request_path_errors: Counter,
    pub(crate) dispatch_request_errors: Counter,
    pub(crate) initialize_handler_errors: Counter,
    pub(crate) decode_request_body_errors: Counter,
    pub(crate) write_response_errors: Counter,
}
impl ServerMetrics {
    pub(crate) fn new(mut builder: MetricBuilder) -> Self {
        // TODO: help
        builder.namespace("fibers_http_server");
        ServerMetrics {
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
            decode_request_body_errors: builder
                .counter("errors_total")
                .label("phase", "decode_request_body")
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

// #[derive(Debug, Clone)]
// pub struct HandlersMetrics(pub(crate) HashMap<(&'static str, &'static str), Arc<HandlerMetrics>>);
// impl HandlersMetrics {
//     pub(crate) fn new() -> Self {
//         HandlersMetrics(HashMap::new())
//     }
// }

// #[derive(Debug, Clone)]
// pub struct HandlerMetrics {
//     requests: HashMap<u16, Counter>,
//     request_duration_seconds: Histogram,
//     builder: Arc<Mutex<MetricBuilder>>,
// }
// impl HandlerMetrics {
//     pub(crate) fn new(mut builder: MetricBuilder) -> Self {
//         builder.namespace("fibers_http_server").subsystem("handler");
//         HandlerMetrics {
//             requests: HashMap::new(),
//             request_duration_seconds: builder
//                 .histogram("request_duration_seconds")
//                 .bucket(0.001)
//                 .bucket(0.005)
//                 .bucket(0.01)
//                 .bucket(0.05)
//                 .bucket(0.1)
//                 .bucket(0.5)
//                 .bucket(1.0)
//                 .bucket(5.0)
//                 .bucket(10.0)
//                 .bucket(50.0)
//                 .finish()
//                 .expect("Never fails"),
//             builder: Arc::new(Mutex::new(builder)),
//         }
//     }
// }
