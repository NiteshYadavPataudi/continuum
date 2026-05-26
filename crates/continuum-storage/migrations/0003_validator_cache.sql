CREATE TABLE IF NOT EXISTS validator_cache (
    cache_key TEXT PRIMARY KEY NOT NULL,
    stage TEXT NOT NULL,
    result TEXT NOT NULL,
    passed INTEGER NOT NULL DEFAULT 0,
    duration_ms INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_validator_cache_expires
    ON validator_cache(expires_at);

CREATE TABLE IF NOT EXISTS audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,
    agent TEXT NOT NULL,
    task_id TEXT,
    detail TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_audit_log_event_type
    ON audit_log(event_type);

CREATE INDEX IF NOT EXISTS idx_audit_log_created_at
    ON audit_log(created_at);
