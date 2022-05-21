CREATE TABLE IF NOT EXISTS mod_action (
    id integer primary key asc,
    moderator integer not null,
    usr integer not null,
    reason text,
    context text,
    action_type integer not null,
    create_date datetime
);

CREATE TABLE IF NOT EXISTS mute (
    mod_action integer not null unique,
    start_time datetime not null,
    end_time datetime not null,
    active boolean not null,
    FOREIGN KEY(mod_action) REFERENCES mod_action(id)
);

CREATE TABLE IF NOT EXISTS fetch (
    usr integer primary key,
    info text not null,
    create_date datetime
);

CREATE TABLE IF NOT EXISTS blocked_regexes (
    pattern text primary key,
    added_by integer not null
);

CREATE TABLE IF NOT EXISTS tag (
    name text primary key,
    moderator integer not null,
    content text not null,
    official boolean not null,
    create_date datetime
);

CREATE TABLE IF NOT EXISTS highlights (
    word text not null,
    usr integer not null,
    PRIMARY KEY (word, usr)
);
create table if not exists emoji_stats (
    emoji_id integer not null,
    emoji_name text,
    animated integer not null,
    in_text_usage integer not null default 0,
    reaction_usage integer not null default 0,
    PRIMARY KEY(emoji_id)
);
