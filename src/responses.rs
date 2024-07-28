use serenity::all::ChannelType;
use serenity::builder::{CreateEmbed, CreateEmbedAuthor, CreateMessage, CreateThread, EditInteractionResponse};
use serenity::model::channel::AutoArchiveDuration;
use serenity::model::Timestamp;
use serenity::prelude::*;
use serenity::utils::QuickModalResponse;

use crate::Error;
use crate::state::Form;

pub async fn create_response(ctx: &Context, form: &Form, response: QuickModalResponse) -> Result<(), Error> {
    response.interaction.defer_ephemeral(ctx).await?;

    let member = response.interaction.member.as_ref().expect("can only be run in guild");
    let user_name = member.display_name();

    let create_thread = CreateThread::new(user_name)
        .kind(ChannelType::PrivateThread)
        .auto_archive_duration(AutoArchiveDuration::OneWeek)
        .invitable(false);
    let thread = form.destination.create_thread(ctx, create_thread).await?;

    let mut embed_builder = CreateEmbed::new()
        .title(form.title())
        .timestamp(Timestamp::now())
        .author(CreateEmbedAuthor::new(user_name).icon_url(member.face()));

    embed_builder = form.fields().iter().zip(response.inputs.into_iter())
        .fold(embed_builder, |acc, (field, value)| field.apply_to_embed(acc, value));

    let mut content = None;

    if let Some(mentionable) = form.mention {
        content = Some(mentionable.to_string() + "\n");
    }

    if let Some(description) = form.description() {
        *content.get_or_insert_with(String::new) += description;
    }

    let mut message_builder = CreateMessage::new().embed(embed_builder);

    if let Some(content) = content {
        message_builder = message_builder.content(content.trim_end());
    }

    thread.send_message(ctx, message_builder).await?;
    thread.id.add_thread_member(ctx, response.interaction.user.id).await?;

    response.interaction.edit_response(ctx, EditInteractionResponse::new().content(format!("{thread} has been created"))).await?;

    Ok(())
}