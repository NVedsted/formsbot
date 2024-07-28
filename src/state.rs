use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::time::Duration;

use poise::serenity_prelude::*;
use poise::SlashArgError;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct FormId(Uuid);

impl FromStr for FormId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Uuid::try_parse(s).map(|u| FormId(u))
    }
}

impl Display for FormId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Uuid::fmt(&self.0, f)
    }
}

#[async_trait]
impl poise::SlashArgument for FormId {
    async fn extract(_ctx: &Context, _interaction: &CommandInteraction, value: &ResolvedValue<'_>) -> std::result::Result<Self, SlashArgError> {
        let value = match value {
            ResolvedValue::String(str) => str,
            _ => return Err(SlashArgError::new_command_structure_mismatch("expected string")),
        };
        value.parse().map_err(|_| SlashArgError::new_command_structure_mismatch("expected uuid"))
    }

    fn create(builder: CreateCommandOption) -> CreateCommandOption {
        builder.kind(CommandOptionType::String)
    }
}

pub struct State {
    pub connection_manager: redis::aio::ConnectionManager,
    pub forms: RwLock<Vec<Form>>, // TODO: add persistence
}

impl State {
    pub async fn get_form(&self, id: FormId) -> Option<Form> {
        self.forms.read().await.iter().find(|f| f.id == id).cloned()
    }

    pub async fn create_form(&self, title: String, destination: ChannelId, mention: Option<SerializableMention>) {
        self.forms.write().await.push(Form { id: FormId(Uuid::new_v4()), title, fields: vec![], destination, mention });
    }

    pub async fn save_form(&self, id: FormId, new_form: Form) {
        let mut forms = self.forms.write().await;

        if let Some(form) = forms.iter_mut().find(|f| f.id == id) {
            *form = new_form;
        }
    }

    pub async fn delete_form(&self, id: FormId) -> bool {
        let mut forms = self.forms.write().await;

        if let Some(index) = forms.iter().enumerate().find(|(_, f)| f.id == id).map(|(i, _)| i) {
            forms.swap_remove(index);
            true
        } else {
            false
        }
    }

    pub async fn get_form_ids(&self) -> Vec<(FormId, String)> {
        self.forms.read().await.iter().map(|f| (f.id, f.title.clone())).collect()
    }

    pub async fn get_fields(&self, id: FormId) -> Option<Vec<FormField>> {
        Some(self.get_form(id).await?.fields)
    }
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum SerializableMention {
    Role(RoleId),
    User(UserId),
}

impl SerializableMention {
    fn mention(&self) -> Mention {
        match *self {
            SerializableMention::Role(r) => Mention::Role(r),
            SerializableMention::User(u) => Mention::User(u),
        }
    }
}

impl std::fmt::Display for SerializableMention {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.mention(), f)
    }
}

#[async_trait]
impl poise::SlashArgument for SerializableMention {
    async fn extract(_ctx: &Context, _interaction: &CommandInteraction, value: &ResolvedValue<'_>) -> Result<Self, SlashArgError> {
        match value {
            ResolvedValue::Role(r) => Ok(SerializableMention::Role(r.id)),
            ResolvedValue::User(u, _) => Ok(SerializableMention::User(u.id)),
            _ => Err(SlashArgError::new_command_structure_mismatch("expected mentionable")),
        }
    }

    fn create(builder: CreateCommandOption) -> CreateCommandOption {
        builder.kind(CommandOptionType::Mentionable)
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct FormField {
    pub name: String,
    pub style: InputTextStyle,
    pub placeholder: Option<String>,
    pub min_length: Option<u16>,
    pub max_length: Option<u16>,
    pub required: bool,
    pub inline: bool,
}

impl FormField {
    fn input_text<T: Into<String>>(&self, custom_id: T) -> CreateInputText {
        let mut builder = CreateInputText::new(self.style, &self.name, custom_id)
            .required(self.required);

        if let Some(placeholder) = &self.placeholder {
            builder = builder.placeholder(placeholder);
        }

        if let Some(min_length) = self.min_length {
            builder = builder.min_length(min_length);
        }

        if let Some(max_length) = self.max_length {
            builder = builder.max_length(max_length);
        }

        builder
    }

    pub fn apply_to_embed(&self, embed: CreateEmbed, value: String) -> CreateEmbed {
        embed.field(&self.name, value, self.inline)
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Form {
    pub id: FormId,
    pub title: String,
    fields: Vec<FormField>,
    pub destination: ChannelId,
    pub mention: Option<SerializableMention>,
}

pub enum AddFieldError {
    TooManyFields,
    IllegalAddBefore,
}

impl Form {
    pub fn fields(&self) -> &[FormField] {
        &self.fields
    }

    pub fn quick_modal(&self) -> Option<CreateQuickModal> {
        if self.fields.is_empty() {
            return None;
        }

        let builder = CreateQuickModal::new(&self.title)
            .timeout(Duration::from_secs(600));

        Some(self.fields.iter().enumerate()
            .fold(builder, |acc, (i, f)| acc.field(f.input_text(i.to_string()))))
    }

    pub fn add_field(
        &mut self,
        name: String,
        style: InputTextStyle,
        placeholder: Option<String>,
        min_length: Option<u16>,
        max_length: Option<u16>,
        required: Option<bool>,
        inline: Option<bool>,
        add_before: Option<usize>,
    ) -> Result<(), AddFieldError> {
        if self.fields.len() >= 5 {
            return Err(AddFieldError::TooManyFields);
        }

        let new_field = FormField {
            name,
            style,
            placeholder,
            min_length,
            max_length,
            required: required.unwrap_or(true),
            inline: inline.unwrap_or(false),
        };

        if let Some(i) = add_before {
            if i > self.fields.len() {
                return Err(AddFieldError::IllegalAddBefore);
            }

            self.fields.insert(i, new_field);
        } else {
            self.fields.push(new_field);
        }

        Ok(())
    }

    pub fn remove_field(&mut self, index: usize) -> bool {
        if index < self.fields.len() {
            self.fields.remove(index);
            true
        } else {
            false
        }
    }
}