-- Add migration script here
CREATE TABLE IF NOT EXISTS warn (
    id integer primary key asc,
    moderator integer not null,
    usr integer not null,
    reason text,
    create_date datetime
);

CREATE TABLE IF NOT EXISTS mute (
    id integer primary key asc,
    guildid integer not null,
    moderator integer not null,
    usr text integer not null,
    start_time datetime not null,
    end_time datetime not null,
    reason text,
    active boolean not null
);

CREATE TABLE IF NOT EXISTS note (
    id integer primary key asc,
    moderator integer not null,
    usr integer not null,
    content text not null,
    note_type integer not null,
    create_date datetime not null
);

CREATE TABLE IF NOT EXISTS fetch (
    usr integer primary key not null,
    info text not null
);
