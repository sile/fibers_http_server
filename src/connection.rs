use bytecodec::combinator::MaybeEos;
use bytecodec::io::{BufferedIo, IoDecodeExt, IoEncodeExt};
use bytecodec::{DecodeExt, Encode};
use fibers::net::TcpStream;
use futures::{Async, Future, Poll};
use httpcodec::{NoBodyDecoder, RequestDecoder};
use slog::Logger;
use std::mem;
use url::Url;

use dispatcher::Dispatcher;
use handler::{BoxReply, HandleInput, RequestHandlerInstance};
use metrics::ServerMetrics;
use response::ResEncoder;
use server::ServerOptions;
use {Error, Req, Result, Status};

#[derive(Debug)]
pub struct Connection {
    logger: Logger,
    metrics: ServerMetrics,
    stream: BufferedIo<TcpStream>,
    req_head_decoder: MaybeEos<RequestDecoder<NoBodyDecoder>>,
    dispatcher: Dispatcher,
    base_url: Url,
    phase: Phase,
    do_close: bool,
}
impl Connection {
    pub fn new(
        logger: Logger,
        metrics: ServerMetrics,
        stream: TcpStream,
        dispatcher: Dispatcher,
        options: &ServerOptions,
    ) -> Result<Self> {
        let _ = unsafe { stream.with_inner(|s| s.set_nodelay(true)) };
        let base_url = format!(
            "http://{}/",
            track!(stream.local_addr().map_err(Error::from))?
        );
        let base_url = track!(Url::parse(&base_url).map_err(Error::from))?;

        metrics.connected_tcp_clients.increment();
        let req_head_decoder =
            RequestDecoder::with_options(NoBodyDecoder, options.decode_options.clone());
        Ok(Connection {
            logger,
            metrics,
            stream: BufferedIo::new(stream, options.read_buffer_size, options.write_buffer_size),
            req_head_decoder: req_head_decoder.maybe_eos(),
            dispatcher,
            base_url,
            phase: Phase::ReadRequestHead,
            do_close: false,
        })
    }

    fn is_closed(&self) -> bool {
        self.stream.is_eos() || (self.stream.write_buf_ref().is_empty() && self.phase.is_closed())
    }

    fn read_request_head(&mut self) -> Phase {
        let result = track!(
            self.req_head_decoder
                .decode_from_read_buf(self.stream.read_buf_mut())
        );
        match result {
            Err(e) => {
                warn!(
                    self.logger,
                    "Cannot decode the head part of a HTTP request: {}", e
                );
                self.metrics.read_request_head_errors.increment();
                self.do_close = true;
                Phase::WriteResponse(ResEncoder::error(Status::BadRequest))
            }
            Ok(None) => Phase::ReadRequestHead,
            Ok(Some(head)) => match track!(Req::new(head, &self.base_url)) {
                Err(e) => {
                    warn!(
                        self.logger,
                        "Cannot parse the path of a HTTP request: {}", e
                    );
                    self.metrics.parse_request_path_errors.increment();
                    self.do_close = true;
                    Phase::WriteResponse(ResEncoder::error(Status::BadRequest))
                }
                Ok(head) => Phase::DispatchRequest(head),
            },
        }
    }

    fn dispatch_request(&mut self, head: Req<()>) -> Phase {
        match self.dispatcher.dispatch(&head) {
            Err(status) => {
                self.metrics.dispatch_request_errors.increment();
                self.do_close = true;
                Phase::WriteResponse(ResEncoder::error(status))
            }
            Ok(mut handler) => match track!(handler.init(head)) {
                Err(e) => {
                    warn!(self.logger, "Cannot initialize a request handler: {}", e);
                    self.metrics.initialize_handler_errors.increment();
                    self.do_close = true;
                    Phase::WriteResponse(ResEncoder::error(Status::InternalServerError))
                }
                Ok(()) => Phase::HandleRequest(handler),
            },
        }
    }

    fn handle_request(&mut self, mut handler: RequestHandlerInstance) -> Phase {
        match track!(handler.handle_input(self.stream.read_buf_mut())) {
            Err(e) => {
                warn!(
                    self.logger,
                    "Cannot decode the body of a HTTP request: {}", e
                );
                self.metrics.decode_request_body_errors.increment();
                self.do_close = true;
                Phase::WriteResponse(ResEncoder::error(Status::BadRequest))
            }
            Ok(None) => Phase::HandleRequest(handler),
            Ok(Some(reply)) => {
                self.do_close = handler.is_closed();
                Phase::PollReply(reply)
            }
        }
    }

    fn poll_reply(&mut self, mut reply: BoxReply) -> Phase {
        if let Async::Ready(res_encoder) = reply.poll().expect("Never fails") {
            Phase::WriteResponse(res_encoder)
        } else {
            Phase::PollReply(reply)
        }
    }

    fn write_response(&mut self, mut encoder: ResEncoder) -> Result<Phase> {
        track!(encoder.encode_to_write_buf(self.stream.write_buf_mut())).map_err(|e| {
            self.metrics.write_response_errors.increment();
            e
        })?;
        if encoder.is_idle() {
            if self.do_close {
                Ok(Phase::Closed)
            } else {
                Ok(Phase::ReadRequestHead)
            }
        } else {
            Ok(Phase::WriteResponse(encoder))
        }
    }

    fn poll_once(&mut self) -> Result<bool> {
        track!(self.stream.execute_io())?;
        let old = mem::discriminant(&self.phase);
        let next = match self.phase.take() {
            Phase::ReadRequestHead => self.read_request_head(),
            Phase::DispatchRequest(req) => self.dispatch_request(req),
            Phase::HandleRequest(handler) => self.handle_request(handler),
            Phase::PollReply(reply) => self.poll_reply(reply),
            Phase::WriteResponse(res) => track!(self.write_response(res))?,
            Phase::Closed => Phase::Closed,
        };
        self.phase = next;
        let changed = mem::discriminant(&self.phase) != old;
        Ok(changed || !self.stream.would_block())
    }
}
impl Future for Connection {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while !self.is_closed() {
            match track!(self.poll_once()) {
                Err(e) => {
                    warn!(self.logger, "Connection aborted: {}", e);
                    self.metrics.disconnected_tcp_clients.increment();
                    return Err(());
                }
                Ok(do_continue) => {
                    if !do_continue {
                        if self.is_closed() {
                            break;
                        }
                        return Ok(Async::NotReady);
                    }
                }
            }
        }

        debug!(self.logger, "Connection closed");
        self.metrics.disconnected_tcp_clients.increment();
        Ok(Async::Ready(()))
    }
}

#[derive(Debug)]
enum Phase {
    ReadRequestHead,
    DispatchRequest(Req<()>),
    HandleRequest(RequestHandlerInstance),
    PollReply(BoxReply),
    WriteResponse(ResEncoder),
    Closed,
}
impl Phase {
    fn take(&mut self) -> Self {
        mem::replace(self, Phase::Closed)
    }

    fn is_closed(&self) -> bool {
        if let Phase::Closed = *self {
            true
        } else {
            false
        }
    }
}
