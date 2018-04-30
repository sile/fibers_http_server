//! [Prometheus][prometheus] metrics.
//!
//! [prometheus]: https://prometheus.io/
use bytecodec::bytes::Utf8Encoder;
use bytecodec::marker::Never;
use bytecodec::value::NullDecoder;
use futures;
use futures::future::Finished;
use httpcodec::{BodyDecoder, BodyEncoder};
use prometrics;
use prometrics::metrics::{Counter, MetricBuilder};

use {HandleRequest, Req, Res, Status};

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
    /// Number of connected TCP clients.
    ///
    /// Metric: `fibers_http_server_connected_tcp_clients_total <COUNTER>`
    pub fn connected_tcp_clients(&self) -> u64 {
        self.connected_tcp_clients.value() as u64
    }

    /// Number of disconnected TCP clients.
    ///
    /// Metric: `fibers_http_server_disconnected_tcp_clients_total <COUNTER>`
    pub fn disconnected_tcp_clients(&self) -> u64 {
        self.disconnected_tcp_clients.value() as u64
    }

    /// Number of errors occurred while reading the head part of requests.
    ///
    /// Metric: `fibers_http_server_errors_total { phase="read_request_head" } <COUNTER>`
    pub fn read_request_head_errors(&self) -> u64 {
        self.read_request_head_errors.value() as u64
    }

    /// Number of errors occurred while parsing the path of requests.
    ///
    /// Metric: `fibers_http_server_errors_total { phase="parse_request_path" } <COUNTER>`
    pub fn parse_request_path_errors(&self) -> u64 {
        self.parse_request_path_errors.value() as u64
    }

    /// Number of errors occurred while dispatcing requests.
    ///
    /// Metric: `fibers_http_server_errors_total { phase="dispatch_request" } <COUNTER>`
    pub fn dispatch_request_errors(&self) -> u64 {
        self.dispatch_request_errors.value() as u64
    }

    /// Number of errors occurred while initializing request handlers.
    ///
    /// Metric: `fibers_http_server_errors_total { phase="initialize_handler" } <COUNTER>`
    pub fn initialize_handler_errors(&self) -> u64 {
        self.initialize_handler_errors.value() as u64
    }

    /// Number of errors occurred while decoding the body part of requests.
    ///
    /// Metric: `fibers_http_server_errors_total { phase="decode_request_body" } <COUNTER>`
    pub fn decode_request_body_errors(&self) -> u64 {
        self.decode_request_body_errors.value() as u64
    }

    /// Number of errors occurred while writing responses to sockets.
    ///
    /// Metric: `fibers_http_server_errors_total { phase="write_response" } <COUNTER>`
    pub fn write_response_errors(&self) -> u64 {
        self.write_response_errors.value() as u64
    }

    pub(crate) fn new(mut builder: MetricBuilder) -> Self {
        builder.namespace("fibers_http_server");
        ServerMetrics {
            connected_tcp_clients: builder
                .counter("connected_tcp_clients_total")
                .help("Number of connected TCP clients")
                .finish()
                .expect("Never fails"),
            disconnected_tcp_clients: builder
                .counter("disconnected_tcp_clients_total")
                .help("Number of disconnected TCP clients")
                .finish()
                .expect("Never fails"),
            read_request_head_errors: builder
                .counter("errors_total")
                .help("Number of errors")
                .label("phase", "read_request_head")
                .finish()
                .expect("Never fails"),
            parse_request_path_errors: builder
                .counter("errors_total")
                .help("Number of errors")
                .label("phase", "parse_request_path")
                .finish()
                .expect("Never fails"),
            dispatch_request_errors: builder
                .counter("errors_total")
                .help("Number of errors")
                .label("phase", "dispatch_request")
                .finish()
                .expect("Never fails"),
            initialize_handler_errors: builder
                .counter("errors_total")
                .help("Number of errors")
                .label("phase", "initialize_handler")
                .finish()
                .expect("Never fails"),
            decode_request_body_errors: builder
                .counter("errors_total")
                .help("Number of errors")
                .label("phase", "decode_request_body")
                .finish()
                .expect("Never fails"),
            write_response_errors: builder
                .counter("errors_total")
                .help("Number of errors")
                .label("phase", "write_response")
                .finish()
                .expect("Never fails"),
        }
    }
}

/// A handler for exposing [prometheus] metrics.
///
/// [prometheus]: https://prometheus.io/
#[derive(Debug)]
pub struct MetricsHandler;
impl HandleRequest for MetricsHandler {
    const METHOD: &'static str = "GET";
    const PATH: &'static str = "/metrics";

    type ReqBody = ();
    type ResBody = String;
    type Decoder = BodyDecoder<NullDecoder>;
    type Encoder = BodyEncoder<Utf8Encoder>;
    type Reply = Finished<Res<Self::ResBody>, Never>;

    fn handle_request(&self, _req: Req<Self::ReqBody>) -> Self::Reply {
        let res = match prometrics::default_gatherer().lock() {
            Err(e) => Res::new(Status::InternalServerError, e.to_string()),
            Ok(mut gatherer) => {
                let metrics = gatherer.gather().to_text();
                Res::new(Status::Ok, metrics)
            }
        };
        futures::finished(res)
    }
}

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
