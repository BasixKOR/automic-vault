use crate::policy::is_public_ip;
use hudsucker::hyper::{Uri, rt};
use hyper_util::client::legacy::connect::{Connected, Connection, HttpConnector, dns::Name};
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::time::{Instant, Sleep};
use tower_service::Service;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub(crate) struct SafeResolver;

impl Service<Name> for SafeResolver {
    type Response = std::vec::IntoIter<SocketAddr>;
    type Error = io::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _context: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, name: Name) -> Self::Future {
        let hostname = name.as_str().to_string();
        Box::pin(async move {
            let addresses = tokio::net::lookup_host((hostname.as_str(), 0))
                .await?
                .collect::<Vec<_>>();
            if addresses.is_empty() || addresses.iter().any(|address| !is_public_ip(address.ip())) {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "destination resolved to a private or reserved address",
                ));
            }
            Ok(addresses.into_iter())
        })
    }
}

pub(crate) fn http_connector() -> HttpConnector<SafeResolver> {
    let mut connector = HttpConnector::new_with_resolver(SafeResolver);
    connector.enforce_http(false);
    connector.set_connect_timeout(Some(REQUEST_TIMEOUT));
    connector.set_nodelay(true);
    connector
}

#[derive(Clone)]
pub(crate) struct DeadlineConnector<C> {
    inner: C,
}

impl<C> DeadlineConnector<C> {
    pub(crate) fn new(inner: C) -> Self {
        Self { inner }
    }
}

impl<C> Service<Uri> for DeadlineConnector<C>
where
    C: Service<Uri> + Clone + Send + 'static,
    C::Response: rt::Read + rt::Write + Connection + Unpin + Send + 'static,
    C::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    C::Future: Send + 'static,
{
    type Response = DeadlineConnection<C::Response>;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, context: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(context).map_err(Into::into)
    }

    fn call(&mut self, uri: Uri) -> Self::Future {
        let future = self.inner.call(uri);
        Box::pin(async move {
            let deadline = Instant::now() + REQUEST_TIMEOUT;
            let inner = tokio::time::timeout_at(deadline, future)
                .await
                .map_err(|_| timeout_error())?
                .map_err(Into::into)?;
            Ok(DeadlineConnection::new(inner, deadline))
        })
    }
}

pub(crate) struct DeadlineConnection<T> {
    inner: T,
    deadline: Pin<Box<Sleep>>,
}

impl<T> DeadlineConnection<T> {
    fn new(inner: T, deadline: Instant) -> Self {
        Self {
            inner,
            deadline: Box::pin(tokio::time::sleep_until(deadline)),
        }
    }

    fn poll_deadline(&mut self, context: &mut Context<'_>) -> io::Result<()> {
        if self.deadline.as_mut().poll(context).is_ready() {
            Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "upstream request exceeded 30 seconds",
            ))
        } else {
            Ok(())
        }
    }
}

impl<T: Connection> Connection for DeadlineConnection<T> {
    fn connected(&self) -> Connected {
        self.inner.connected()
    }
}

impl<T: rt::Read + Unpin> rt::Read for DeadlineConnection<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: rt::ReadBufCursor<'_>,
    ) -> Poll<Result<(), io::Error>> {
        if let Err(error) = self.poll_deadline(context) {
            return Poll::Ready(Err(error));
        }
        Pin::new(&mut self.inner).poll_read(context, buffer)
    }
}

impl<T: rt::Write + Unpin> rt::Write for DeadlineConnection<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        if let Err(error) = self.poll_deadline(context) {
            return Poll::Ready(Err(error));
        }
        Pin::new(&mut self.inner).poll_write(context, buffer)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        if let Err(error) = self.poll_deadline(context) {
            return Poll::Ready(Err(error));
        }
        Pin::new(&mut self.inner).poll_flush(context)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        Pin::new(&mut self.inner).poll_shutdown(context)
    }

    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffers: &[io::IoSlice<'_>],
    ) -> Poll<Result<usize, io::Error>> {
        if let Err(error) = self.poll_deadline(context) {
            return Poll::Ready(Err(error));
        }
        Pin::new(&mut self.inner).poll_write_vectored(context, buffers)
    }
}

fn timeout_error() -> Box<dyn std::error::Error + Send + Sync> {
    Box::new(io::Error::new(
        io::ErrorKind::TimedOut,
        "upstream connection exceeded 30 seconds",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn resolver_rejects_localhost() {
        let result = SafeResolver.call("localhost".parse().unwrap()).await;
        assert!(result.is_err());
    }
}
