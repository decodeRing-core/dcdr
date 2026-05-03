use std::future::{Ready, ready};
use std::marker::PhantomData;

use actix_web::body::{EitherBody, MessageBody};
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready};
use actix_web::{Error, HttpMessage};
use actix_web::{Responder, web};
use decodering_db::Database;
use futures_util::future::LocalBoxFuture;
use tracing::trace;
use tracing::{Instrument, Level, field, span};

use crate::app_data::AppData;
use crate::handlers::response::{ApiResponse, ErrorStatus};

pub(crate) struct TracingHelper;

impl<S, B> Transform<S, ServiceRequest> for TracingHelper
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = TracingHelperMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(TracingHelperMiddleware { service }))
    }
}

pub(crate) struct TracingHelperMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for TracingHelperMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let span = span!(Level::TRACE, "request", user_id = field::Empty);
        let _guard = span.enter();
        let _ = req.extensions_mut().insert(span.clone());
        trace!("Uri: {}", req.path());
        let fut = self.service.call(req).in_current_span();
        Box::pin(async move {
            let res = fut.await?;
            Ok(res)
        })
    }
}

pub(crate) struct LockState<D: Database>(PhantomData<D>);

impl<D: Database> LockState<D> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<D: Database> Default for LockState<D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, B, D> Transform<S, ServiceRequest> for LockState<D>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
    D: Database + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>; // ← changed
    type Error = Error;
    type InitError = ();
    type Transform = LockStateHelperMiddleware<S, D>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(LockStateHelperMiddleware {
            service,
            _db: PhantomData,
        }))
    }
}

pub(crate) struct LockStateHelperMiddleware<S, D: Database> {
    service: S,
    _db: PhantomData<D>,
}

impl<S, B, D> Service<ServiceRequest> for LockStateHelperMiddleware<S, D>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
    D: Database + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>; // ← changed
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let app_state = req.app_data::<web::Data<AppData<D>>>();

        if let Some(state) = app_state {
            if state.master_key.get().is_none() {
                let (http_req, _payload) = req.into_parts();
                let response = ApiResponse::<()>::error(ErrorStatus::Locked)
                    .respond_to(&http_req)
                    .map_into_right_body(); // ← BoxBody → EitherBody::Right
                return Box::pin(async move { Ok(ServiceResponse::new(http_req, response)) });
            }
        }

        let fut = self.service.call(req).in_current_span();
        Box::pin(async move {
            let res = fut.await?.map_into_left_body(); // ← B → EitherBody::Left
            Ok(res)
        })
    }
}
