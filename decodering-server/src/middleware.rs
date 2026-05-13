use std::future::Ready;
use std::future::ready;
use std::marker::PhantomData;

use actix_web::Responder;
use actix_web::body::{EitherBody, MessageBody};
use actix_web::dev::Service;
use actix_web::dev::ServiceRequest;
use actix_web::dev::ServiceResponse;
use actix_web::dev::Transform;
use actix_web::dev::forward_ready;
use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::web;
use actix_web::{Error, HttpMessage};
use decodering_core::tx::Database;
use futures_util::future::LocalBoxFuture;
use std::rc::Rc;
use tracing::Instrument;
use tracing_actix_web::RequestId;

use crate::app_data::AppData;
use crate::config::StorageConfig;
use crate::config::get_config;
use crate::error::ErrorReason;
use crate::handlers::response::ApiResponse;
use crate::handlers::response::ErrorStatus;

const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

pub struct PropagateRequestId;

impl<S, B> Transform<S, ServiceRequest> for PropagateRequestId
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = PropagateRequestIdMw<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(PropagateRequestIdMw {
            service: Rc::new(service),
        }))
    }
}

pub struct PropagateRequestIdMw<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for PropagateRequestIdMw<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let req_id = req.extensions().get::<RequestId>().copied();
        let svc = self.service.clone();
        Box::pin(async move {
            let mut res = svc.call(req).await?;
            if let Some(id) = req_id
                && let Ok(val) = HeaderValue::from_str(&id.to_string())
            {
                res.headers_mut().insert(X_REQUEST_ID, val);
            }
            Ok(res)
        })
    }
}

pub struct LockState<D: Database>(PhantomData<D>);

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

pub struct LockStateHelperMiddleware<S, D: Database> {
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

        if let Some(state) = app_state
            && state.master_key.get().is_none()
        {
            let (http_req, _payload) = req.into_parts();
            let response =
                ApiResponse::<()>::error(ErrorStatus::OperationFailed(ErrorReason::Locked))
                    .respond_to(&http_req)
                    .map_into_right_body(); // ← BoxBody → EitherBody::Right
            return Box::pin(async move { Ok(ServiceResponse::new(http_req, response)) });
        }

        let fut = self.service.call(req).in_current_span();
        Box::pin(async move {
            let res = fut.await?.map_into_left_body(); // ← B → EitherBody::Left
            Ok(res)
        })
    }
}

pub struct RaftInitializedHelper<D: Database> {
    _db: PhantomData<D>,
}

impl<D: Database> RaftInitializedHelper<D> {
    pub fn new() -> Self {
        Self { _db: PhantomData }
    }
}

impl<D: Database> Default for RaftInitializedHelper<D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, B, D> Transform<S, ServiceRequest> for RaftInitializedHelper<D>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
    D: Database + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Transform = RaftInitializedHelperMiddleware<S, D>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RaftInitializedHelperMiddleware {
            service: Rc::new(service),
            _db: PhantomData,
        }))
    }
}

pub struct RaftInitializedHelperMiddleware<S, D: Database> {
    service: Rc<S>,
    _db: PhantomData<D>,
}

impl<S, B, D> Service<ServiceRequest> for RaftInitializedHelperMiddleware<S, D>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
    D: Database + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let raft = req
            .app_data::<web::Data<AppData<D>>>()
            .and_then(|state| state.raft.clone());

        let srv = Rc::clone(&self.service);

        let Some(raft_bits) = raft else {
            let fut = srv.call(req).in_current_span();
            return Box::pin(async move {
                let res = fut.await?.map_into_left_body();
                Ok(res)
            });
        };

        Box::pin(
            async move {
                let (http_req, payload) = req.into_parts();

                let is_initialized = raft_bits.raft.is_initialized().await;
                if !matches!(is_initialized, Ok(true)) {
                    let response = ApiResponse::<()>::error(ErrorStatus::OperationFailed(
                        ErrorReason::RaftNotInitialized,
                    ))
                    .respond_to(&http_req)
                    .map_into_right_body();
                    return Ok(ServiceResponse::new(http_req, response));
                }

                let req = ServiceRequest::from_parts(http_req, payload);
                let res = srv.call(req).await?.map_into_left_body();
                Ok(res)
            }
            .in_current_span(),
        )
    }
}

pub struct RaftLeaderHelper<D: Database> {
    _db: PhantomData<D>,
}

impl<D: Database> RaftLeaderHelper<D> {
    pub fn new() -> Self {
        Self { _db: PhantomData }
    }
}

impl<D: Database> Default for RaftLeaderHelper<D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, B, D> Transform<S, ServiceRequest> for RaftLeaderHelper<D>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
    D: Database + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Transform = RaftLeaderHelperMiddleware<S, D>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RaftLeaderHelperMiddleware {
            service: Rc::new(service),
            _db: PhantomData,
        }))
    }
}

pub struct RaftLeaderHelperMiddleware<S, D: Database> {
    service: Rc<S>,
    _db: PhantomData<D>,
}

impl<S, B, D> Service<ServiceRequest> for RaftLeaderHelperMiddleware<S, D>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
    D: Database + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let raft = req
            .app_data::<web::Data<AppData<D>>>()
            .and_then(|state| state.raft.clone());

        let srv = Rc::clone(&self.service);

        let Some(raft_bits) = raft else {
            let fut = srv.call(req).in_current_span();
            return Box::pin(async move {
                let res = fut.await?.map_into_left_body();
                Ok(res)
            });
        };

        Box::pin(
            async move {
                let (http_req, payload) = req.into_parts();

                if !raft_bits.raft.is_leader() {
                    let response = ApiResponse::<()>::error(ErrorStatus::OperationFailed(
                        ErrorReason::RaftNotLeader,
                    ))
                    .respond_to(&http_req)
                    .map_into_right_body();
                    return Ok(ServiceResponse::new(http_req, response));
                }

                let req = ServiceRequest::from_parts(http_req, payload);
                let res = srv.call(req).await?.map_into_left_body();
                Ok(res)
            }
            .in_current_span(),
        )
    }
}

pub struct RaftBackendOnly<D: Database> {
    _db: PhantomData<D>,
}

impl<D: Database> RaftBackendOnly<D> {
    pub fn new() -> Self {
        Self { _db: PhantomData }
    }
}

impl<D: Database> Default for RaftBackendOnly<D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, B, D> Transform<S, ServiceRequest> for RaftBackendOnly<D>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
    D: Database + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Transform = RaftBackendOnlyMiddleware<S, D>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RaftBackendOnlyMiddleware {
            service: Rc::new(service),
            _db: PhantomData,
        }))
    }
}

pub struct RaftBackendOnlyMiddleware<S, D: Database> {
    service: Rc<S>,
    _db: PhantomData<D>,
}

impl<S, B, D> Service<ServiceRequest> for RaftBackendOnlyMiddleware<S, D>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
    D: Database + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let srv = Rc::clone(&self.service);

        if !matches!(get_config().storage, StorageConfig::Raft { .. }) {
            let (http_req, _payload) = req.into_parts();
            let response = ApiResponse::<()>::error(ErrorStatus::OperationFailed(
                ErrorReason::RaftNotAvailable,
            ))
            .respond_to(&http_req)
            .map_into_right_body();
            return Box::pin(async move { Ok(ServiceResponse::new(http_req, response)) });
        }

        Box::pin(async move {
            let res = srv.call(req).in_current_span().await?.map_into_left_body();
            Ok(res)
        })
    }
}
