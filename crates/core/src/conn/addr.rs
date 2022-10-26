//! addr module
use std::fmt::{self, Display, Formatter};
use std::ops::{Deref, DerefMut};
#[cfg(unix)]
use std::sync::Arc;

use super::{TransProto, AppProto};

/// Network socket address
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum SocketAddr {
    /// Unknown address
    Unknown,
    /// IPv4 socket address
    IPv4(std::net::SocketAddrV4),
    /// IPv6 socket address
    IPv6(std::net::SocketAddrV6),
    /// Unix socket address
    #[cfg(unix)]
    #[cfg_attr(docsrs, doc(cfg(unix)))]
    Unix(Arc<tokio::net::unix::SocketAddr>),
}
impl From<std::net::SocketAddr> for SocketAddr {
    #[inline]
    fn from(addr: std::net::SocketAddr) -> Self {
        match addr {
            std::net::SocketAddr::V4(val) => SocketAddr::IPv4(val),
            std::net::SocketAddr::V6(val) => SocketAddr::IPv6(val),
        }
    }
}
impl From<std::net::SocketAddrV4> for SocketAddr {
    #[inline]
    fn from(addr: std::net::SocketAddrV4) -> Self {
        SocketAddr::IPv4(addr)
    }
}
impl From<std::net::SocketAddrV6> for SocketAddr {
    #[inline]
    fn from(addr: std::net::SocketAddrV6) -> Self {
        SocketAddr::IPv6(addr)
    }
}

#[cfg(unix)]
impl From<tokio::net::unix::SocketAddr> for SocketAddr {
    #[inline]
    fn from(addr: tokio::net::unix::SocketAddr) -> Self {
        SocketAddr::Unix(addr.into())
    }
}
impl SocketAddr {
    /// Returns is a ipv4 socket address.
    #[inline]
    pub fn is_ipv4(&self) -> bool {
        matches!(*self, SocketAddr::IPv4(_))
    }
    /// Returns is a ipv6 socket address.
    #[inline]
    pub fn is_ipv6(&self) -> bool {
        matches!(*self, SocketAddr::IPv6(_))
    }

    /// Convert to [`std::net::SocketAddr`].
    #[inline]
    pub fn into_std(self) -> Option<std::net::SocketAddr> {
        match self {
            SocketAddr::IPv4(addr) => Some(addr.into()),
            SocketAddr::IPv6(addr) => Some(addr.into()),
            _ => None,
        }
    }

    cfg_feature! {
        #![unix]
        /// Returns is a unix socket address.
        #[inline]
        pub fn is_unix(&self) -> bool {
            matches!(*self, SocketAddr::Unix(_))
        }
    }

    /// Returns ipv6 socket address.
    #[inline]
    pub fn as_ipv6(&self) -> Option<&std::net::SocketAddrV6> {
        match self {
            SocketAddr::IPv6(addr) => Some(addr),
            _ => None,
        }
    }
    /// Returns ipv4 socket address.
    #[inline]
    pub fn as_ipv4(&self) -> Option<&std::net::SocketAddrV4> {
        match self {
            SocketAddr::IPv4(addr) => Some(addr),
            _ => None,
        }
    }

    cfg_feature! {
        #![unix]
        /// Returns unix socket address.
        #[inline]
        pub fn as_unix(&self) -> Option<&tokio::net::unix::SocketAddr> {
            match self {
                SocketAddr::Unix(addr) => Some(addr),
                _ => None,
            }
        }
    }
}

impl Display for SocketAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SocketAddr::Unknown => write!(f, "unknown"),
            SocketAddr::IPv4(addr) => write!(f, "socket://{}", addr),
            SocketAddr::IPv6(addr) => write!(f, "socket://{}", addr),
            #[cfg(unix)]
            SocketAddr::Unix(addr) => match addr.as_pathname() {
                Some(path) => write!(f, "unix://{}", path.display()),
                None => f.write_str("unix://unknown"),
            },
        }
    }
}

/// `LocalAddr` is a wrapper around [`SocketAddr`].
/// `LocalAddr`also contains information about
/// transport protocol and application protocol.
#[derive(Clone, Debug)]
pub struct LocalAddr {
    pub(crate) addr: SocketAddr,
    pub(crate) trans_proto: TransProto,
    pub(crate) app_proto: AppProto,
}
impl LocalAddr {
    /// Create new `LocalAddr`.
    pub fn new(addr: SocketAddr, trans_proto: TransProto, app_proto: AppProto) -> Self {
        LocalAddr {
            addr,
            trans_proto,
            app_proto,
        }
    }

    /// Convert `LocalAddr` to [`std::net::SocketAddr`].
    #[inline]
    pub fn into_std(self) -> Option<std::net::SocketAddr> {
        self.addr.into_std()
    }
}
impl Default for LocalAddr {
    fn default() -> Self {
        LocalAddr::new(SocketAddr::Unknown, TransProto::Unknown, AppProto::Unknown)
    }
}
impl Deref for LocalAddr {
    type Target = SocketAddr;

    fn deref(&self) -> &Self::Target {
        &self.addr
    }
}
impl DerefMut for LocalAddr {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.addr
    }
}

impl Display for LocalAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.addr {
            SocketAddr::Unknown => write!(f, "unknown"),
            SocketAddr::IPv4(addr) => write!(f, "({}) {}://{}", self.trans_proto, self.app_proto, addr),
            SocketAddr::IPv6(addr) => write!(f, "({}) {}://{}", self.trans_proto, self.app_proto, addr),
            #[cfg(unix)]
            SocketAddr::Unix(addr) => match addr.as_pathname() {
                Some(path) => write!(f, "({}) unix://{}", self.trans_proto, path.display()),
                None => f.write_str("({}) unix://unknown", self.trans_proto),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_addr_ipv4() {
        let ipv4: std::net::SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let ipv4: SocketAddr = ipv4.into();
        assert!(ipv4.is_ipv4());
        assert!(!ipv4.is_ipv6());
        #[cfg(target_os = "linux")]
        assert!(!ipv4.is_unix());
        assert_eq!(ipv4.as_ipv4().unwrap().to_string(), "127.0.0.1:8080");
        assert!(ipv4.as_ipv6().is_none());
        #[cfg(target_os = "linux")]
        assert!(ipv4.as_unix().is_none());
    }

    #[tokio::test]
    async fn test_addr_ipv6() {
        let ipv6 = std::net::SocketAddr::new(
            std::net::IpAddr::V6(std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 65535, 0, 1)),
            8080,
        );
        let ipv6: SocketAddr = ipv6.into();
        assert!(!ipv6.is_ipv4());
        assert!(ipv6.is_ipv6());
        #[cfg(target_os = "linux")]
        assert!(!ipv6.is_unix());
        assert!(ipv6.as_ipv4().is_none());
        assert_eq!(ipv6.as_ipv6().unwrap().to_string(), "[::ffff:0.0.0.1]:8080");
        #[cfg(target_os = "linux")]
        assert!(ipv6.as_unix().is_none());
    }
}
