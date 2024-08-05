use poise::serenity_prelude::Mentionable;
use serenity::all::UserId;

use crate::{ApplicationContext, Context, Error};
use crate::state::FormRef;

use super::autocomplete::autocomplete_form;

/// Manage cooldowns
#[poise::command(slash_command, subcommands("clear_cooldown"))]
pub async fn cooldowns(_ctx: Context<'_>) -> serenity::Result<(), Error> {
    panic!("called root command")
}

/// Clear cooldown of user for a form
#[poise::command(slash_command, rename = "clear", ephemeral)]
async fn clear_cooldown(
    ctx: ApplicationContext<'_>,
    #[description = "The form to clear cooldowns for"]
    #[rename = "form"]
    #[autocomplete = "autocomplete_form"]
    form_ref: FormRef,
    #[description = "The user to clear cooldown for"]
    #[rename = "user"]
    user_id: UserId,
) -> serenity::Result<(), Error> {
    ctx.defer_ephemeral().await?;
    if ctx.data.clear_cooldown(form_ref, user_id).await? {
        ctx.say(format!("Cooldown was cleared for {}", user_id.mention())).await?;
    } else {
        ctx.say(format!("{} was not on cooldown for this form", user_id.mention())).await?;
    }
    Ok(())
}