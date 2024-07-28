use poise::serenity_prelude::*;

use crate::{Error, FrameworkContext};

pub const CUSTOM_ID_PREFIX: &str = "show_form:";

pub async fn event_handler(ctx: &Context, event: &FullEvent, framework: FrameworkContext<'_>) -> Result<(), Error> {
    if let FullEvent::InteractionCreate { interaction: Interaction::Component(interaction) } = event {
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

        response.interaction.defer_ephemeral(ctx).await?;

        let member = interaction.member.as_ref().expect("can only be run in guild");
        let user_name = member.display_name();

        let create_thread = CreateThread::new(user_name)
            .kind(ChannelType::PrivateThread)
            .auto_archive_duration(AutoArchiveDuration::OneWeek)
            .invitable(false);
        let thread = form.destination.create_thread(ctx, create_thread).await?;


        let mut embed_builder = CreateEmbed::new()
            .timestamp(Timestamp::now())
            .author(CreateEmbedAuthor::new(user_name).icon_url(member.face()));

        embed_builder = form.fields().iter().zip(response.inputs.into_iter())
            .fold(embed_builder, |acc, (field, value)| field.apply_to_embed(acc, value));

        thread.send_message(ctx, CreateMessage::new().content("This is a nice place").embed(embed_builder)).await?;
        thread.id.add_thread_member(ctx, response.interaction.user.id).await?;

        response.interaction.edit_response(ctx, EditInteractionResponse::new().content(format!("{thread} has been created"))).await?;
    }

    Ok(())
}