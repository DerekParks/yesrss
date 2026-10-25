CREATE TABLE users (
    id            INTEGER PRIMARY KEY,
    username      TEXT    NOT NULL UNIQUE COLLATE NOCASE,
    password_hash TEXT    NOT NULL,
    created_at    INTEGER NOT NULL
);

CREATE TABLE sessions (
    token_hash TEXT    PRIMARY KEY,
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind       TEXT    NOT NULL CHECK (kind IN ('web', 'api')),
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL
);
CREATE INDEX sessions_user ON sessions(user_id);

CREATE TABLE folders (
    id      INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name    TEXT    NOT NULL,
    UNIQUE (user_id, name)
);

CREATE TABLE feeds (
    id              INTEGER PRIMARY KEY,
    user_id         INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    folder_id       INTEGER REFERENCES folders(id) ON DELETE SET NULL,
    url             TEXT    NOT NULL,
    site_url        TEXT,
    title           TEXT    NOT NULL,
    user_title      TEXT,
    etag            TEXT,
    last_modified   TEXT,
    last_fetched_at INTEGER,
    last_success_at INTEGER,
    next_fetch_at   INTEGER NOT NULL DEFAULT 0,
    error_count     INTEGER NOT NULL DEFAULT 0,
    last_error      TEXT,
    created_at      INTEGER NOT NULL,
    UNIQUE (user_id, url)
);
CREATE INDEX feeds_next_fetch ON feeds(next_fetch_at);
CREATE INDEX feeds_user ON feeds(user_id);

CREATE TABLE favicons (
    feed_id      INTEGER PRIMARY KEY REFERENCES feeds(id) ON DELETE CASCADE,
    content_type TEXT,
    data         BLOB,
    fetched_at   INTEGER NOT NULL
);

CREATE TABLE entries (
    id             INTEGER PRIMARY KEY,
    feed_id        INTEGER NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
    user_id        INTEGER NOT NULL,
    guid           TEXT    NOT NULL,
    url            TEXT,
    title          TEXT    NOT NULL,
    author         TEXT,
    content        TEXT    NOT NULL,
    snippet        TEXT    NOT NULL,
    enclosure_url  TEXT,
    enclosure_type TEXT,
    content_hash   TEXT    NOT NULL,
    published_at   INTEGER NOT NULL,
    created_at     INTEGER NOT NULL,
    last_seen_at   INTEGER NOT NULL,
    is_read        INTEGER NOT NULL DEFAULT 0,
    read_at        INTEGER,
    is_starred     INTEGER NOT NULL DEFAULT 0,
    starred_at     INTEGER,
    UNIQUE (feed_id, guid)
);
CREATE INDEX entries_user_pub ON entries(user_id, published_at DESC, id DESC);
CREATE INDEX entries_feed_pub ON entries(feed_id, published_at DESC, id DESC);
CREATE INDEX entries_user_unread ON entries(user_id, published_at DESC, id DESC) WHERE is_read = 0;
CREATE INDEX entries_feed_unread ON entries(feed_id) WHERE is_read = 0;
CREATE INDEX entries_user_starred ON entries(user_id, published_at DESC, id DESC) WHERE is_starred = 1;

CREATE VIRTUAL TABLE entries_fts USING fts5(
    title,
    body,
    tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TRIGGER entries_fts_delete AFTER DELETE ON entries BEGIN
    DELETE FROM entries_fts WHERE rowid = old.id;
END;
