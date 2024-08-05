use poise::{ChoiceParameter, CreateReply};
use poise::serenity_prelude::*;

use crate::{ApplicationContext, Error};
use crate::errors::UserFriendlyError;
use crate::responses::create_response;
use crate::state::{Form, FormField, FormId, FormRef, SerializableMention};

use super::{CUSTOM_ID_PREFIX, get_form, parse_cooldown};
use super::autocomplete::autocomplete_form;

/// Creates a new form
#[poise::command(slash_command, rename = "create", ephemeral)]
pub async fn create_form(
    ctx: ApplicationContext<'_>,
    #[description = "The title of the form"]
    #[max_length = 45]
    title: String,
    #[description = "The text shown in top of responses after the form is submitted"]
    #[max_length = 4096]
    description: Option<String>,
    #[description = "The channel to create the thread under"]
    #[channel_types("Text")]
    destination: GuildChannel,
    #[description = "New role/user to be mentioned on submission"]
    mention: Option<SerializableMention>,
    #[description = "How long users must wait between submitting (e.g. `15days 2min 2s`)"]
    cooldown: Option<String>,
) -> serenity::Result<(), Error> {
    ctx.defer_ephemeral().await?;

    validate_destination(ctx, &destination)?;

    let mut form = Form::new(title, destination)?;
    form.mention = mention;
    form.set_description(description)?;
    form.set_cooldown(cooldown.map(parse_cooldown).transpose()?);

    ctx.data.save_form(ctx.guild_id().unwrap(), &form).await?;
    ctx.say("Form was created").await?;

    Ok(())
}

/// Deletes a form
#[poise::command(slash_command, rename = "delete", ephemeral)]
pub async fn delete_form(
    ctx: ApplicationContext<'_>,
    #[description = "The form to delete"]
    #[rename = "form"]
    #[autocomplete = "autocomplete_form"]
    form_id: FormId,
) -> serenity::Result<(), Error> {
    if ctx.data.delete_form(ctx.guild_id().unwrap(), form_id).await? {
        ctx.say("Form was deleted").await?;
    } else {
        ctx.say("Unknown form").await?;
    }

    Ok(())
}

/// Changes the destination channel of a form
#[poise::command(slash_command, ephemeral)]
pub async fn rename(
    ctx: ApplicationContext<'_>,
    #[description = "The form to modify"]
    #[rename = "form"]
    #[autocomplete = "autocomplete_form"]
    form_ref: FormRef,
    #[description = "New title for the form"]
    #[max_length = 45]
    title: String,
) -> serenity::Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let mut form = get_form(ctx, form_ref).await?;
    form.set_title(title)?;
    ctx.data.save_form(ctx.guild_id().unwrap(), &form).await?;
    ctx.say("Form was renamed").await?;
    Ok(())
}

/// Changes the description of a form
#[poise::command(slash_command, ephemeral)]
pub async fn description(
    ctx: ApplicationContext<'_>,
    #[description = "The form to modify"]
    #[rename = "form"]
    #[autocomplete = "autocomplete_form"]
    form_ref: FormRef,
    #[description = "The new text to be shown shown in top of responses (leave it out to clear)"]
    #[max_length = 4096]
    description: Option<String>,
) -> serenity::Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let mut form = get_form(ctx, form_ref).await?;
    form.set_description(description)?;
    ctx.data.save_form(ctx.guild_id().unwrap(), &form).await?;
    ctx.say("Form description was changed").await?;
    Ok(())
}

/// Changes the cooldown of a form
#[poise::command(slash_command, ephemeral)]
pub async fn cooldown(
    ctx: ApplicationContext<'_>,
    #[description = "The form to modify"]
    #[rename = "form"]
    #[autocomplete = "autocomplete_form"]
    form_ref: FormRef,
    #[description = "The new duration users must wait between submissions (leave it out to clear)"]
    cooldown: Option<String>,
) -> serenity::Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let mut form = get_form(ctx, form_ref).await?;
    form.set_cooldown(cooldown.map(parse_cooldown).transpose()?);
    ctx.data.save_form(ctx.guild_id().unwrap(), &form).await?;
    ctx.say("Form cooldown was changed").await?;
    Ok(())
}

/// Changes who is mentioned on submission of the form
#[poise::command(slash_command, ephemeral)]
pub async fn mention(
    ctx: ApplicationContext<'_>,
    #[description = "The form to modify"]
    #[rename = "form"]
    #[autocomplete = "autocomplete_form"]
    form_ref: FormRef,
    #[description = "New role/user to be mentioned on submission (leave it out to remove)"]
    mention: Option<SerializableMention>,
) -> serenity::Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let mut form = get_form(ctx, form_ref).await?;
    form.mention = mention;
    ctx.data.save_form(ctx.guild_id().unwrap(), &form).await?;
    ctx.say("Mention of the form was changed").await?;
    Ok(())
}

fn validate_destination(ctx: ApplicationContext<'_>, destination: &GuildChannel) -> serenity::Result<(), Error> {
    if destination.permissions_for_user(ctx, ctx.framework.bot_id)?.create_private_threads() {
        Ok(())
    } else {
        Err(UserFriendlyError::new(format!("I do not have permission to create private threads in {}", destination)).into())
    }
}

/// Changes the destination channel of a form
#[poise::command(slash_command, ephemeral)]
pub async fn destination(
    ctx: ApplicationContext<'_>,
    #[description = "The form to modify"]
    #[rename = "form"]
    #[autocomplete = "autocomplete_form"]
    form_ref: FormRef,
    #[description = "The new channel to create the thread under"]
    #[channel_types("Text")]
    destination: GuildChannel,
) -> serenity::Result<(), Error> {
    let mut form = get_form(ctx, form_ref).await?;

    validate_destination(ctx, &destination)?;

    form.destination = destination.id;
    ctx.data.save_form(ctx.guild_id().unwrap(), &form).await?;
    ctx.say("Form destination was updated").await?;
    Ok(())
}

#[derive(ChoiceParameter)]
enum ButtonColor {
    Blurple,
    Grey,
    Green,
    Red,
}

impl From<ButtonColor> for ButtonStyle {
    fn from(value: ButtonColor) -> Self {
        match value {
            ButtonColor::Blurple => ButtonStyle::Primary,
            ButtonColor::Grey => ButtonStyle::Secondary,
            ButtonColor::Green => ButtonStyle::Success,
            ButtonColor::Red => ButtonStyle::Danger,
        }
    }
}

/// Create a button for a form
#[poise::command(slash_command, ephemeral)]
pub async fn button(
    ctx: ApplicationContext<'_>,
    #[description = "The form to delete"]
    #[rename = "form"]
    #[autocomplete = "autocomplete_form"]
    form_id: FormId,
    #[description = "Text for the button"]
    #[max_length = 80]
    text: String,
    #[description = "A string to send with the button"] message: Option<String>,
    #[description = "The color of the button"] color: ButtonColor,
    #[description = "An emoji for the button"] emoji: Option<String>,
) -> serenity::Result<(), Error> {
    ctx.defer_ephemeral().await?;

    let mut button = CreateButton::new(format!("{CUSTOM_ID_PREFIX}{form_id}"))
        .label(text)
        .style(color.into());

    if let Some(emoji) = emoji {
        let Ok(reaction) = ReactionType::try_from(emoji) else {
            ctx.say("Failed to parse the provided emoji").await?;
            return Ok(());
        };

        button = button.emoji(reaction);
    }

    let mut create_message = CreateMessage::new()
        .button(button);

    if let Some(message) = message {
        create_message = create_message.content(message);
    }

    ctx.channel_id().send_message(ctx, create_message).await?;

    ctx.say("Button created").await?;

    Ok(())
}

/// Shows a form
#[poise::command(slash_command, rename = "show", ephemeral)]
pub async fn show_form(
    ctx: ApplicationContext<'_>,
    #[description = "The form to delete"]
    #[rename = "form"]
    #[autocomplete = "autocomplete_form"]
    form_ref: FormRef,
    #[description = "Whether submitting should create a response (defaults to false)"]
    create: Option<bool>,
) -> serenity::Result<(), Error> {
    let form = get_form(ctx, form_ref).await?;
    let Some(quick_modal) = form.quick_modal() else {
        ctx.say("A form must have fields to be shown.").await?;
        return Ok(());
    };

    let Some(response) = ctx.interaction.quick_modal(ctx.serenity_context(), quick_modal).await? else {
        return Ok(());
    };

    if let Some(true) = create {
        create_response(ctx.serenity_context, &form, response).await?;
    } else {
        response.interaction.create_response(ctx, CreateInteractionResponse::Acknowledge).await?;
    }

    Ok(())
}

/// Shows the details of a form
#[poise::command(slash_command, rename = "details", ephemeral)]
pub async fn form_details(
    ctx: ApplicationContext<'_>,
    #[description = "The form to consider"]
    #[rename = "form"]
    #[autocomplete = "autocomplete_form"]
    form_ref: FormRef,
) -> serenity::Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let form = get_form(ctx, form_ref).await?;
    let mut embed_builder = CreateEmbed::new()
        .title(form.title());

    fn style_list<const N: usize>(elements: [(&str, Option<String>); N]) -> String {
        elements.into_iter().filter_map(|(name, value)| value.map(|v| (name, v)))
            .map(|(name, v)| format!("- **{}**: {}", name, v))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn field_details(field: &FormField) -> String {
        style_list([
            ("Style", match field.style {
                InputTextStyle::Short => Some("Short".to_owned()),
                InputTextStyle::Paragraph => Some("Paragraph".to_owned()),
                _ => None,
            }),
            ("Placeholder", field.placeholder().map(str::to_owned)),
            ("Minimum length", field.min_length.map(|l| l.to_string())),
            ("Max length", field.max_length.map(|l| l.to_string())),
            ("Required", Some(field.required.to_string())),
            ("In-line", Some(field.inline.to_string())),
        ])
    }

    embed_builder = form.fields().iter()
        .fold(embed_builder, |acc, f| acc.field(f.name(), field_details(f), true))
        .description(style_list([
            ("Destination", Some(form.destination.mention().to_string())),
            ("Description", form.description().map(str::to_owned)),
            ("Mentions", form.mention.map(|m| m.to_string())),
            ("Cooldown", form.cooldown().map(|c| humantime::format_duration(c).to_string())),
        ]));

    ctx.send(CreateReply::default().embed(embed_builder)).await?;

    Ok(())
}