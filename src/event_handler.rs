use poise::serenity_prelude::*;

use crate::{Error, FrameworkContext};
use crate::responses::create_response;

pub const CUSTOM_ID_PREFIX: &str = "show_form:";

async fn reply<T: Into<String>>(ctx: &Context, interaction: &ComponentInteraction, message: T) -> Result<(), Error> {
    interaction.create_response(
        ctx,
        CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().ephemeral(true).content(message))
    ).await?;
    Ok(())
}

pub async fn event_handler(ctx: &Context, event: &FullEvent, framework: FrameworkContext<'_>) -> Result<(), Error> {
    if let FullEvent::InteractionCreate { interaction: Interaction::Component(interaction @ ComponentInteraction { guild_id: Some(guild_id), .. }) } = event {
        // TODO implement cooldown
        let custom_id = &interaction.data.custom_id;
        if !custom_id.starts_with(CUSTOM_ID_PREFIX) {
            return Ok(());
        }
        let form_id = custom_id[CUSTOM_ID_PREFIX.len()..].parse()?;

        if let Some(cooldown) = framework.user_data.cooldown(*guild_id, form_id, interaction.user.id).await? {
            reply(ctx, interaction, format!("You have submitted this form recently; please wait {} before trying again", humantime::format_duration(cooldown))).await?;
            return Ok(());
        }

        let Some(form) = framework.user_data.get_form(*guild_id, form_id).await? else {
            reply(ctx, interaction, "This form no longer exists").await?;
            return Ok(());
        };

        if !form.destination.to_channel(ctx).await?.guild().expect("not a guild channel")
            .permissions_for_user(ctx, framework.bot_id)?.create_private_threads() {
            reply(ctx, interaction, "This form is not correctly configured (cannot create threads)").await?;
            return Ok(());
        }

        let Some(quick_modal) = form.quick_modal() else {
            reply(ctx, interaction, "This form is not correctly configured (no fields on form)").await?;
            return Ok(());
        };

        let Some(response) = interaction.quick_modal(ctx, quick_modal).await? else {
            return Ok(());
        };

        create_response(ctx, &form, response).await?;

        framework.user_data.trigger_cooldown(*guild_id, &form, interaction.user.id).await?;
    }

    Ok(())
}