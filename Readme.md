# Trup, but Rust!

**A Discord bot for the Unixporn community**

Now written in a _good_ language!


## Dependencies

- Rust
- sqlx-cli (if you need to change the database schema)

## Set up environment variables

The bot reads the data it needs from environment variables.
To see which values have to be set, check out the provided [.env.example](./.env.example) file.
you can use `export $(.env)` to export the variables from a .env file in your current environment.

## Database

The bot uses a SQLite database, which does not have to be started externally.
The included sqlite-db file is not the actual database used in production, but just an empty database used for development.
To change and work with the database, use [sqlx-cli](https://github.com/launchbadge/sqlx/tree/master/sqlx-cli) to add migrations and generate a new, updated database file.
