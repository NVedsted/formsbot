use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::time::Duration;

use poise::serenity_prelude::*;
use poise::SlashArgError;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

pub const LABEL_MAX_LENGTH: usize = 45;
pub const PLACEHOLDER_MAX_LENGTH: usize = 100;
pub const FIELD_RESPONSE_MAX_LENGTH: u16 = 1024;

#[derive(Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct FormId(Uuid);

impl FromStr for FormId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Uuid::try_parse(s).map(FormId)
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

    pub async fn save_form(&self, new_form: &Form) {
        let mut forms = self.forms.write().await;

        if let Some(form) = forms.iter_mut().find(|f| f.id == new_form.id) {
            *form = new_form.clone();
        } else {
            forms.push(new_form.clone())
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

impl Display for SerializableMention {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
    name: String,
    pub style: InputTextStyle,
    placeholder: Option<String>,
    pub min_length: Option<u16>,
    pub max_length: Option<u16>,
    pub required: bool,
    pub inline: bool,
}

impl FormField {
    pub fn new(name: String, style: InputTextStyle) -> Result<Self, ValueTooLong> {
        Self::validate_name(&name)?;

        Ok(Self {
            name,
            style,
            placeholder: None,
            min_length: None,
            max_length: None,
            required: true,
            inline: false,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: String) -> Result<(), ValueTooLong> {
        Self::validate_name(&name)?;

        self.name = name;
        Ok(())
    }

    pub fn placeholder(&self) -> Option<&str> {
        self.placeholder.as_deref()
    }

    pub fn set_placeholder(&mut self, placeholder: Option<String>) -> Result<(), ValueTooLong> {
        if placeholder.as_ref().map(|p| p.len() > PLACEHOLDER_MAX_LENGTH).unwrap_or(false) {
            return Err(ValueTooLong);
        }

        self.placeholder = placeholder;
        Ok(())
    }

    fn input_text<T: Into<String>>(&self, custom_id: T) -> CreateInputText {
        let mut builder = CreateInputText::new(self.style, &self.name, custom_id)
            .min_length(self.min_length.unwrap_or(FIELD_RESPONSE_MAX_LENGTH))
            .max_length(self.min_length.unwrap_or(FIELD_RESPONSE_MAX_LENGTH))
            .required(self.required);

        if let Some(placeholder) = &self.placeholder {
            builder = builder.placeholder(placeholder);
        }

        builder
    }

    pub fn apply_to_embed(&self, embed: CreateEmbed, value: String) -> CreateEmbed {
        embed.field(&self.name, value, self.inline)
    }

    fn validate_name(name: &str) -> Result<(), ValueTooLong> {
        if name.len() > LABEL_MAX_LENGTH {
            Err(ValueTooLong)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Form {
    id: FormId,
    title: String,
    description: Option<String>,
    fields: Vec<FormField>,
    pub destination: ChannelId,
    pub mention: Option<SerializableMention>,
}

pub enum AddFieldError {
    TooManyFields,
    IllegalAddBefore,
}

#[derive(Debug)]
pub struct ValueTooLong;

impl Display for ValueTooLong {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "field too long")
    }
}

impl std::error::Error for ValueTooLong {}

impl Form {
    pub fn new(title: String, destination: ChannelId) -> Result<Self, ValueTooLong> {
        Self::validate_title(&title)?;
        Ok(Self {
            id: FormId(Uuid::new_v4()),
            title,
            description: None,
            fields: vec![],
            destination,
            mention: None,
        })
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn set_title(&mut self, title: String) -> Result<(), ValueTooLong> {
        Self::validate_title(&title)?;
        self.title = title;
        Ok(())
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn set_description(&mut self, description: Option<String>) -> Result<(), ValueTooLong> {
        if let Some(true) = description.as_ref().map(|d| d.len() > 4096) {
            return Err(ValueTooLong);
        }

        self.description = description;
        Ok(())
    }

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
        field: FormField,
        add_before: Option<usize>,
    ) -> Result<(), AddFieldError> {
        if self.fields.len() >= 5 {
            return Err(AddFieldError::TooManyFields);
        }

        if let Some(i) = add_before {
            if i > self.fields.len() {
                return Err(AddFieldError::IllegalAddBefore);
            }

            self.fields.insert(i, field);
        } else {
            self.fields.push(field);
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

    fn validate_title(title: &str) -> Result<(), ValueTooLong> {
        if title.len() > 256 {
            Err(ValueTooLong)
        } else {
            Ok(())
        }
    }
}