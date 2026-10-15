CREATE TABLE users (
    uid integer PRIMARY KEY,
    username text,
    password_bcrypt text
);

CREATE TABLE tokens (
    token blob PRIMARY KEY,
    uid integer,
    expiry integer default (unixepoch() + (60 * 60 * 24 * 7)),
    FOREIGN KEY(uid) REFERENCES users(uid)
);

CREATE TABLE scores (
    id integer PRIMARY KEY,
    beatmap_hash blob,
    meta_hash blob,
    by_uid integer,
    early_quit integer,
    score integer,
    accuracy real,
    perfect integer,
    okay integer,
    bad integer,
    miss integer,
    FOREIGN KEY(by_uid) REFERENCES users(uid)
);
