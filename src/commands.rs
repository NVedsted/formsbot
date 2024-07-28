use poise::{ChoiceParameter, SlashArgument};
use poise::serenity_prelude::*;

use crate::{ApplicationContext, Context, Error};
use crate::event_handler::CUSTOM_ID_PREFIX;
use crate::responses::create_response;
use crate::state::{AddFieldError, Form, FormId, SerializableMention, State};

/// Manage forms in the server
#[poise::command(
    slash_command,
    guild_only,
    ephemeral,
    subcommands("create_form", "delete_form", "button", "fields", "destination", "rename", "mention", "show_form"
    )
)]
async fn form(_ctx: Context<'_>) -> Result<(), Error> {
    panic!("called root command")
}

/// Changes the destination channel of a form
#[poise::command(slash_command, ephemeral)]
async fn rename(
    ctx: ApplicationContext<'_>,
    #[description = "The form to modify"]
    #[rename = "form"]
    #[autocomplete = "autocomplete_form"]
    form_id: FormId,
    #[description = "New title for the form"]
    #[max_length = 45]
    title: String,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let Some(mut form) = ctx.data.get_form(form_id).await else {
        ctx.say("Unknown form").await?;
        return Ok(());
    };

    form.set_title(title)?;
    ctx.data.save_form(&form).await;
    ctx.say("Form was renamed").await?;
    Ok(())
}

/// Changes who is mentioned on submission of the form
#[poise::command(slash_command, ephemeral)]
async fn mention(
    ctx: ApplicationContext<'_>,
    #[description = "The form to modify"]
    #[rename = "form"]
    #[autocomplete = "autocomplete_form"]
    form_id: FormId,
    #[description = "New role/user to be mentioned on submission (leave it out to remove)"]
    mention: Option<SerializableMention>,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let Some(mut form) = ctx.data.get_form(form_id).await else {
        ctx.say("Unknown form").await?;
        return Ok(());
    };

    form.mention = mention;
    ctx.say("Mention of the form was changed").await?;
    Ok(())
}

/// Changes the destination channel of a form
#[poise::command(slash_command, ephemeral)]
async fn destination(
    ctx: ApplicationContext<'_>,
    #[description = "The form to modify"]
    #[rename = "form"]
    #[autocomplete = "autocomplete_form"]
    form_id: FormId,
    #[description = "The new channel to create the thread under"]
    #[channel_types("Text")]
    destination: ChannelId,
) -> Result<(), Error> {
    let Some(mut form) = ctx.data.get_form(form_id).await else {
        ctx.say("Unknown form").await?;
        return Ok(());
    };

    form.destination = destination;
    ctx.data.save_form(&form).await;
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
async fn button(
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
) -> Result<(), Error> {
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
async fn show_form(
    ctx: ApplicationContext<'_>,
    #[description = "The form to delete"]
    #[rename = "form"]
    #[autocomplete = "autocomplete_form"]
    form_id: FormId,
    #[description = "Whether submitting should create a response (defaults to false)"]
    create: Option<bool>,
) -> Result<(), Error> {
    let Some(form) = ctx.data.get_form(form_id).await else {
        ctx.say("Unknown form").await?;
        return Ok(());
    };

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

/// Manages the fields of forms
#[poise::command(slash_command, ephemeral, subcommands("add_field", "remove_field"))]
async fn fields(_ctx: Context<'_>) -> Result<(), Error> {
    panic!("called root command")
}

#[derive(poise::ChoiceParameter)]
enum FieldStyle {
    #[name = "Short (single-line)"]
    Short,
    #[name = "Paragraph (multi-line)"]
    Paragraph,
}

impl From<FieldStyle> for InputTextStyle {
    fn from(value: FieldStyle) -> Self {
        match value {
            FieldStyle::Short => Self::Short,
            FieldStyle::Paragraph => Self::Paragraph,
        }
    }
}

/// Adds a field to a form
#[poise::command(slash_command, rename = "add", ephemeral)]
async fn add_field(
    ctx: ApplicationContext<'_>,
    #[description = "The form to consider"]
    #[rename = "form"]
    #[autocomplete = "autocomplete_form"]
    form_id: FormId,
    #[description = "The name of the field"]
    #[max_length = 45]
    name: String,
    #[description = "The style of the field"] style: FieldStyle,
    #[description = "Placeholder text for the field"]
    #[max_length = 100]
    placeholder: Option<String>,
    #[description = "The minimum length of responses (always at least 1 if required)"]
    #[max = 1024]
    min_length: Option<u16>,
    #[description = "The maximum length of responses"]
    #[min = 1]
    #[max = 1024]
    max_length: Option<u16>,
    #[description = "Whether the field is required (defaults to true)"] required: Option<bool>,
    #[description = "Whether to add this field before another existing field; otherwise, it is added to the bottom"]
    #[autocomplete = "autocomplete_field"]
    add_before: Option<usize>,
    #[description = "Whether to inline the field when printing responses"] inline: Option<bool>,
) -> Result<(), Error> {
    let Some(mut form) = ctx.data.get_form(form_id).await else {
        ctx.say("Unknown form").await?;
        return Ok(());
    };

    match form.add_field(
        name,
        style.into(),
        placeholder,
        min_length,
        max_length,
        required,
        inline,
        add_before,
    ) {
        Ok(_) => {
            ctx.data.save_form(&form).await;
            ctx.say("Field was added").await?
        }
        Err(AddFieldError::IllegalAddBefore) => ctx.say("`add_before` is not valid").await?,
        Err(AddFieldError::TooManyFields) => ctx.say("The maximum amount of fields has been reached").await?,
    };

    Ok(())
}

fn find_resolved_value<'a>(ctx: ApplicationContext, opts: &'a [ResolvedOption], name: &str) -> Option<&'a ResolvedValue<'a>> {
    for opt in opts {
        match &opt.value {
            ResolvedValue::SubCommand(opts)
            | ResolvedValue::SubCommandGroup(opts) => {
                return find_resolved_value(ctx, opts, name);
            }
            v if opt.name == name => {
                return Some(v);
            }
            _ => {}
        }
    }
    None
}

async fn find_value<T: SlashArgument>(ctx: ApplicationContext<'_>, name: &str) -> Option<T> {
    let options = ctx.interaction.data.options();
    let value = find_resolved_value(ctx, &options, name)?;
    SlashArgument::extract(ctx.serenity_context, ctx.interaction, value).await.ok()
}

async fn autocomplete_field(
    ctx: ApplicationContext<'_>,
    _partial: &str,
) -> Vec<AutocompleteChoice> {
    if let Some(form_id) = find_value(ctx, "form").await {
        if let Some(fields) = ctx.data.get_fields(form_id).await {
            fields.into_iter().enumerate().map(|(i, f)| AutocompleteChoice::new(f.name, i)).collect()
        } else {
            vec![]
        }
    } else {
        vec![]
    }
}

/// Removes a field from a form
#[poise::command(slash_command, rename = "remove", ephemeral)]
async fn remove_field(
    ctx: ApplicationContext<'_>,
    #[description = "The form to consider"]
    #[rename = "form"]
    #[autocomplete = "autocomplete_form"]
    form_id: FormId,
    #[description = "The field to remove"]
    #[autocomplete = "autocomplete_field"]
    field: usize,
) -> Result<(), Error> {
    let Some(mut form) = ctx.data.get_form(form_id).await else {
        ctx.say("Unknown form").await?;
        return Ok(());
    };

    if form.remove_field(field) {
        ctx.say("Field was removed").await?;
        ctx.data.save_form(&form).await;
    } else {
        ctx.say("Unknown field").await?;
    }

    Ok(())
}

/// Creates a new form
#[poise::command(slash_command, rename = "create", ephemeral)]
async fn create_form(
    ctx: ApplicationContext<'_>,
    #[description = "The title of the form"]
    #[max_length = 45]
    title: String,
    #[description = "The text shown in top of responses after the form is submitted"]
    #[max_length = 4096]
    description: Option<String>,
    #[description = "The channel to create the thread under"]
    #[channel_types("Text")]
    destination: ChannelId,
    #[description = "New role/user to be mentioned on submission (leave it out to remove)"]
    mention: Option<SerializableMention>,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let mut form = Form::new(title, destination);
    form.mention = mention;
    form.set_description(description)?;
    ctx.data.save_form(&form).await;
    ctx.say("Form was created").await?;

    Ok(())
}

async fn autocomplete_form(
    ctx: ApplicationContext<'_>,
    _partial: &str,
) -> Vec<AutocompleteChoice> {
    ctx.data.get_form_ids().await.into_iter().map(|(id, name)| AutocompleteChoice::new(name, id.to_string())).collect()
}

/// Deletes a form
#[poise::command(slash_command, rename = "delete", ephemeral)]
async fn delete_form(
    ctx: ApplicationContext<'_>,
    #[description = "The form to delete"]
    #[rename = "form"]
    #[autocomplete = "autocomplete_form"]
    form_id: FormId,
) -> Result<(), Error> {
    if ctx.data.delete_form(form_id).await {
        ctx.say("Form was deleted").await?;
    } else {
        ctx.say("Unknown form").await?;
    }

    Ok(())
}

#[poise::command(prefix_command, owners_only)]
async fn register(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

pub fn get_commands() -> Vec<poise::Command<State, Error>> {
    vec![register(), form()]
}
