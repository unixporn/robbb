CREATE TABLE IF NOT EXISTS highlights (
    word text not null,
    usr integer not null,
    PRIMARY KEY (word, usr)
);
