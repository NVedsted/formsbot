use serenity::all::InputTextStyle;

use crate::{ApplicationContext, Context, Error};
use crate::errors::UserFriendlyError;
use crate::state::{AddFieldError, FormField, FormRef};

use super::autocomplete::{autocomplete_field, autocomplete_form};
use super::get_form;

/// Manages the fields of forms
#[poise::command(
    slash_command,
    ephemeral,
    subcommands("add", "remove", "rename", "style", "placeholder", "validation", "inline", "move_field"
    )
)]
pub async fn fields(_ctx: Context<'_>) -> serenity::Result<(), Error> {
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
#[poise::command(slash_command, ephemeral)]
async fn add(
    ctx: ApplicationContext<'_>,
    #[description = "The form to consider"]
    #[rename = "form"]
    #[autocomplete = "autocomplete_form"]
    form_ref: FormRef,
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
    #[description = "Whether to inline the field when printing responses (defaults to false)"] inline: Option<bool>,
) -> serenity::Result<(), Error> {
    let mut form = get_form(ctx, form_ref).await?;
    let mut field = FormField::new(name, style.into())?;
    field.min_length = min_length;
    field.max_length = max_length;
    field.required = required.unwrap_or(true);
    field.inline = inline.unwrap_or(false);
    field.set_placeholder(placeholder)?;

    match form.add_field(field, add_before) {
        Ok(_) => {
            ctx.data.save_form(ctx.guild_id().unwrap(), &form).await?;
            ctx.say("Field was added").await?
        }
        Err(AddFieldError::IllegalAddBefore) => ctx.say("`add_before` is not valid").await?,
        Err(AddFieldError::TooManyFields) => ctx.say("The maximum amount of fields has been reached").await?,
    };

    Ok(())
}

/// Removes a field from a form
#[poise::command(slash_command, ephemeral)]
async fn remove(
    ctx: ApplicationContext<'_>,
    #[description = "The form to consider"]
    #[rename = "form"]
    #[autocomplete = "autocomplete_form"]
    form_ref: FormRef,
    #[description = "The field to remove"]
    #[autocomplete = "autocomplete_field"]
    field: usize,
) -> serenity::Result<(), Error> {
    let mut form = get_form(ctx, form_ref).await?;
    if form.remove_field(field) {
        ctx.say("Field was removed").await?;
        ctx.data.save_form(ctx.guild_id().unwrap(), &form).await?;
    } else {
        ctx.say("Unknown field").await?;
    }

    Ok(())
}

async fn update_field<F: FnOnce(&mut FormField) -> Result<(), Error>>(
    ctx: ApplicationContext<'_>,
    form_ref: FormRef,
    field: usize,
    updater: F,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let mut form = get_form(ctx, form_ref).await?;
    let field = form.fields_mut().get_mut(field)
        .ok_or_else(|| UserFriendlyError::new("Field could not be found"))?;
    updater(field)?;
    ctx.data.save_form(form_ref.guild_id, &form).await?;
    ctx.say("Field updated").await?;
    Ok(())
}

/// Renames a field
#[poise::command(slash_command, ephemeral)]
async fn rename(
    ctx: ApplicationContext<'_>,
    #[description = "The form to consider"]
    #[rename = "form"]
    #[autocomplete = "autocomplete_form"]
    form_ref: FormRef,
    #[description = "The field to update"]
    #[autocomplete = "autocomplete_field"]
    field: usize,
    #[description = "The new name of the field"]
    #[max_length = 45]
    name: String,
) -> serenity::Result<(), Error> {
    update_field(ctx, form_ref, field, |field| {
        field.set_name(name)?;
        Ok(())
    }).await
}

/// Updates style of a field
#[poise::command(slash_command, ephemeral)]
async fn style(
    ctx: ApplicationContext<'_>,
    #[description = "The form to consider"]
    #[rename = "form"]
    #[autocomplete = "autocomplete_form"]
    form_ref: FormRef,
    #[description = "The field to update"]
    #[autocomplete = "autocomplete_field"]
    field: usize,
    #[description = "The new style of the field"] style: FieldStyle,
) -> serenity::Result<(), Error> {
    update_field(ctx, form_ref, field, |field| {
        field.style = style.into();
        Ok(())
    }).await
}

/// Updates placeholder of a field
#[poise::command(slash_command, ephemeral)]
async fn placeholder(
    ctx: ApplicationContext<'_>,
    #[description = "The form to consider"]
    #[rename = "form"]
    #[autocomplete = "autocomplete_form"]
    form_ref: FormRef,
    #[description = "The field to update"]
    #[autocomplete = "autocomplete_field"]
    field: usize,
    #[description = "New placeholder text for the field (leave it out to remove)"]
    #[max_length = 100]
    placeholder: Option<String>,
) -> serenity::Result<(), Error> {
    update_field(ctx, form_ref, field, |field| {
        field.set_placeholder(placeholder)?;
        Ok(())
    }).await
}

/// Updates validation of a field
#[poise::command(slash_command, ephemeral)]
async fn validation(
    ctx: ApplicationContext<'_>,
    #[description = "The form to consider"]
    #[rename = "form"]
    #[autocomplete = "autocomplete_form"]
    form_ref: FormRef,
    #[description = "The field to update"]
    #[autocomplete = "autocomplete_field"]
    field: usize,
    #[description = "The new minimum length of responses (always at least 1 if required)"]
    #[max = 1024]
    min_length: Option<u16>,
    #[description = "The new maximum length of responses"]
    #[min = 1]
    #[max = 1024]
    max_length: Option<u16>,
    #[description = "Whether the field is required (defaults to true)"] required: Option<bool>,
) -> serenity::Result<(), Error> {
    update_field(ctx, form_ref, field, |field| {
        field.min_length = min_length;
        field.max_length = max_length;
        field.required = required.unwrap_or(true);
        Ok(())
    }).await
}

/// Updates whether to inline responses to this field
#[poise::command(slash_command, ephemeral)]
async fn inline(
    ctx: ApplicationContext<'_>,
    #[description = "The form to consider"]
    #[rename = "form"]
    #[autocomplete = "autocomplete_form"]
    form_ref: FormRef,
    #[description = "The field to update"]
    #[autocomplete = "autocomplete_field"]
    field: usize,
    #[description = "Whether to inline the field when printing responses"] inline: bool,
) -> serenity::Result<(), Error> {
    update_field(ctx, form_ref, field, |field| {
        field.inline = inline;
        Ok(())
    }).await
}

/// Moves field
#[poise::command(slash_command, rename = "move", ephemeral)]
async fn move_field(
    ctx: ApplicationContext<'_>,
    #[description = "The form to consider"]
    #[rename = "form"]
    #[autocomplete = "autocomplete_form"]
    form_ref: FormRef,
    #[description = "The field to update"]
    #[autocomplete = "autocomplete_field"]
    field: usize,
    #[description = "The new position for this field"]
    #[min = 1]
    #[max = 5]
    position: usize,
) -> serenity::Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let mut form = get_form(ctx, form_ref).await?;
    match form.move_field(field, position - 1) {
        Ok(true) => {
            ctx.data.save_form(form_ref.guild_id, &form).await?;
            ctx.say("Field moved").await?;
        }
        Ok(false) => { ctx.say("Unknown field").await?; }
        Err(AddFieldError::IllegalAddBefore) => {
            ctx.say(format!("The form has {0} fields thus position must be between 1 and {0}", form.fields().len())).await?;
        }
        Err(e) => { return Err(e.into()); }
    }

    Ok(())
}
