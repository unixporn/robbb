use crate::db::note::NoteType;

use super::*;

/// Write a note about a user.
#[command]
#[usage("note <user> <content>")]
pub async fn note(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let config = data.get::<Config>().unwrap().clone();
    let db = data.get::<Db>().unwrap().clone();

    let guild = msg.guild(&ctx).await.context("Failed to load guild")?;

    let mentioned_user_id = {
        let user_mention = args
            .single::<String>()
            .invalid_usage(&NOTE_COMMAND_OPTIONS)?;
        disambiguate_user_mention(&ctx, &guild, msg, &user_mention)
            .await?
            .ok_or(UserErr::MentionedUserNotFound)?
    };

    let note_content = args.remains().invalid_usage(&NOTE_COMMAND_OPTIONS)?;

    db.add_note(
        msg.author.id,
        mentioned_user_id,
        note_content.to_string(),
        Utc::now(),
        NoteType::ManualNote,
    )
    .await?;

    config
        .log_bot_action(&ctx, |e| {
            e.title("Note");
            e.description(format!(
                "{} took a note about {}",
                msg.author.id.mention(),
                mentioned_user_id.mention()
            ));
            e.field("Note", note_content, false);
        })
        .await;
    msg.reply(&ctx, "Noted!").await?;
    Ok(())
}

/// Read notes about a user.
#[command]
#[usage("notes <user> [all|mod|warn|mute|blocklist]")]
pub async fn notes(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let db = data.get::<Db>().unwrap().clone();

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
        _ => abort_with!(UserErr::invalid_usage(&NOTE_COMMAND_OPTIONS)),
    };

    let avatar_url = mentioned_user_id
        .to_user(&ctx)
        .await
        .map(|user| user.face())
        .ok();

    let notes = db.get_notes(mentioned_user_id, note_filter).await?;
    msg.reply_embed(&ctx, |e| {
        e.title("Notes");
        e.description(format!("Notes about {}", mentioned_user_id.mention()));
        e.author(|a| {
            avatar_url.map(|url| a.icon_url(url));
            a
        });
        for note in notes {
            e.field(
                format!(
                    "{} - {}",
                    note.note_type,
                    util::format_date_ago(note.create_date)
                ),
                format!("{} - {}", note.content, note.moderator.mention(),),
                false,
            );
        }
    })
    .await?;

    Ok(())
}
