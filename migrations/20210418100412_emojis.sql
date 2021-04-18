create table if not exists emojis (
    emoji_id integer not null,
	emoji_name	text,
	animated integer not null,
	in_text_usage	integer not null default 0,
	reaction_usage	integer not null default 0,
	primary key("emoji_id")
);

