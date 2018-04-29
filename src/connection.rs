use slog::Logger;
use bytecodec::Encode;
use bytecodec::io::{IoDecodeExt, IoEncodeExt};
use bytecodec::marker::Never;
use fibers::net::TcpStream;
use futures::{Async, Future, Poll};
use httpcodec::{NoBodyDecoder, RequestDecoder};
use url::Url;

use {Error, Req, Result};
use bc::BufferedIo;
use dispatcher::Dispatcher;
use handler::{BoxReply, BoxStreamHandler};
use metrics::Metrics;
use server::ServerOptions;

pub struct Connection {
    logger: Logger,
    metrics: Metrics,
    stream: BufferedIo<TcpStream>,
    req_head_decoder: RequestDecoder<NoBodyDecoder>,
    dispatcher: Dispatcher,
    stream_handler: Option<BoxStreamHandler>,
    base_url: Url,
    is_closed: bool,
    reply: Option<BoxReply>,
    res_encoder: Option<Box<Encode<Item = Never> + Send + 'static>>,
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
            stream_handler: None,
            base_url,
            is_closed: false,
            reply: None,
            res_encoder: None,
        })
    }
}
impl Future for Connection {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        // TODO: refactoring
        loop {
            self.stream.execute_io().expect("TODO: 6");

            if let Some(mut reply) = self.reply.take() {
                if let Async::Ready(res_encoder) = reply.poll().expect("TODO: 8") {
                    self.res_encoder = Some(res_encoder);
                } else {
                    self.reply = Some(reply);
                    return Ok(Async::NotReady);
                }
            }

            if let Some(mut res_encoder) = self.res_encoder.take() {
                res_encoder
                    .encode_to_write_buf(self.stream.write_buf_mut())
                    .expect("TODO: 7");
                if !res_encoder.is_idle() {
                    self.res_encoder = Some(res_encoder);
                    continue;
                }
            }

            if let Some(mut handler) = self.stream_handler.take() {
                if let Some(reply) = handler
                    .handle_request(self.stream.read_buf_mut())
                    .expect("TODO: 9")
                {
                    self.is_closed = handler.is_closed();
                    self.reply = Some(reply);
                    continue;
                } else {
                    self.stream_handler = Some(handler);
                }
            } else if self.res_encoder.is_none() {
                // TODO: return bad reuqest if error
                let item = self.req_head_decoder
                    .decode_from_read_buf(self.stream.read_buf_mut())
                    .map_err(|_| ())?;
                if let Some(head) = item {
                    let head = Req::new(head, &self.base_url).expect("TODO: 10");
                    if let Some(mut handler) = self.dispatcher.dispatch(&head) {
                        if handler.init(head).is_err() {
                            // TODO:
                            return Ok(Async::Ready(()));
                        }
                        self.stream_handler = Some(handler);
                        continue;
                    } else {
                        // TODO:
                        println!("[TODO] # Not Found");
                        return Ok(Async::Ready(()));
                    }
                }
            }

            if self.stream.is_eos() {
                return Ok(Async::Ready(()));
            }
            if self.stream.would_block() {
                break;
            }
        }
        Ok(Async::NotReady)
    }
}
