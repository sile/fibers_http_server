use factory::Factory;
use fibers::net::futures::{Connected, TcpListenerBind};
use fibers::net::streams::Incoming;
use fibers::net::TcpListener;
use fibers::{BoxSpawn, Spawn};
use futures::{Async, Future, Poll, Stream};
use httpcodec::DecodeOptions;
use prometrics::metrics::MetricBuilder;
use slog::{Discard, Logger};
use std::net::SocketAddr;

use connection::Connection;
use dispatcher::{Dispatcher, DispatcherBuilder};
use metrics::ServerMetrics;
use {Error, HandleRequest, HandlerOptions, Result};

/// HTTP server builder.
#[derive(Debug)]
pub struct ServerBuilder {
    bind_addr: SocketAddr,
    logger: Logger,
    metrics: MetricBuilder,
    dispatcher: DispatcherBuilder,
    options: ServerOptions,
}
impl ServerBuilder {
    /// Makes a new `ServerBuilder` instance.
    pub fn new(bind_addr: SocketAddr) -> Self {
        ServerBuilder {
            bind_addr,
            logger: Logger::root(Discard, o!()),
            metrics: MetricBuilder::default(),
            dispatcher: DispatcherBuilder::new(),
            options: ServerOptions {
                read_buffer_size: 8192,
                write_buffer_size: 8192,
                decode_options: DecodeOptions::default(),
            },
        }
    }

    /// Adds a HTTP request handler.
    ///
    /// # Errors
    ///
    /// If the path and method of the handler conflicts with the already registered handlers,
    /// an `ErrorKind::InvalidInput` error will be returned.
    pub fn add_handler<H>(&mut self, handler: H) -> Result<&mut Self>
    where
        H: HandleRequest,
        H::Decoder: Default,
        H::Encoder: Default,
    {
        self.add_handler_with_options(handler, HandlerOptions::default())
    }

    /// Adds a HTTP request handler with the given options.
    ///
    /// # Errors
    ///
    /// If the path and method of the handler conflicts with the already registered handlers,
    /// an `ErrorKind::InvalidInput` error will be returned.
    pub fn add_handler_with_options<H, D, E>(
        &mut self,
        handler: H,
        options: HandlerOptions<H, D, E>,
    ) -> Result<&mut Self>
    where
        H: HandleRequest,
        D: Factory<Item = H::Decoder> + Send + Sync + 'static,
        E: Factory<Item = H::Encoder> + Send + Sync + 'static,
    {
        track!(self.dispatcher.register_handler(handler, options))?;
        Ok(self)
    }

    /// Sets the logger of the server.
    ///
    /// The default value is `Logger::root(Discard, o!())`.
    pub fn logger(&mut self, logger: Logger) -> &mut Self {
        self.logger = logger;
        self
    }

    /// Sets `MetricBuilder` used by the server.
    ///
    /// The default value is `MetricBuilder::default()`.
    pub fn metrics(&mut self, metrics: MetricBuilder) -> &mut Self {
        self.metrics = metrics;
        self
    }

    /// Sets the application level read buffer size of the server in bytes.
    ///
    /// The default value is `8192`.
    pub fn read_buffer_size(&mut self, n: usize) -> &mut Self {
        self.options.read_buffer_size = n;
        self
    }

    /// Sets the application level write buffer size of the server in bytes.
    ///
    /// The default value is `8192`.
    pub fn write_buffer_size(&mut self, n: usize) -> &mut Self {
        self.options.write_buffer_size = n;
        self
    }

    /// Sets the options of the request decoder of the server.
    ///
    /// The default value is `DecodeOptions::default()`.
    pub fn decode_options(&mut self, options: DecodeOptions) -> &mut Self {
        self.options.decode_options = options;
        self
    }

    /// Builds a HTTP server with the given settings.
    pub fn finish<S>(self, spawner: S) -> Server
    where
        S: Spawn + Send + 'static,
    {
        let logger = self.logger.new(o!("server" => self.bind_addr.to_string()));

        info!(logger, "Starts HTTP server");
        Server {
            logger,
            metrics: ServerMetrics::new(self.metrics),
            spawner: spawner.boxed(),
            listener: Listener::Binding(TcpListener::bind(self.bind_addr)),
            dispatcher: self.dispatcher.finish(),
            options: self.options,
            connected: Vec::new(),
        }
    }
}

/// HTTP server.
///
/// This is created via `ServerBuilder`.
#[derive(Debug)]
pub struct Server {
    logger: Logger,
    metrics: ServerMetrics,
    spawner: BoxSpawn,
    listener: Listener,
    dispatcher: Dispatcher,
    options: ServerOptions,
    connected: Vec<(SocketAddr, Connected)>,
}
impl Server {
    /// Returns the metrics of the server.
    pub fn metrics(&self) -> &ServerMetrics {
        &self.metrics
    }
}
impl Future for Server {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match track!(self.listener.poll())? {
                Async::NotReady => {
                    break;
                }
                Async::Ready(None) => {
                    warn!(self.logger, "The socket of the HTTP server has been closed");
                    return Ok(Async::Ready(()));
                }
                Async::Ready(Some((connected, addr))) => {
                    self.connected.push((addr, connected));
                }
            }
        }

        let mut i = 0;
        while i < self.connected.len() {
            if let Async::Ready(stream) = track!(self.connected[i].1.poll().map_err(Error::from))? {
                let client_addr = self.connected.swap_remove(i).0;
                let logger = self.logger.new(o!("client" => client_addr.to_string()));
                debug!(logger, "New client arrived");
                let future = track!(Connection::new(
                    logger,
                    self.metrics.clone(),
                    stream,
                    self.dispatcher.clone(),
                    &self.options,
                ))?;
                self.spawner.spawn(future);
            } else {
                i += 1;
            }
        }
        Ok(Async::NotReady)
    }
}

#[derive(Debug)]
enum Listener {
    Binding(TcpListenerBind),
    Listening(Incoming),
}
impl Stream for Listener {
    type Item = (Connected, SocketAddr);
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            let next = match *self {
                Listener::Binding(ref mut f) => {
                    if let Async::Ready(listener) = track!(f.poll().map_err(Error::from))? {
                        Listener::Listening(listener.incoming())
                    } else {
                        return Ok(Async::NotReady);
                    }
                }
                Listener::Listening(ref mut s) => {
                    return track!(s.poll().map_err(Error::from));
                }
            };
            *self = next;
        }
    }
}

#[derive(Debug)]
pub struct ServerOptions {
    pub read_buffer_size: usize,
    pub write_buffer_size: usize,
    pub decode_options: DecodeOptions,
}
