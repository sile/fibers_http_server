use std::io::{Read, Write};
use bytecodec::{self, ByteCount, Decode, Eos};
use bytecodec::io::{ReadBuf, WriteBuf};

// TODO: move to bytecodec
#[derive(Debug)]
pub struct BufferedIo<T> {
    stream: T,
    rbuf: ReadBuf<Vec<u8>>,
    wbuf: WriteBuf<Vec<u8>>,
}
impl<T: Read + Write> BufferedIo<T> {
    pub fn new(stream: T, read_buf_size: usize, write_buf_size: usize) -> Self {
        BufferedIo {
            stream,
            rbuf: ReadBuf::new(vec![0; read_buf_size]),
            wbuf: WriteBuf::new(vec![0; write_buf_size]),
        }
    }

    // pub fn read_buf(&self) -> &ReadBuf<Vec<u8>> {
    //     &self.rbuf
    // }

    pub fn read_buf_mut(&mut self) -> &mut ReadBuf<Vec<u8>> {
        &mut self.rbuf
    }

    pub fn write_buf_mut(&mut self) -> &mut WriteBuf<Vec<u8>> {
        &mut self.wbuf
    }

    pub fn write_buf(&self) -> &WriteBuf<Vec<u8>> {
        &self.wbuf
    }

    pub fn execute_io(&mut self) -> bytecodec::Result<()> {
        track!(self.rbuf.fill(&mut self.stream))?;
        track!(self.wbuf.flush(&mut self.stream))?;
        Ok(())
    }

    pub fn is_eos(&self) -> bool {
        self.rbuf.stream_state().is_eos() || self.wbuf.stream_state().is_eos()
    }

    pub fn would_block(&self) -> bool {
        self.rbuf.stream_state().would_block()
            && (self.wbuf.is_empty() || self.wbuf.stream_state().would_block())
    }

    // TODO: stream_ref(), stream_mut(), into_stream()
}

#[derive(Debug)]
pub struct MaybeEos<D> {
    inner: D,
    started: bool,
}
impl<D> MaybeEos<D> {
    pub fn new(inner: D) -> Self {
        MaybeEos {
            inner,
            started: false,
        }
    }
}
impl<D: Decode> Decode for MaybeEos<D> {
    type Item = D::Item;

    fn decode(
        &mut self,
        buf: &[u8],
        mut eos: Eos,
    ) -> bytecodec::Result<(usize, Option<Self::Item>)> {
        if !self.started && eos.is_reached() {
            eos = Eos::new(false);
        }
        let (size, item) = track!(self.inner.decode(buf, eos))?;
        if let Some(item) = item {
            self.started = false;
            Ok((size, Some(item)))
        } else {
            self.started |= size > 0;
            Ok((size, None))
        }
    }

    fn has_terminated(&self) -> bool {
        self.inner.has_terminated()
    }

    fn requiring_bytes(&self) -> ByteCount {
        self.inner.requiring_bytes()
    }
}
