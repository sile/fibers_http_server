use std::net::SocketAddr;
use slog::{Discard, Logger};
use bytecodec::io::{ReadBuf, WriteBuf};
use factory::Factory;
use fibers::{BoxSpawn, Spawn};
use fibers::net::{TcpListener, TcpStream};
use fibers::net::futures::{Connected, TcpListenerBind};
use fibers::net::streams::Incoming;
use futures::{Async, Future, Poll, Stream};
use httpcodec::{DecodeOptions, NoBodyDecoder, RequestDecoder};

use {Error, HandleRequest, HandlerOptions, Result};
use dispatcher::{Dispatcher, DispatcherBuilder};
use handler::BoxStreamHandler;

#[derive(Debug)]
pub struct ServerBuilder {
    bind_addr: SocketAddr,
    logger: Logger,
    decode_options: DecodeOptions, // TODO
    dispatcher: DispatcherBuilder,
}
impl ServerBuilder {
    pub fn new(bind_addr: SocketAddr) -> Self {
        ServerBuilder {
            bind_addr,
            logger: Logger::root(Discard, o!()),
            decode_options: DecodeOptions::default(),
            dispatcher: DispatcherBuilder::new(),
        }
    }

    pub fn logger(&mut self, logger: Logger) -> &mut Self {
        self.logger = logger;
        self
    }

    pub fn add_handler<H>(&mut self, handler: H) -> Result<&mut Self>
    where
        H: HandleRequest,
        H::Decoder: Default,
        H::Encoder: Default,
    {
        self.add_handler_with_options(handler, HandlerOptions::default())
    }

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

    pub fn finish<S>(self, spawner: S) -> Server
    where
        S: Spawn + Send + 'static,
    {
        info!(self.logger, "Starts HTTP server");
        let listener = Listener::Binding(TcpListener::bind(self.bind_addr));
        Server {
            logger: self.logger,
            spawner: spawner.boxed(),
            listener,
            connected: Vec::new(),
            dispatcher: self.dispatcher.finish(),
        }
    }
}

#[derive(Debug)]
pub struct Server {
    logger: Logger,
    spawner: BoxSpawn,
    listener: Listener,
    dispatcher: Dispatcher,
    connected: Vec<(SocketAddr, Connected)>,
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
                    debug!(self.logger, "New client arrived: {}", addr);
                    self.connected.push((addr, connected));
                }
            }
        }

        let mut i = 0;
        while i < self.connected.len() {
            match track!(self.connected[i].1.poll().map_err(Error::from)) {
                Err(e) => {
                    warn!(
                        self.logger,
                        "Cannot initialize client socket {}: {}", self.connected[i].0, e
                    );
                    self.connected.swap_remove(i);
                }
                Ok(Async::NotReady) => {
                    i += 1;
                }
                Ok(Async::Ready(stream)) => {
                    self.connected.swap_remove(i);
                    let future = Connection::new(self, stream);
                    self.spawner.spawn(future);
                }
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
struct Connection {
    stream: TcpStream,
    rbuf: ReadBuf<Vec<u8>>,
    wbuf: WriteBuf<Vec<u8>>,
    head_decoder: RequestDecoder<NoBodyDecoder>,
    dispatcher: Dispatcher,
    stream_handler: Option<BoxStreamHandler>,
}
impl Connection {
    fn new(server: &Server, stream: TcpStream) -> Self {
        Connection {
            stream,
            rbuf: ReadBuf::new(vec![0; 4096]), // TODO: parameter
            wbuf: WriteBuf::new(vec![0; 4096]),
            head_decoder: Default::default(),
            dispatcher: server.dispatcher.clone(),
            stream_handler: None,
        }
    }
}
impl Future for Connection {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        unimplemented!()
    }
}
