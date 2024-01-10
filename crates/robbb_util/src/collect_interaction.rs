use futures::{Stream, StreamExt};
use poise::serenity_prelude::{Context, Message, UserId};
use serenity::all::ComponentInteraction;

pub struct UserSpecificComponentInteractionCollector<T> {
    stream: T,
    by_user_limit: usize,
}

impl<T> UserSpecificComponentInteractionCollector<T>
where
    T: Stream<Item = ComponentInteraction> + Unpin,
{
    pub async fn next(&mut self) -> Option<ComponentInteraction> {
        if self.by_user_limit == 0 {
            return None;
        }
        let interaction = self.stream.next().await?;
        self.by_user_limit -= 1;
        Some(interaction)
    }
}

pub fn await_component_interactions_by(
    ctx: &Context,
    message: &Message,
    user_id: UserId,
    by_user_limit: usize,
    timeout: std::time::Duration,
) -> UserSpecificComponentInteractionCollector<impl Stream<Item = ComponentInteraction>> {
    let stream = message
        .await_component_interaction(ctx)
        .filter(move |x| x.user.id == user_id)
        .timeout(timeout)
        .stream();
    UserSpecificComponentInteractionCollector { by_user_limit, stream }
}
