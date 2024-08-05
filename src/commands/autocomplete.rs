use poise::SlashArgument;
use serenity::all::{AutocompleteChoice, ResolvedOption, ResolvedValue};

use crate::ApplicationContext;
use crate::state::FormRef;

pub async fn autocomplete_form(
    ctx: ApplicationContext<'_>,
    _partial: &str,
) -> Vec<AutocompleteChoice> {
    match ctx.data.get_form_ids(ctx.guild_id().unwrap()).await {
        Ok(form_ids) => {
            form_ids.into_iter().map(|(id, name)| AutocompleteChoice::new(name, id.to_string())).collect()
        }
        Err(e) => {
            tracing::error!("an error occurred fetching auto-complete values for forms: {}", e);
            vec![]
        }
    }
}

fn find_resolved_value<'a>(opts: &'a [ResolvedOption], name: &str) -> Option<&'a ResolvedValue<'a>> {
    for opt in opts {
        match &opt.value {
            ResolvedValue::SubCommand(opts)
            | ResolvedValue::SubCommandGroup(opts) => {
                return find_resolved_value(opts, name);
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
    let value = find_resolved_value(&options, name)?;
    SlashArgument::extract(ctx.serenity_context, ctx.interaction, value).await.ok()
}

pub async fn autocomplete_field(
    ctx: ApplicationContext<'_>,
    _partial: &str,
) -> Vec<AutocompleteChoice> {
    let Some(form_id) = find_value(ctx, "form").await else {
        return vec![];
    };

    match ctx.data.get_fields(FormRef::new(ctx.guild_id().unwrap(), form_id)).await {
        Ok(Some(fields)) => {
            return fields.into_iter().enumerate().map(|(i, f)| AutocompleteChoice::new(f.name(), i)).collect();
        }
        Err(e) => tracing::error!("an error occurred fetching auto-complete values for fields: {}", e),
        _ => {}
    }

    vec![]
}