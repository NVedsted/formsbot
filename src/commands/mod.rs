#![allow(clippy::too_many_arguments)]

use std::time::Duration;

use poise::serenity_prelude::*;

use cooldowns::cooldowns;
use fields::fields;
use forms::*;

use crate::{ApplicationContext, Context, Error};
use crate::errors::UserFriendlyError;
use crate::event_handler::CUSTOM_ID_PREFIX;
use crate::state::{Form, FormRef, State};

mod cooldowns;
mod forms;
mod fields;
mod autocomplete;

async fn get_form(ctx: ApplicationContext<'_>, form_ref: FormRef) -> Result<Form, Error> {
    ctx.data.get_form(form_ref).await?.ok_or_else(|| UserFriendlyError::new("Form could not be found").into())
}

fn parse_cooldown(cooldown: String) -> Result<Duration, Error> {
    match humantime::parse_duration(&cooldown) {
        Ok(cooldown) => Ok(cooldown),
        Err(e) => Err(UserFriendlyError::new(format!("Cooldown was not formatted correctly: {e}")).into()),
    }
}

/// Manage forms in the server
#[poise::command(
    slash_command,
    guild_only,
    ephemeral,
    default_member_permissions = "MANAGE_CHANNELS",
    subcommands("create_form", "delete_form", "button", "fields", "destination", "rename", "mention", "show_form", "form_details", "description", "cooldown", "cooldowns"
    )
)]
pub async fn forms(_ctx: Context<'_>) -> serenity::Result<(), Error> {
    panic!("called root command")
}

#[poise::command(prefix_command, owners_only)]
async fn register(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

pub fn get_commands() -> Vec<poise::Command<State, Error>> {
    vec![register(), forms()]
}
