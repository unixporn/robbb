CREATE TABLE IF NOT EXISTS kick (
    id integer primary key asc,
    moderator integer not null,
    usr integer not null,
    reason text,
    create_date datetime,
    context text
);


CREATE TABLE IF NOT EXISTS ban (
    id integer primary key asc,
    moderator integer not null,
    usr integer not null,
    reason text,
    create_date datetime,
    context text
);
