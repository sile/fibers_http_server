use std::mem;
use slog::Logger;
use bytecodec::Encode;
use bytecodec::io::{IoDecodeExt, IoEncodeExt};
use fibers::net::TcpStream;
use futures::{Async, Future, Poll};
use httpcodec::{NoBodyDecoder, RequestDecoder};
use url::Url;

use {Error, Req, Result, Status};
use bc::BufferedIo;
use dispatcher::Dispatcher;
use handler::{BoxReply, BoxStreamHandler};
use metrics::Metrics;
use response::ResEncoder;
use server::ServerOptions;

#[derive(Debug)]
pub struct Connection {
    logger: Logger,
    metrics: Metrics,
    stream: BufferedIo<TcpStream>,
    req_head_decoder: RequestDecoder<NoBodyDecoder>,
    dispatcher: Dispatcher,
    base_url: Url,
    phase: Phase,
}
impl Connection {
    pub fn new(
        logger: Logger,
        metrics: Metrics,
        stream: TcpStream,
        dispatcher: Dispatcher,
        options: &ServerOptions,
    ) -> Result<Self> {
        let base_url = format!(
            "http://{}/",
            track!(stream.peer_addr().map_err(Error::from))?
        );
        let base_url = track!(Url::parse(&base_url).map_err(Error::from))?;

        metrics.connected_tcp_clients.increment();
        let req_head_decoder =
            RequestDecoder::with_options(NoBodyDecoder, options.decode_options.clone());
        Ok(Connection {
            logger,
            metrics,
            stream: BufferedIo::new(stream, options.read_buffer_size, options.write_buffer_size),
            req_head_decoder,
            dispatcher,
            base_url,
            phase: Phase::ReadRequestHead,
        })
    }

    fn is_closed(&self) -> bool {
        self.stream.is_eos() || (self.stream.write_buf().is_empty() && self.phase.is_closed())
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
                let res = ResEncoder::error(Status::BadRequest);
                Phase::WriteResponse(res, true)
            }
            Ok(None) => Phase::ReadRequestHead,
            Ok(Some(head)) => match track!(Req::new(head, &self.base_url)) {
                Err(e) => {
                    warn!(
                        self.logger,
                        "Cannot parse the path of a HTTP request: {}", e
                    );
                    self.metrics.parse_request_path_errors.increment();
                    let res = ResEncoder::error(Status::BadRequest);
                    Phase::WriteResponse(res, true)
                }
                Ok(head) => Phase::DispatchRequest(head),
            },
        }
    }

    fn dispatch_request(&mut self, head: Req<()>) -> Phase {
        match self.dispatcher.dispatch(&head) {
            Err(status) => {
                self.metrics.dispatch_request_errors.increment();
                let res = ResEncoder::error(status);
                Phase::WriteResponse(res, true)
            }
            Ok(mut handler) => match track!(handler.init(head)) {
                Err(e) => {
                    warn!(self.logger, "Cannot initialize a request handler: {}", e);
                    self.metrics.initialize_handler_errors.increment();
                    let res = ResEncoder::error(Status::InternalServerError);
                    Phase::WriteResponse(res, true)
                }
                Ok(()) => Phase::HandleRequest(handler),
            },
        }
    }

    fn handle_request(&mut self, handler: BoxStreamHandler) -> Phase {
        // if let Some(mut handler) = self.stream_handler.take() {
        //     if let Some(reply) = handler
        //         .handle_request(self.stream.read_buf_mut())
        //         .expect("TODO: 9")
        //     {
        //         self.is_closed = handler.is_closed();
        //         self.reply = Some(reply);
        //         continue;
        //     } else {
        //         self.stream_handler = Some(handler);
        //     }
        // } else if self.res_encoder.is_none() {
        // }
        unimplemented!()
    }

    fn poll_reply(&mut self, reply: BoxReply) -> Phase {
        // if let Some(mut reply) = self.reply.take() {
        //     if let Async::Ready(res_encoder) = reply.poll().expect("TODO: 8") {
        //         self.res_encoder = Some(res_encoder);
        //     } else {
        //         self.reply = Some(reply);
        //         return Ok(Async::NotReady);
        //     }
        // }
        unimplemented!()
    }

    fn write_response(&mut self, mut encoder: ResEncoder, last: bool) -> Result<(bool, Phase)> {
        track!(encoder.encode_to_write_buf(self.stream.write_buf_mut())).map_err(|e| {
            self.metrics.write_response_errors.increment();
            e
        })?;
        if encoder.is_idle() {
            if last {
                Ok((true, Phase::Closed))
            } else {
                Ok((true, Phase::ReadRequestHead))
            }
        } else {
            let suspended = !self.stream.write_buf().is_full();
            Ok((!suspended, Phase::WriteResponse(encoder, last)))
        }
    }

    fn poll_once(&mut self) -> Result<bool> {
        track!(self.stream.execute_io())?;
        let (do_continue, next) = match self.phase.take() {
            Phase::ReadRequestHead => (true, self.read_request_head()),
            Phase::DispatchRequest(req) => (true, self.dispatch_request(req)),
            Phase::HandleRequest(handler) => (true, self.handle_request(handler)),
            Phase::PollReply(reply) => (true, self.poll_reply(reply)),
            Phase::WriteResponse(res, last) => track!(self.write_response(res, last))?,
            Phase::Closed => (true, Phase::Closed),
        };
        self.phase = next;
        Ok(do_continue && !self.stream.would_block())
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
    HandleRequest(BoxStreamHandler),
    PollReply(BoxReply),
    WriteResponse(ResEncoder, bool),
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
