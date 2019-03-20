// re-export i3ipc-types so users only have to import 1 thing
pub use i3ipc_types::*;

use bytes::{Buf, BufMut, Bytes, BytesMut, IntoBuf};
use futures::{try_ready, Async, Future, Poll};
use serde::de::DeserializeOwned;
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_uds::{ConnectFuture, UnixStream};

use std::{
    env,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::Command,
    os::unix::net,
};

#[derive(Debug)]
pub struct MsgResponse<D> {
    pub msg_type: msg::Msg,
    pub payload: D,
}

#[derive(Debug)]
pub struct EventResponse<D> {
    pub evt_type: event::Event,
    pub payload: D,
}

#[derive(Debug)]
pub struct I3Connect(ConnectFuture);

impl I3Connect {
    pub fn socket_path() -> io::Result<String> {
        if let Ok(p) = env::var("I3SOCK") {
            return Ok(p);
        }
        let out = Command::new("i3").arg("--get-socketpath").output()?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
        } else {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "Unable to get i3 socket path",
            ))
        }
    }
    pub fn new() -> io::Result<Self> {
        Ok(I3Connect(UnixStream::connect(I3Connect::socket_path()?)))
    }
    pub fn connect(&mut self) -> Poll<I3Stream, io::Error> {
        let stream = try_ready!(self.0.poll());
        Ok(Async::Ready(I3Stream(stream)))
    }
    pub fn from_stream(handle: &Handle) -> io::Result<I3Stream> {
        let std_stream = net::UnixStream::new(I3Connect::socket_path()?);
        UnixStream::from_std(std_stream, handle)
    }
}

#[derive(Debug)]
pub struct I3Stream(UnixStream);

impl I3Stream {
    pub const MAGIC: &'static str = "i3-ipc";
    pub fn subscribe<E, D>(&mut self, events: E) -> Poll<EventResponse<D>, io::Error>
    where
        E: AsRef<[event::Event]>,
        D: DeserializeOwned,
    {
        let sub_json = serde_json::to_string(events.as_ref())?;
        loop {
            let _ = try_ready!(self.send_msg(Msg::Subscribe, &sub_json));
            let resp: MsgResponse<reply::Success> = try_ready!(self.receive_msg());
dbg!(resp);
unimplemented!()
        }
    }

    pub fn send_msg<P>(&mut self, msg: msg::Msg, payload: P) -> Poll<usize, io::Error>
    where
        P: AsRef<str>,
    {
        let payload = payload.as_ref();
        let mut buf = BytesMut::with_capacity(14 + payload.len());
        buf.put_slice(I3Stream::MAGIC.as_bytes());
        buf.put_u32_le(payload.len() as u32);
        buf.put_u32_le(msg.into());
        buf.put_slice(payload.as_bytes());
        self.write_buf(&mut buf.into_buf())
    }

    pub fn receive_msg<D: DeserializeOwned>(&mut self) -> Poll<MsgResponse<D>, io::Error> {
        let mut buf = BytesMut::with_capacity(6);
        let l = try_ready!(self.read_buf(&mut buf));
        dbg!(&buf);
        let magic_str = buf.take();
        if magic_str != I3Stream::MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Expected 'i3-ipc' but received: {:?}", magic_str),
            ));
        }

        let len = try_ready!(self.read_buf(&mut buf));
        unimplemented!()
    }

    pub fn send_receive<D: DeserializeOwned>(&mut self) -> Poll<MsgResponse<D>, io::Error> {
        unimplemented!()
    }
}

impl Read for I3Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl Write for I3Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl AsyncRead for I3Stream {
    unsafe fn prepare_uninitialized_buffer(&self, buf: &mut [u8]) -> bool {
        self.0.prepare_uninitialized_buffer(buf)
    }

    fn read_buf<B>(&mut self, buf: &mut B) -> Poll<usize, io::Error>
    where
        B: BufMut,
    {
        self.0.read_buf(buf)
    }
}

impl AsyncWrite for I3Stream {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        match self {
            I3Stream(s) => s.shutdown(),
        }
    }

    fn write_buf<B>(&mut self, buf: &mut B) -> Poll<usize, io::Error>
    where
        B: Buf,
    {
        self.0.write_buf(buf)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test() -> Result<(), Box<dyn std::error::Error>> {
//         let fut = I3Connect::new()?.connect().and_then(|stream| {
//             stream.subscribe(&[event::Event::Window]).map(|o| {dbg!(o); () }).map_err(|e| eprintln!("{:?}", e));
//             futures::ok(())
//         });
//         tokio::run(fut);
//         Ok(())
//     }
// }