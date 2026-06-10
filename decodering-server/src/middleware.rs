use std::time::Instant;

use actix_web::Responder;
use actix_web::body::{EitherBody, MessageBody};
use actix_web::dev::ServiceRequest;
use actix_web::dev::ServiceResponse;
use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::middleware::Next;
use actix_web::web;
use actix_web::{Error, HttpMessage};
use decodering_core::operation::{HTTP_REQUEST_DURATION_SECONDS, HTTP_REQUESTS_TOTAL};
use decodering_core::tx::Database;
use metrics::{counter, histogram};
use tracing::Instrument;
use tracing_actix_web::RequestId;

use crate::app_data::AppData;
use crate::config::Config;
use crate::config::StorageConfig;
use crate::error::ErrorReason;
use crate::handlers::response::ApiResponse;
use crate::handlers::response::ErrorStatus;

const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

pub async fn propagate_request_id<B: MessageBody>(
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<B>, Error> {
    let req_id = req.extensions().get::<RequestId>().copied();
    let mut res = next.call(req).await?;
    if let Some(id) = req_id
        && let Ok(val) = HeaderValue::from_str(&id.to_string())
    {
        res.headers_mut().insert(X_REQUEST_ID, val);
    }
    Ok(res)
}

pub async fn require_unlocked<D, B>(
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<EitherBody<B>>, Error>
where
    D: Database + 'static,
    B: MessageBody + 'static,
{
    let locked = req
        .app_data::<web::Data<AppData<D>>>()
        .is_some_and(|state| state.master_key.get().is_none());

    if locked {
        let (http_req, _payload) = req.into_parts();
        let response = ApiResponse::<()>::error(ErrorStatus::OperationFailed(ErrorReason::Locked))
            .respond_to(&http_req)
            .map_into_right_body();
        return Ok(ServiceResponse::new(http_req, response));
    }

    Ok(next.call(req).in_current_span().await?.map_into_left_body())
}

pub async fn require_raft_initialized<D, B>(
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<EitherBody<B>>, Error>
where
    D: Database + 'static,
    B: MessageBody + 'static,
{
    let raft = req
        .app_data::<web::Data<AppData<D>>>()
        .and_then(|state| state.raft.clone());

    let Some(raft_bits) = raft else {
        return Ok(next.call(req).in_current_span().await?.map_into_left_body());
    };

    if !matches!(raft_bits.raft.is_initialized().await, Ok(true)) {
        let (http_req, _payload) = req.into_parts();
        let response = ApiResponse::<()>::error(ErrorStatus::OperationFailed(
            ErrorReason::RaftNotInitialized,
        ))
        .respond_to(&http_req)
        .map_into_right_body();
        return Ok(ServiceResponse::new(http_req, response));
    }

    Ok(next.call(req).in_current_span().await?.map_into_left_body())
}

pub async fn require_raft_leader<D, B>(
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<EitherBody<B>>, Error>
where
    B: MessageBody + 'static,
    D: Database + 'static,
{
    let raft = req
        .app_data::<web::Data<AppData<D>>>()
        .and_then(|s| s.raft.clone());

    let Some(raft_bits) = raft else {
        return Ok(next.call(req).await?.map_into_left_body());
    };

    if !raft_bits.raft.is_leader() {
        let (http_req, _payload) = req.into_parts();
        let resp =
            ApiResponse::<()>::error(ErrorStatus::OperationFailed(ErrorReason::RaftNotLeader))
                .respond_to(&http_req)
                .map_into_right_body();
        return Ok(ServiceResponse::new(http_req, resp));
    }

    Ok(next.call(req).await?.map_into_left_body())
}

pub async fn require_raft_backend<B>(
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<EitherBody<B>>, Error>
where
    B: MessageBody + 'static,
{
    let is_raft = req
        .app_data::<web::Data<Config>>()
        .is_some_and(|config| matches!(config.storage, StorageConfig::Raft { .. }));

    if !is_raft {
        let (http_req, _payload) = req.into_parts();
        let response =
            ApiResponse::<()>::error(ErrorStatus::OperationFailed(ErrorReason::RaftNotAvailable))
                .respond_to(&http_req)
                .map_into_right_body();
        return Ok(ServiceResponse::new(http_req, response));
    }

    Ok(next.call(req).in_current_span().await?.map_into_left_body())
}

pub async fn track_http(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let start = Instant::now();
    let method = req.method().as_str().to_owned();
    let path = req
        .match_pattern()
        .unwrap_or_else(|| "unmatched".to_owned());

    let res = next.call(req).await?;

    let status = res.status().as_u16().to_string();
    counter!(HTTP_REQUESTS_TOTAL,
        "method" => method.clone(), "path" => path.clone(), "status" => status)
    .increment(1);
    histogram!(HTTP_REQUEST_DURATION_SECONDS,
        "method" => method, "path" => path)
    .record(start.elapsed().as_secs_f64());

    Ok(res)
}
