// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use tower::Service;

pub(crate) trait CloneSyncService<Req, Res, Err>: Send + Sync {
    fn clone_box(&self) -> Box<dyn CloneSyncService<Req, Res, Err>>;
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Err>>;
    fn call(&mut self, req: Req) -> Pin<Box<dyn Future<Output = Result<Res, Err>> + Send>>;
}

struct BoxServiceClone<T> {
    inner: T,
}

impl<T: Clone> Clone for BoxServiceClone<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<Req, Res: 'static, Err: 'static, T> CloneSyncService<Req, Res, Err> for BoxServiceClone<T>
where
    T: Service<Req, Response = Res, Error = Err> + Clone + Send + Sync + 'static,
    T::Future: Future<Output = Result<Res, Err>> + Send + 'static,
{
    fn clone_box(&self) -> Box<dyn CloneSyncService<Req, Res, Err>> {
        Box::new(self.clone())
    }

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Err>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Req) -> Pin<Box<dyn Future<Output = Result<Res, Err>> + Send>> {
        Box::pin(self.inner.call(req))
    }
}

/// A [`Clone`], [`Send`], and [`Sync`] boxed [`Service`].
///
/// This type wraps any service that implements [`Service`], [`Clone`], [`Send`], and [`Sync`],
/// making it suitable for storage in thread-safe collections and use with frameworks like Axum.
pub struct BoxCloneSyncService<Req, Res, Err> {
    inner: Box<dyn CloneSyncService<Req, Res, Err>>,
}

impl<Req, Res: 'static, Err: 'static> BoxCloneSyncService<Req, Res, Err> {
    /// Creates a new [`BoxCloneSyncService`] wrapping the given service.
    pub fn new<S>(service: S) -> Self
    where
        S: Service<Req, Response = Res, Error = Err> + Clone + Send + Sync + 'static,
        S::Future: Future<Output = Result<Res, Err>> + Send + 'static,
    {
        Self {
            inner: Box::new(BoxServiceClone { inner: service }),
        }
    }
}

impl<Req, Res, Err> Clone for BoxCloneSyncService<Req, Res, Err> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone_box(),
        }
    }
}

impl<Req, Res, Err> Service<Req> for BoxCloneSyncService<Req, Res, Err> {
    type Response = Res;
    type Error = Err;
    type Future = Pin<Box<dyn Future<Output = Result<Res, Err>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Req) -> Self::Future {
        self.inner.call(req)
    }
}
