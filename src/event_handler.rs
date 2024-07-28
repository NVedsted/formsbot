use poise::serenity_prelude::*;

use crate::{Error, FrameworkContext};
use crate::responses::create_response;

pub const CUSTOM_ID_PREFIX: &str = "show_form:";

pub async fn event_handler(ctx: &Context, event: &FullEvent, framework: FrameworkContext<'_>) -> Result<(), Error> {
    if let FullEvent::InteractionCreate { interaction: Interaction::Component(interaction) } = event {
        // TODO implement cooldown
        let custom_id = &interaction.data.custom_id;
        if !custom_id.starts_with(CUSTOM_ID_PREFIX) {
            return Ok(());
        }

        let form_id = custom_id[CUSTOM_ID_PREFIX.len()..].parse()?;
        let Some(form) = framework.user_data.get_form(form_id).await else {
            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().ephemeral(true).content("This form no longer exists"))).await?;
            return Ok(());
        };
        let Some(quick_modal) = form.quick_modal() else {
            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().ephemeral(true).content("This form is not correctly configured"))).await?;
            return Ok(());
        };

        let Some(response) = interaction.quick_modal(ctx, quick_modal).await? else {
            return Ok(());
        };

        create_response(ctx, &form, response).await?;
    }

    Ok(())
}