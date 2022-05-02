use crate::db::note::NoteType;

use super::*;

/// Write a note about a user.
#[command]
#[only_in(guilds)]
#[usage("note <user> <content> | note undo <user>")]
#[sub_commands(undo_note)]
pub async fn note(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let db = ctx.get_db().await;

    let guild = msg.guild(&ctx).await.context("Failed to load guild")?;

    let mentioned_user_id = {
        let user_mention = args
            .single_quoted::<String>()
            .invalid_usage(&NOTE_COMMAND_OPTIONS)?;
        disambiguate_user_mention(&ctx, &guild, msg, &user_mention)
            .await?
            .ok_or(UserErr::MentionedUserNotFound)?
    };

    let mentioned_user = mentioned_user_id.to_user(&ctx).await?;

    let note_content = args.remains().invalid_usage(&NOTE_COMMAND_OPTIONS)?;

    db.add_note(
        msg.author.id,
        mentioned_user_id,
        note_content.to_string(),
        Utc::now(),
        NoteType::ManualNote,
        Some(msg.link()),
    )
    .await?;

    modlog::log_note(&ctx, msg, &mentioned_user, note_content).await;
    msg.reply_success(&ctx, "Noted!").await?;
    Ok(())
}

/// Read notes about a user.
#[command]
#[only_in(guilds)]
#[usage("notes <user> [all|mod|warn|mute|blocklist]")]
pub async fn notes(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let db = ctx.get_db().await;

    let guild = msg.guild(&ctx).await.context("Failed to load guild")?;

    let mentioned_user_id = {
        let user_mention = args
            .single::<String>()
            .invalid_usage(&NOTES_COMMAND_OPTIONS)?;
        disambiguate_user_mention(&ctx, &guild, msg, &user_mention)
            .await?
            .ok_or(UserErr::MentionedUserNotFound)?
    };

    let note_filter = args
        .single::<String>()
        .unwrap_or_else(|_| "all".to_string());
    let note_filter = match note_filter.as_str() {
        "all" => None,
        "mod" => Some(NoteType::ManualNote),
        "blocklist" => Some(NoteType::BlocklistViolation),
        "warn" => Some(NoteType::Warn),
        "mute" => Some(NoteType::Mute),
        "ban" => Some(NoteType::Ban),
        "kick" => Some(NoteType::Kick),
        _ => abort_with!(UserErr::invalid_usage(&NOTE_COMMAND_OPTIONS)),
    };

    let avatar_url = mentioned_user_id
        .to_user(&ctx)
        .await
        .map(|user| user.face())
        .ok();

    let notes = fetch_note_values(&db, mentioned_user_id, note_filter).await?;

    let fields = notes.iter().map(|note| {
        let context_link = note
            .context
            .clone()
            .map(|link| format!(" - [(context)]({})", link))
            .unwrap_or_else(String::new);
        (
            format!("{} - {} ", note.note_type, util::format_date_ago(note.date)),
            format!(
                "{} - {}{}",
                note.description,
                note.moderator.mention(),
                context_link
            ),
        )
    });

    let base_embed = embeds::make_create_embed(ctx, |e| {
        e.title("Notes")
            .description(format!("Notes about {}", mentioned_user_id.mention()))
            .author(|a| {
                avatar_url.map(|url| a.icon_url(url));
                a
            })
    })
    .await;

    embeds::PaginatedEmbed::create_from_fields(fields, base_embed)
        .await
        .reply_to(&ctx, &msg)
        .await?;

    Ok(())
}

/// Remove the most recent note on a user
#[command("undo")]
#[usage("note undo <user>")]
pub async fn undo_note(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = msg.guild(&ctx).await.context("Failed to load guild")?;
    let mentioned_user = &args
        .single_quoted::<String>()
        .invalid_usage(&WARN_COMMAND_OPTIONS)?;
    let mentioned_user_id = disambiguate_user_mention(&ctx, &guild, msg, mentioned_user)
        .await?
        .ok_or(UserErr::MentionedUserNotFound)?;

    let db = ctx.get_db().await;
    db.undo_latest_note(mentioned_user_id).await?;

    msg.reply_success(&ctx, "Successfully removed the note!")
        .await?;

    Ok(())
}

struct NotesEntry {
    note_type: NoteType,
    description: String,
    date: chrono::DateTime<Utc>,
    moderator: UserId,
    context: Option<String>,
}

async fn fetch_note_values(
    db: &Db,
    user_id: UserId,
    filter: Option<NoteType>,
) -> Result<Vec<NotesEntry>> {
    let mut entries = Vec::new();

    if filter.is_none()
        || filter == Some(NoteType::ManualNote)
        || filter == Some(NoteType::BlocklistViolation)
    {
        let notes = db
            .get_notes(user_id, filter)
            .await?
            .into_iter()
            .map(|x| NotesEntry {
                note_type: x.note_type,
                description: x.content,
                moderator: x.moderator,
                date: x.create_date,
                context: x.context,
            });
        entries.extend(notes);
    }
    if filter.is_none() || filter == Some(NoteType::Mute) {
        let mutes = db
            .get_mutes(user_id)
            .await?
            .into_iter()
            .map(|x| NotesEntry {
                note_type: NoteType::Mute,
                description: format!(
                    "[{}] {}",
                    humantime::Duration::from(
                        (x.end_time - x.start_time).to_std().unwrap_or_default()
                    ),
                    x.reason
                ),
                date: x.start_time,
                moderator: x.moderator,
                context: x.context,
            });
        entries.extend(mutes);
    }
    if filter.is_none() || filter == Some(NoteType::Warn) {
        let warns = db
            .get_warns(user_id)
            .await?
            .into_iter()
            .map(|x| NotesEntry {
                note_type: NoteType::Warn,
                description: x.reason,
                date: x.create_date,
                moderator: x.moderator,
                context: x.context,
            });
        entries.extend(warns);
    }
    if filter.is_none() || filter == Some(NoteType::Ban) {
        let bans = db.get_bans(user_id).await?.into_iter().map(|x| NotesEntry {
            note_type: NoteType::Ban,
            description: x.reason,
            date: x.create_date,
            moderator: x.moderator,
            context: x.context,
        });
        entries.extend(bans);
    }
    if filter.is_none() || filter == Some(NoteType::Kick) {
        let kicks = db
            .get_kicks(user_id)
            .await?
            .into_iter()
            .map(|x| NotesEntry {
                note_type: NoteType::Kick,
                description: x.reason,
                date: x.create_date,
                moderator: x.moderator,
                context: x.context,
            });
        entries.extend(kicks);
    }
    entries.sort_by_key(|x| x.date);
    Ok(entries)
}
