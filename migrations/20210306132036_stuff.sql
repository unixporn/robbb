-- Add migration script here
CREATE TABLE IF NOT EXISTS warn (
    id integer primary key asc,
    moderator integer not null,
    usr integer not null,
    reason text,
    create_date integer
);

CREATE TABLE IF NOT EXISTS mute (
    id integer primary key asc,
    guildid integer not null,
    moderator integer not null,
    usr text integer not null,
    start_time integer not null,
    end_time integer not null,
    reason text,
    active boolean not null
);
