//! [Prometheus][prometheus] metrics.
//!
//! [prometheus]: https://prometheus.io/
use crate::{Error, HandleRequest, Req, Res, Status};
use atomic_immut::AtomicImmut;
use bytecodec::bytes::Utf8Encoder;
use bytecodec::marker::Never;
use bytecodec::null::NullDecoder;
use fibers::sync::oneshot;
use futures::{Async, Future, Poll};
use httpcodec::{BodyDecoder, BodyEncoder};
use prometrics;
use prometrics::bucket::Bucket;
use prometrics::metrics::{Counter, Histogram, HistogramBuilder, MetricBuilder};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

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
    type Reply = Box<dyn Future<Item = Res<Self::ResBody>, Error = Never> + Send + 'static>;

    fn handle_request(&self, _req: Req<Self::ReqBody>) -> Self::Reply {
        let (tx, rx) = oneshot::channel();
        thread::spawn(move || {
            let res = match prometrics::default_gatherer().lock() {
                Err(e) => Res::new(Status::InternalServerError, e.to_string()),
                Ok(mut gatherer) => {
                    let metrics = gatherer.gather().to_text();
                    Res::new(Status::Ok, metrics)
                }
            };
            let _ = tx.send(res);
        });
        Box::new(rx.or_else(|e| Ok(Res::new(Status::InternalServerError, e.to_string()))))
    }
}

/// A handler for granting the metrics collection functionality to the inner handler `H`.
#[derive(Debug)]
pub struct WithMetrics<H> {
    inner: H,
    metrics: HandlerMetrics,
}
impl<H: HandleRequest> WithMetrics<H> {
    /// Makes a new `WithMetrics` instance.
    pub fn new(inner: H) -> Self {
        Self::with_metrics(inner, MetricBuilder::new())
    }

    /// Makes a new `WithMetrics` instance with the given `MetricBuilder`.
    pub fn with_metrics(inner: H, metric_builder: MetricBuilder) -> Self {
        Self::with_metrics_and_bucket_config(inner, metric_builder, BucketConfig::default())
    }

    /// Makes a new `WithMetrics` instance with the given `MetricBuilder` and `BucketConfig`.
    pub fn with_metrics_and_bucket_config(
        inner: H,
        metric_builder: MetricBuilder,
        bucket_config: BucketConfig,
    ) -> Self {
        let metrics = HandlerMetrics::new::<H>(metric_builder, bucket_config);
        WithMetrics { inner, metrics }
    }

    /// Returns the metrics of the handler.
    pub fn metrics(&self) -> &HandlerMetrics {
        &self.metrics
    }
}
impl<H: HandleRequest> HandleRequest for WithMetrics<H> {
    const METHOD: &'static str = H::METHOD;
    const PATH: &'static str = H::PATH;

    type ReqBody = H::ReqBody;
    type ResBody = H::ResBody;
    type Decoder = H::Decoder;
    type Encoder = H::Encoder;
    type Reply = Time<H>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        Time::new(self.inner.handle_request(req), self.metrics.clone())
    }

    fn handle_request_head(&self, req: &Req<()>) -> Option<Res<Self::ResBody>> {
        let result = self.inner.handle_request_head(req);
        if let Some(ref res) = result {
            self.metrics.increment_status(res.status_code());
        }
        result
    }

    fn handle_decoding_error(&self, req: Req<()>, error: &Error) -> Option<Res<Self::ResBody>> {
        let result = self.inner.handle_decoding_error(req, error);
        if let Some(ref res) = result {
            self.metrics.increment_status(res.status_code());
        }
        result
    }
}

/// `Future` that for measuring the time elapsed to handle a request.
#[derive(Debug)]
pub struct Time<H: HandleRequest> {
    future: H::Reply,
    start: Instant,
    metrics: HandlerMetrics,
    _handler: PhantomData<H>,
}
impl<H: HandleRequest> Time<H> {
    fn new(future: H::Reply, metrics: HandlerMetrics) -> Self {
        Time {
            future,
            start: Instant::now(),
            metrics,
            _handler: PhantomData,
        }
    }
}
impl<H: HandleRequest> Future for Time<H> {
    type Item = Res<H::ResBody>;
    type Error = Never;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Ok(Async::Ready(res)) = self.future.poll() {
            let elapsed = prometrics::timestamp::duration_to_seconds(self.start.elapsed());
            self.metrics.request_duration_seconds.observe(elapsed);
            self.metrics.increment_status(res.status_code());
            Ok(Async::Ready(res))
        } else {
            Ok(Async::NotReady)
        }
    }
}

/// HTTP handler metrics.
#[derive(Debug, Clone)]
pub struct HandlerMetrics {
    requests: Arc<AtomicImmut<HashMap<u16, Counter>>>,
    request_duration_seconds: Histogram,
    builder: Arc<Mutex<MetricBuilder>>,
}
impl HandlerMetrics {
    /// Number of requests that the handler handled.
    ///
    /// Metric: `fibers_http_server_handler_requests_total { status = "..." } <COUNTER>`
    pub fn requests(&self, status_code: u16) -> Option<u64> {
        self.requests
            .load()
            .get(&status_code)
            .map(|c| c.value() as u64)
    }

    /// Histogram bucket of requests processing duration.
    ///
    /// It does not contains the time elapsed for reading/writing requests/responses.
    ///
    /// Metric: `fibers_http_server_handler_request_duration_seconds_bucket
    /// { le="...", method="...", path="..." } <COUNTER>`
    pub fn request_duration_seconds_buckets(&self) -> &[Bucket] {
        self.request_duration_seconds.buckets()
    }

    fn new<H: HandleRequest>(mut builder: MetricBuilder, bucket_config: BucketConfig) -> Self {
        builder
            .namespace("fibers_http_server")
            .subsystem("handler")
            .label("method", H::METHOD)
            .label("path", H::PATH);
        HandlerMetrics {
            requests: Default::default(),
            request_duration_seconds: bucket_config
                .prepare_histogram(
                    builder
                        .histogram("request_duration_seconds")
                        .help("Requests processing duration"),
                )
                .finish()
                .expect("Never fails"),
            builder: Arc::new(Mutex::new(builder)),
        }
    }

    fn increment_status(&self, status: u16) {
        if self
            .requests
            .load()
            .get(&status)
            .map(|c| c.increment())
            .is_none()
        {
            if let Ok(builder) = self.builder.try_lock() {
                let counter = builder
                    .counter("requests_total")
                    .help("Number of requests")
                    .label("status", &status.to_string())
                    .finish()
                    .expect("Never fails");
                self.requests.update(|old| {
                    let mut new = old.clone();
                    new.insert(status, counter.clone());
                    new
                });
            }
            if let Some(c) = self.requests.load().get(&status) {
                c.increment()
            }
        }
    }
}

/// Bucket configuration. Holds an increasing sequence of upper_bound.
pub struct BucketConfig(Vec<f64>);

impl Default for BucketConfig {
    fn default() -> Self {
        let upper_bounds = vec![0.0001, 0.0005, 0.001, 0.005, 0.1, 0.5, 1.0, 5.0, 10.0, 50.0];
        Self::new(upper_bounds)
    }
}

impl BucketConfig {
    /// Creates a new BucketConfig using the given upper_bounds.
    /// If upper_bounds is not strictly increasing, this function will panic.
    pub fn new(upper_bounds: Vec<f64>) -> Self {
        assert!(!upper_bounds.is_empty(), "upper_bounds cannot be empty");
        for i in 0..upper_bounds.len() - 1 {
            assert!(
                upper_bounds[i] < upper_bounds[i + 1],
                "upper_bounds is not strictly increasing: {:?}",
                upper_bounds
            );
        }
        Self(upper_bounds)
    }
    // Build a histogram using this BucketConfig.
    fn prepare_histogram<'a>(
        &self,
        histogram_builder: &'a mut HistogramBuilder,
    ) -> &'a mut HistogramBuilder {
        for &upper_bound in &self.0 {
            histogram_builder.bucket(upper_bound);
        }
        histogram_builder
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_config_new_succeeds() {
        let upper_bounds = vec![
            0.001, 0.005, 0.01, 0.05, 0.1, 0.4, 0.8, 1.0, 2.0, 4.0, 6.0, 8.0, 10.0, 50.0,
        ];
        let _ = BucketConfig::new(upper_bounds); // never panics
    }

    #[test]
    #[should_panic]
    fn bucket_config_new_correctly_panics() {
        let upper_bounds = vec![
            0.1, 0.5, 0.4, // not increasing!
        ];
        let _ = BucketConfig::new(upper_bounds);
    }
}
