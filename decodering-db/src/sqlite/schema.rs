pub const SCHEMA: &str = r#"
    -- State machine bookkeeping. Updated atomically with every mutation.
    CREATE TABLE IF NOT EXISTS meta (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS users (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        username TEXT NOT NULL UNIQUE,
        email TEXT NOT NULL UNIQUE,
        password_hash TEXT NOT NULL,
        is_admin INTEGER NOT NULL DEFAULT 0 CHECK (is_admin IN (0, 1)),
        created_at INTEGER NOT NULL
    );

    CREATE TABLE IF NOT EXISTS api_keys (
        id            INTEGER PRIMARY KEY AUTOINCREMENT,
        user_id       INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        key_hash      TEXT NOT NULL UNIQUE,        -- sha256 hex of the raw key
        key_prefix    TEXT NOT NULL,               -- first 8 chars of raw key, for UX
        created_at    INTEGER NOT NULL,
        expires_at    INTEGER,
        revoked_at    INTEGER,
        last_used_at  INTEGER
    );

    CREATE INDEX IF NOT EXISTS idx_api_key_user_id ON api_keys(user_id);

    CREATE TABLE IF NOT EXISTS shamir_configuration (
        id                INTEGER PRIMARY KEY AUTOINCREMENT,
        total_shares      INTEGER NOT NULL,
        threshold         INTEGER NOT NULL,
        validation_hash   BLOB NOT NULL,
        created_at        INTEGER NOT NULL
    );

    CREATE TABLE IF NOT EXISTS applications (
        app_id TEXT PRIMARY KEY,
        app_name TEXT NOT NULL UNIQUE,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    );

    CREATE TABLE IF NOT EXISTS secret_backend_mapping (
        app_id TEXT NOT NULL REFERENCES applications(app_id),
        secret_name TEXT NOT NULL,
        backend TEXT NOT NULL,
        mount_path TEXT NOT NULL,
        tainted INTEGER NOT NULL DEFAULT 0 CHECK (tainted IN (0, 1)),
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL,
        PRIMARY KEY (app_id, secret_name)
    );

    CREATE TABLE IF NOT EXISTS principals (
        principal_id TEXT PRIMARY KEY,
        name         TEXT NOT NULL,
        kind         TEXT NOT NULL CHECK (kind IN ('human', 'machine', 'service')),
        status TEXT NOT NULL DEFAULT 'active',
        created_at   INTEGER NOT NULL,
        updated_at   INTEGER NOT NULL,
        deleted_at   INTEGER
    );

    CREATE TABLE IF NOT EXISTS principal_app_grants (
        principal_id  TEXT NOT NULL REFERENCES principals(principal_id) ON DELETE CASCADE,
        app_id        TEXT NOT NULL REFERENCES applications(app_id) ON DELETE CASCADE,
        granted_at    INTEGER NOT NULL,
        granted_by    INTEGER REFERENCES users(id) ON DELETE SET NULL,
        revoked_at    INTEGER,
        revoked_by    INTEGER REFERENCES users(id) ON DELETE SET NULL,
        PRIMARY KEY (principal_id, app_id)
    );

    -- Each row is one way a principal can authenticate.
    -- Supports vTPM (EK hash), AWS IAM (role ARN), and future identity types
    -- without schema changes.
    CREATE TABLE IF NOT EXISTS principal_credentials (
        credential_id   TEXT PRIMARY KEY,
        principal_id    TEXT NOT NULL REFERENCES principals(principal_id) ON DELETE CASCADE,
        kind            TEXT NOT NULL,           -- 'vtpm' | 'aws_iam' | 'api_key' | ...
        -- The lookup key: what the auth flow searches by.
        --   api_key   -> hash of the key (NEVER store the raw key)
        --   vtpm      -> hex(sha256(EK public key))
        --   aws_iam   -> the role ARN
        lookup_key      TEXT NOT NULL,           -- EK hash, role ARN, hashed API key, etc.
        -- The verification material:
        --   api_key   -> {} (lookup_key holds the hash; no further data needed)
        --   vtpm      -> {"ek_pubkey_pem": "...", "ak_cert_required": true}
        --   aws_iam   -> {"account_id": "123456789", "role_name": "payments-service"}
        secret_material TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(secret_material)),
        status          TEXT NOT NULL DEFAULT 'active',
        expires_at      INTEGER,
        last_used_at    INTEGER,
        created_at      INTEGER NOT NULL,
        revoked_at      INTEGER,
        UNIQUE (kind, lookup_key)
    );

    -- Short-lived tokens issued after a principal authenticates.
    -- A principal can have many active tokens.
    CREATE TABLE IF NOT EXISTS principal_tokens (
        token_id      TEXT PRIMARY KEY,
        token_hash    TEXT NOT NULL UNIQUE,
        principal_id  TEXT NOT NULL REFERENCES principals(principal_id) ON DELETE CASCADE,
        credential_id TEXT REFERENCES principal_credentials(credential_id) ON DELETE SET NULL,
        issued_at     INTEGER NOT NULL,
        expires_at    INTEGER NOT NULL,
        revoked_at    INTEGER
    );

    CREATE INDEX IF NOT EXISTS idx_principal_tokens_principal_id
        ON principal_tokens(principal_id);
    CREATE INDEX IF NOT EXISTS idx_principal_tokens_expires_at
        ON principal_tokens(expires_at);

    -- Unified audit log. Every mutation produces one row.
    -- before_state and after_state are JSON blobs whose shape depends on action_type.
    -- undone_by points to the audit row that undid this one (NULL if not undone).
    -- undoes points back to the audit row this one undid (NULL if this isn't an undo).
    CREATE TABLE IF NOT EXISTS audit_log (
        id                  INTEGER PRIMARY KEY AUTOINCREMENT,
        raft_index          INTEGER NOT NULL,
        timestamp           INTEGER NOT NULL,

        -- Actor: exactly one of these is set.
        user_id             INTEGER REFERENCES users(id),
        principal_id        TEXT,  -- UUID as TEXT in sqlite; FK to principals(principal_id)

        action_type         TEXT NOT NULL,
        target_type         TEXT,
        target_id           TEXT,

        outcome             TEXT NOT NULL CHECK (outcome IN ('allowed', 'denied', 'error')),
        reason              TEXT,

        before_state        TEXT,  -- JSON
        after_state         TEXT,  -- JSON
        metadata            TEXT,  -- JSON

        revertible          INTEGER NOT NULL DEFAULT 1,  -- sqlite boolean
        undone_by           INTEGER REFERENCES audit_log(id),
        undoes              INTEGER REFERENCES audit_log(id),

        CHECK (NOT (user_id IS NOT NULL AND principal_id IS NOT NULL))
    );

    CREATE INDEX IF NOT EXISTS audit_log_target_idx ON audit_log (target_type, target_id);
    CREATE INDEX IF NOT EXISTS audit_log_active_idx ON audit_log (id) WHERE undone_by IS NULL;
    CREATE INDEX IF NOT EXISTS audit_log_raft_idx ON audit_log (raft_index);

    CREATE TABLE IF NOT EXISTS tpm_challenges (
        challenge_id    TEXT PRIMARY KEY,
        nonce           BLOB NOT NULL,           -- 32 random bytes
        ek_pubkey_hash  TEXT,                    -- optional: which credential we expect
        issued_at       INTEGER NOT NULL,
        expires_at      INTEGER NOT NULL,
        consumed_at     INTEGER                  -- single-use flag
    );

    CREATE INDEX IF NOT EXISTS idx_tpm_challenges_expires ON tpm_challenges(expires_at);

    CREATE TABLE IF NOT EXISTS plugin_configs (
        backend_name TEXT PRIMARY KEY,
        secret_blob  BLOB,           -- master key encrypts credential
        updated_at   INTEGER NOT NULL
    );

    -- Enable foreign key enforcement (SQLite default is OFF, must be set per connection).
    PRAGMA foreign_keys = ON;
"#;
