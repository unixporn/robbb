CREATE TABLE IF NOT EXISTS highlights (
    id integer primary key asc,
    word text not null,
    user integer not null
);
