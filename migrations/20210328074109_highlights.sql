CREATE TABLE IF NOT EXISTS tag (
    name text primary key,
    moderator integer not null,
    content text not null,
    official boolean not null
);
