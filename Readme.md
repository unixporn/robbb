# Trup, but Rust!

**A Discord bot for the Unixporn community**

Now written in a _good_ language!


## Dependencies

- Rust
- sqlx-cli (if you need to change the database schema)

## Set up environment variables

The bot reads the data it needs from environment variables.
To see which values have to be set, check out the provided [.env.example](./.env.example) file.
you can use `export $(cat .env)` to export the variables from a .env file in your current environment.

### Extra information 

Most environment variables are retrieved by right clicking, and copying the ID of the relevant channel, category, role.
You need to have developer mode turned on for that to be possible. 

- TOKEN: The discord bot token, retrieved from: https://discord.com/developers/applications
- GUILD: The ID of the guild, where the host resides
- ROLE_*: IDs of relevant roles, easily copied from Server Settings -> Roles.
- ROLE_COLOR: Unlike other ROLE variables, this is a comma (`,`) separated list, ex.: `ROLES_COLOR=825158129711972372,635627141123538966`
- CHANNEL_*: Channel IDs, based on which the bot performs moderation or responses
- ATTACHMENT_CACHE_*: Location (directory) and size of local message attachments cache (in case they get deleted)


## Database

The bot uses a SQLite database, which does not have to be started externally.
The included sqlite-db file is not the actual database used in production, but just an empty database used for development.
To change and work with the database, use [sqlx-cli](https://github.com/launchbadge/sqlx/tree/master/sqlx-cli) to add migrations and generate a new, updated database file.
