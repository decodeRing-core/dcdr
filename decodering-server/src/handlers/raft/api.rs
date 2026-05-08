use std::io::Cursor;

use actix_web::web::{Data, Json};
use actix_web::{HttpResponse, Responder};
use decodering_core::tx::Database;
use decodering_raft::raft_types::AppendEntriesRequest;
use decodering_raft::raft_types::Snapshot;
use decodering_raft::raft_types::SnapshotMetaOf;
use decodering_raft::raft_types::VoteOf;
use decodering_raft::raft_types::VoteRequest;

use crate::app_data::AppData;

pub async fn vote_raft<D: Database + 'static>(
    app: Data<AppData<D>>,
    req: Json<VoteRequest>,
) -> impl Responder {
    match &app.raft {
        Some(raft_bits) => {
            let result = raft_bits.vote(req.into_inner()).await;
            HttpResponse::Ok().json(result)
        }
        _ => HttpResponse::ServiceUnavailable().finish(),
    }
}

pub async fn append_raft<D: Database + 'static>(
    app: Data<AppData<D>>,
    req: Json<AppendEntriesRequest>,
) -> impl Responder {
    match &app.raft {
        Some(raft_bits) => {
            let result = raft_bits.append(req.into_inner()).await;
            HttpResponse::Ok().json(result)
        }
        _ => HttpResponse::ServiceUnavailable().finish(),
    }
}

pub async fn snapshot_raft<D: Database + 'static>(
    app: Data<AppData<D>>,
    req: Json<(VoteOf, SnapshotMetaOf, Vec<u8>)>,
) -> impl Responder {
    match &app.raft {
        Some(raft_bits) => {
            let (vote, meta, data) = req.into_inner();
            let snapshot = Snapshot {
                meta,
                snapshot: Cursor::new(data),
            };

            let result = raft_bits.raft.install_full_snapshot(vote, snapshot).await;
            HttpResponse::Ok().json(result)
        }
        _ => HttpResponse::ServiceUnavailable().finish(),
    }
}
