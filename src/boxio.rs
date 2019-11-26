use std::io;
use std::io::ErrorKind;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite};
use tokio_net::tcp::TcpStream;
use tokio_rustls::TlsAcceptor;

pub trait Io:
AsyncRead + AsyncWrite + Send + Unpin + 'static
{}

impl<T> Io for T where T: AsyncRead + AsyncWrite + Send + Unpin + 'static {}

pub struct BoxedIo(Pin<Box<dyn Io>>);

impl BoxedIo {
    pub fn new<I: Io>(io: I) -> Self {
        BoxedIo(Box::pin(io))
    }
}

impl AsyncRead for BoxedIo {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.0).poll_read(cx, buf)
    }
}

impl AsyncWrite for BoxedIo {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.0).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_shutdown(cx)
    }
}

pub async fn connect(acceptor: TlsAcceptor, io: TcpStream) -> Result<BoxedIo, std::io::Error> {
    let io = {
        let tls = acceptor.accept(io).await.map_err(|e| std::io::Error::from(ErrorKind::NotFound))?;
        BoxedIo::new(tls)
    };
    Ok(io)
}
