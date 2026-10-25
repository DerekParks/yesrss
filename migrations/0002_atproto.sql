CREATE TABLE app_secrets (
    name  TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE oauth_pending (
    state          TEXT    PRIMARY KEY,
    browser_hash   TEXT    NOT NULL,
    user_id        INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    did            TEXT    NOT NULL,
    handle         TEXT    NOT NULL,
    pds_url        TEXT    NOT NULL,
    issuer         TEXT    NOT NULL,
    token_endpoint TEXT    NOT NULL,
    revocation_endpoint TEXT,
    pkce_verifier  TEXT    NOT NULL,
    dpop_key       TEXT    NOT NULL,
    dpop_nonce     TEXT,
    created_at     INTEGER NOT NULL
);

CREATE TABLE atproto_accounts (
    user_id             INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    did                 TEXT    NOT NULL,
    handle              TEXT    NOT NULL,
    pds_url             TEXT    NOT NULL,
    issuer              TEXT    NOT NULL,
    token_endpoint      TEXT    NOT NULL,
    revocation_endpoint TEXT,
    scope               TEXT    NOT NULL,
    access_token        TEXT    NOT NULL,
    refresh_token       TEXT,
    expires_at          INTEGER NOT NULL,
    dpop_key            TEXT    NOT NULL,
    as_nonce            TEXT,
    pds_nonce           TEXT,
    created_at          INTEGER NOT NULL
);

ALTER TABLE entries ADD COLUMN like_uri TEXT;
ALTER TABLE entries ADD COLUMN like_doc_uri TEXT;
