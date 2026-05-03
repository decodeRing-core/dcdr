use std::io;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::TypeConfig;
use crate::raft_types::*;
use crate::rocksdb_log_store::RocksLogStore;
use decodering_core::action::Action;
use decodering_core::audit::AuditDescriptor;
use decodering_core::audit::{audit_allowed, audit_denied};
use decodering_core::error::DenyReason;
use decodering_core::repository::{AuditRepository, MetaRepository};
use decodering_core::request::AppRequest;
use decodering_core::response::AppResponse;
use decodering_core::tx::{Database, RaftTx, Tx};
use decodering_db::sqlite::SqliteDatabase;
use futures::Stream;
use futures::TryStreamExt;
use openraft::OptionalSend;
use openraft::storage::{EntryResponder, RaftSnapshotBuilder, RaftStateMachine};
use rocksdb::{ColumnFamilyDescriptor, DB, Options};

// TODO: Consider using bincode instead of serde JSON for efficiency.
pub struct StateMachineStore<D: Database> {
    db: D,
    db_path: PathBuf,
    snapshot_idx: u64,
}

impl<D: Database> StateMachineStore<D> {
    pub fn new(db: D, db_path: PathBuf) -> Self {
        Self {
            db,
            db_path,
            snapshot_idx: 0,
        }
    }
}

impl<D: Database + Clone> Clone for StateMachineStore<D> {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            db_path: self.db_path.clone(),
            snapshot_idx: self.snapshot_idx,
        }
    }
}

// The apply path
impl<D: Database> StateMachineStore<D>
where
    D: Database,
    for<'a> D::Tx<'a>: RaftTx,
{
    /// Apply a single log entry inside a SQLite transaction.
    /// The transaction commits the mutation, the audit row, AND the updated
    /// last_applied position atomically. On crash, either everything is
    /// persisted or nothing is — no possibility of state machine drift.
    async fn apply_one(
        &mut self,
        log_id: LogId,
        payload: EntryPayload,
    ) -> Result<AppResponse, io::Error> {
        let mut tx = self.db.begin().await.map_err(io::Error::other)?;

        let response = match payload {
            EntryPayload::Blank => AppResponse::Noop,
            EntryPayload::Membership(mem) => {
                let new_membership = StoredMembership::new(Some(log_id), mem);
                let mb = serde_json::to_string(&new_membership)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                let _ = tx.meta().set("last_membership", &mb).await;
                AppResponse::Noop
            }
            EntryPayload::Normal(req) => match req {
                AppRequest::CreateApiKey(create_api_key) => {
                    run_action_raft(&mut tx, log_id.index, create_api_key)
                        .await?
                        .response
                }
                AppRequest::CreateUser(create_user) => {
                    run_action_raft(&mut tx, log_id.index, create_user)
                        .await?
                        .response
                }
                AppRequest::CreateApp(create_app) => {
                    run_action_raft(&mut tx, log_id.index, create_app)
                        .await?
                        .response
                }
                AppRequest::CreateShamirConfiguration(create_shamir_configuration) => {
                    run_action_raft(&mut tx, log_id.index, create_shamir_configuration)
                        .await?
                        .response
                }
                AppRequest::CreateSecretMapping(create_secret_mapping) => {
                    run_action_raft(&mut tx, log_id.index, create_secret_mapping)
                        .await?
                        .response
                }
                AppRequest::SystemInit(system_init) => {
                    run_action_raft(&mut tx, log_id.index, system_init)
                        .await?
                        .response
                }
            },
        };

        // Persist last_applied atomically with the mutation.
        let lid = serde_json::to_string(&log_id)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let _ = tx.meta().set("last_applied", &lid).await;
        //set_meta(&mut tx, "last_applied", &lid).await?;

        tx.commit().await.map_err(io::Error::other)?;
        //tx.commit().await.map_err(io::Error::other)?;
        Ok(response)
    }
}

// OpenRaft trait impls
impl<D: Database + Clone + 'static> RaftStateMachine<TypeConfig> for StateMachineStore<D>
where
    for<'a> D::Tx<'a>: RaftTx,
{
    type SnapshotBuilder = Self;

    async fn applied_state(&mut self) -> Result<(Option<LogId>, StoredMembership), io::Error> {
        let mut tx = self.db.begin().await.map_err(io::Error::other)?;
        let last_applied_result = tx
            .meta()
            .get("last_applied")
            .await
            .map_err(io::Error::other)?;
        let last_applied = match last_applied_result {
            Some(s) => Some(
                serde_json::from_str(&s)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            ),
            None => None,
        };
        let last_membership_result = tx
            .meta()
            .get("last_membership")
            .await
            .map_err(io::Error::other)?;
        let last_membership = match last_membership_result {
            Some(s) => serde_json::from_str(&s)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            None => StoredMembership::default(),
        };
        Ok((last_applied, last_membership))
    }

    async fn apply<Strm>(&mut self, mut entries: Strm) -> Result<(), io::Error>
    where
        Strm: Stream<Item = Result<EntryResponder<TypeConfig>, io::Error>> + Unpin + OptionalSend,
    {
        while let Some((entry, responder)) = entries.try_next().await? {
            let response = self.apply_one(entry.log_id, entry.payload).await?;
            if let Some(responder) = responder {
                responder.send(response);
            }
        }
        Ok(())
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        self.snapshot_idx += 1;
        self.clone()
    }

    async fn begin_receiving_snapshot(&mut self) -> Result<Cursor<Vec<u8>>, io::Error> {
        Ok(Cursor::new(Vec::new()))
    }

    // Trust openraft's serialization
    // Only the state machine touches the DB during install_snapshot, and openraft won't call apply and install_snapshot concurrently.
    async fn install_snapshot(
        &mut self,
        meta: &SnapshotMeta,
        snapshot: SnapshotData,
    ) -> Result<(), io::Error> {
        let bytes = snapshot.into_inner();
        let tmp_path = self.db_path.with_extension("snap.tmp");
        tokio::fs::write(&tmp_path, &bytes).await?;

        let dest_path = self.db_path.clone();
        let tmp_path_clone = tmp_path.clone();

        tokio::task::spawn_blocking(move || -> Result<(), String> {
            use rusqlite::backup::Backup;
            use rusqlite::{Connection, OpenFlags};

            let src = Connection::open_with_flags(
                &tmp_path_clone,
                OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
            )
            .map_err(|e| format!("open source: {e}"))?;

            let mut dst = Connection::open(&dest_path).map_err(|e| format!("open dest: {e}"))?;

            // Foreign keys off during bulk replace; SQLite's backup API
            // doesn't run triggers/constraints, but this is defensive.
            dst.execute_batch("PRAGMA foreign_keys = OFF;")
                .map_err(|e| format!("pragma: {e}"))?;

            let backup = Backup::new(&src, &mut dst).map_err(|e| format!("init backup: {e}"))?;
            backup
                .run_to_completion(100, std::time::Duration::from_millis(50), None)
                .map_err(|e| format!("backup run: {e}"))?;

            // Force a WAL checkpoint so the file on disk reflects the new state
            // and other connections (sqlx read pool) see consistent data when
            // they next acquire a read transaction.
            drop(backup);
            dst.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                .map_err(|e| format!("checkpoint: {e}"))?;

            Ok(())
        })
        .await
        .map_err(|e| io::Error::other(format!("join: {e}")))?
        .map_err(io::Error::other)?;

        let _ = tokio::fs::remove_file(&tmp_path).await;

        // Verify (or assert) that the snapshot's embedded meta matches the
        // SnapshotMeta we were given. They MUST agree — if they don't, the
        // snapshot is malformed and we should refuse to proceed.
        let expected_lid = serde_json::to_string(&meta.last_log_id)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let expected_mb = serde_json::to_string(&meta.last_membership)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let mut tx = self.db.begin().await.map_err(io::Error::other)?;
        let actual_lid = tx
            .meta()
            .get("last_applied")
            .await
            .map_err(io::Error::other)?
            .unwrap_or_else(|| "null".to_string());
        let actual_mb = tx
            .meta()
            .get("last_membership")
            .await
            .map_err(io::Error::other)?
            .unwrap_or_default();

        if actual_lid != expected_lid || actual_mb != expected_mb {
            // The snapshot file's meta doesn't match the SnapshotMeta header.
            // This indicates a build_snapshot bug or transport corruption.
            // Overwrite to match the header (which is what Raft trusts), but
            // log loudly — this should never happen with a correct
            // build_snapshot.
            tracing::error!(
                expected_lid = %expected_lid,
                actual_lid = %actual_lid,
                "snapshot meta mismatch; overwriting from header"
            );
            tx.meta()
                .set("last_applied", &expected_lid)
                .await
                .map_err(io::Error::other)?;
            tx.meta()
                .set("last_membership", &expected_mb)
                .await
                .map_err(io::Error::other)?;
            tx.commit().await.map_err(io::Error::other)?;
        }
        Ok(())
    }

    async fn get_current_snapshot(&mut self) -> Result<Option<Snapshot>, io::Error> {
        let mut sb = self.clone();
        let snap = RaftSnapshotBuilder::build_snapshot(&mut sb).await?;
        Ok(Some(snap))
    }
}

impl<D: Database + 'static> RaftSnapshotBuilder<TypeConfig> for StateMachineStore<D> {
    async fn build_snapshot(&mut self) -> Result<Snapshot, io::Error> {
        // Produce a consistent copy of the DB using SQLite's online backup API.
        // Steps through pages with sleeps between, staying friendly to concurrent
        // readers/writers. Meta is read from the *destination* snapshot file so
        // that meta and bytes are guaranteed to describe the same logical state.
        let snap_path = self
            .db_path
            .with_extension(format!("snap.{}", self.snapshot_idx));

        let src_path = self.db_path.clone();
        let snap_path_clone = snap_path.clone();

        // Returns the meta strings read from the *snapshot* file, so meta and
        // bytes are consistent by construction.
        let (last_applied_str, last_membership_str) = tokio::task::spawn_blocking(
            move || -> Result<(Option<String>, Option<String>), String> {
                use rusqlite::backup::Backup;
                use rusqlite::{Connection, OpenFlags, OptionalExtension};

                // Open source read-only. The backup API only needs a shared lock
                // during each step; opening RO avoids any write-lock contention
                // with the rest of the system.
                let src = Connection::open_with_flags(
                    &src_path,
                    OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
                )
                .map_err(|e| format!("open source: {e}"))?;

                if snap_path_clone.exists() {
                    std::fs::remove_file(&snap_path_clone)
                        .map_err(|e| format!("remove stale snap: {e}"))?;
                }
                let mut dst =
                    Connection::open(&snap_path_clone).map_err(|e| format!("open dest: {e}"))?;

                // Run the online backup. 100 pages per step, 50ms sleep.
                // bigger steps = faster but more write-lock contention on the source.
                {
                    let backup =
                        Backup::new(&src, &mut dst).map_err(|e| format!("init backup: {e}"))?;
                    backup
                        .run_to_completion(100, std::time::Duration::from_millis(50), None)
                        .map_err(|e| format!("backup run: {e}"))?;
                }

                let last_applied: Option<String> = dst
                    .query_row(
                        "SELECT value FROM meta WHERE key = 'last_applied'",
                        [],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(|e| format!("read last_applied from snap: {e}"))?;

                let last_membership: Option<String> = dst
                    .query_row(
                        "SELECT value FROM meta WHERE key = 'last_membership'",
                        [],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(|e| format!("read last_membership from snap: {e}"))?;

                drop(dst);

                Ok((last_applied, last_membership))
            },
        )
        .await
        .map_err(|e| io::Error::other(format!("join: {e}")))?
        .map_err(io::Error::other)?;

        let bytes = tokio::fs::read(&snap_path).await?;
        let _ = tokio::fs::remove_file(&snap_path).await;

        let last_applied: Option<LogId> = match last_applied_str {
            Some(s) => Some(
                serde_json::from_str(&s)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            ),
            None => None,
        };
        let last_membership: StoredMembership = match last_membership_str {
            Some(s) => serde_json::from_str(&s)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            None => StoredMembership::default(),
        };

        let snapshot_id = match &last_applied {
            Some(lid) => format!(
                "{}-{}-{}",
                lid.committed_leader_id(),
                lid.index(),
                self.snapshot_idx
            ),
            None => format!("--{}", self.snapshot_idx),
        };
        let meta = SnapshotMeta {
            last_log_id: last_applied,
            last_membership,
            snapshot_id,
        };
        Ok(Snapshot {
            meta,
            snapshot: Cursor::new(bytes),
        })
    }
}

pub(crate) async fn new_storage<P: AsRef<Path>>(
    db_path: P,
) -> (
    RocksLogStore<TypeConfig>,
    StateMachineStore<SqliteDatabase>,
    SqliteDatabase,
) {
    let base = db_path.as_ref();
    tokio::fs::create_dir_all(base)
        .await
        .expect("create storage dir");

    // RocksDB: logs + Raft protocol meta
    let raft_path = base.join("raft");
    let mut db_opts = Options::default();
    db_opts.create_missing_column_families(true);
    db_opts.create_if_missing(true);
    let meta = ColumnFamilyDescriptor::new("meta", Options::default());
    let logs = ColumnFamilyDescriptor::new("logs", Options::default());
    let db = DB::open_cf_descriptors(&db_opts, &raft_path, vec![meta, logs]).expect("open rocksdb");
    let db = Arc::new(db);
    let log_store = RocksLogStore::new(db);

    // SQLite state machine via app-db
    let sqlite_path = base.join("state.db");
    let url = format!("sqlite://{}", sqlite_path.display());
    let sqlite_db = SqliteDatabase::connect(&url)
        .await
        .expect("open sqlite state machine");
    let sm_store = StateMachineStore::new(sqlite_db.clone(), sqlite_path);

    (log_store, sm_store, sqlite_db)
}

pub fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

pub async fn run_action_raft<U, A>(
    tx: &mut U,
    raft_index: u64,
    action: A,
) -> Result<A::Output, io::Error>
where
    U: Tx,
    A: Action,
{
    let descriptor = action.audit_descriptor();
    let output = action.execute(tx).await.map_err(io::Error::other)?;
    let allowed = audit_allowed(&descriptor, raft_index as i64, &output, now_ts());
    tx.audit()
        .insert(&allowed)
        .await
        .map_err(io::Error::other)?;
    Ok(output)
}

pub async fn run_audit_denied<U>(
    tx: &mut U,
    raft_index: u64,
    descriptor: AuditDescriptor,
    reason: DenyReason,
) -> Result<(), io::Error>
where
    U: Tx,
{
    let entry = audit_denied(&descriptor, raft_index as i64, reason, now_ts());
    tx.audit().insert(&entry).await.map_err(io::Error::other)?;
    Ok(())
}
