//! TcpListener and it's implements.
use std::io::{Error as IoError, ErrorKind, Result as IoResult};
use std::sync::Arc;
use std::vec;

use tokio::net::{TcpListener as TokioTcpListener, TcpStream, ToSocketAddrs};

use crate::async_trait;
use crate::conn::{AppProto, LocalAddr, TransProto};
use crate::conn::HttpBuilders;
use crate::http::{HttpConnection, Version};
use crate::service::HyperHandler;

use super::{Accepted, Acceptor, IntoAcceptor, Listener};

/// TcpListener
pub struct TcpListener<T> {
    addr: T,
}
impl<T: ToSocketAddrs> TcpListener<T> {
    /// Bind to socket address.
    #[inline]
    pub fn bind(addr: T) -> Self {
        TcpListener { addr }
    }
}
#[async_trait]
impl<T> IntoAcceptor for TcpListener<T>
where
    T: ToSocketAddrs + Send,
{
    type Acceptor = TcpAcceptor;
    async fn into_acceptor(self) -> IoResult<Self::Acceptor> {
        let inner = TokioTcpListener::bind(self.addr).await?;
        let local_addr = LocalAddr::new(inner.local_addr()?.into(), TransProto::Tcp, AppProto::Http);
        Ok(TcpAcceptor { inner, local_addr })
    }
}
impl<T> Listener for TcpListener<T> where T: ToSocketAddrs + Send {}

pub struct TcpAcceptor {
    inner: TokioTcpListener,
    local_addr: LocalAddr,
}

#[async_trait]
impl HttpConnection for TcpStream {
    async fn version(&mut self) -> Option<Version> {
        Some(Version::HTTP_11)
    }
    async fn serve(self, handler: HyperHandler, builders: Arc<HttpBuilders>) -> IoResult<()> {
        builders
            .http1
            .serve_connection(self, handler)
            .with_upgrades()
            .await
            .map_err(|e| IoError::new(ErrorKind::Other, e.to_string()))
    }
}

#[async_trait]
impl Acceptor for TcpAcceptor {
    type Conn = TcpStream;

    #[inline]
    fn local_addrs(&self) -> Vec<LocalAddr> {
        vec![self.local_addr.clone()]
    }

    #[inline]
    async fn accept(&mut self) -> IoResult<Accepted<Self::Conn>> {
        self.inner.accept().await.map(move |(conn, remote_addr)| Accepted {
            conn,
            local_addr: self.local_addr.clone(),
            remote_addr: remote_addr.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use futures_util::{Stream, StreamExt};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    use super::*;

    #[tokio::test]
    async fn test_tcp_listener() {
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 6878));

        let listener = TcpListener::bind(addr);
        let mut acceptor = listener.into_acceptor().await.unwrap();
        let addr = acceptor.local_addrs().remove(0).into_std().unwrap();
        tokio::spawn(async move {
            let mut stream = TcpStream::connect(addr).await.unwrap();
            stream.write_i32(150).await.unwrap();
        });

        let Accepted { mut conn, .. } = acceptor.accept().await.unwrap();
        assert_eq!(conn.read_i32().await.unwrap(), 150);
    }
}
