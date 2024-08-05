use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::time::Duration;

use poise::serenity_prelude::*;
use poise::SlashArgError;
use redis::{AsyncCommands, FromRedisValue, RedisResult, RedisWrite, SetExpiry, SetOptions, ToRedisArgs, Value};
use serde::{Deserialize, Serialize};
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
        // TODO turn this to invalid if it becomes available
        value.parse().map_err(|_| SlashArgError::new_command_structure_mismatch("expected uuid"))
    }

    fn create(builder: CreateCommandOption) -> CreateCommandOption {
        builder.kind(CommandOptionType::String)
    }
}

#[derive(Copy, Clone)]
pub struct FormRef {
    pub guild_id: GuildId,
    pub form_id: FormId,
}

impl FormRef {
    pub fn new(guild_id: GuildId, form_id: FormId) -> Self {
        FormRef { guild_id, form_id }
    }
}

#[async_trait]
impl poise::SlashArgument for FormRef {
    async fn extract(ctx: &Context, interaction: &CommandInteraction, value: &ResolvedValue<'_>) -> std::result::Result<Self, SlashArgError> {
        let form_id = FormId::extract(ctx, interaction, value).await?;
        match interaction.guild_id {
            Some(guild_id) => Ok(FormRef::new(guild_id, form_id)),
            None => Err(SlashArgError::new_command_structure_mismatch("only possible in guilds")),
        }
    }

    fn create(builder: CreateCommandOption) -> CreateCommandOption {
        FormId::create(builder)
    }
}

#[derive(Clone)]
pub struct State {
    pub connection_manager: redis::aio::ConnectionManager,
}

fn get_forms_key(guild_id: GuildId) -> String {
    format!("forms:{guild_id}")
}

fn get_cooldown_key(FormRef { guild_id, form_id }: FormRef, user_id: UserId) -> String {
    format!("forms:{guild_id}:{form_id}:{user_id}")
}

impl State {
    pub async fn get_form(&self, form_ref: FormRef) -> Result<Option<Form>, crate::Error> {
        Ok(self.connection_manager.clone().hget(get_forms_key(form_ref.guild_id), form_ref.form_id.to_string()).await?)
    }

    pub async fn save_form(&self, guild_id: GuildId, new_form: &Form) -> Result<(), crate::Error> {
        Ok(self.connection_manager.clone().hset(get_forms_key(guild_id), new_form.id.to_string(), new_form).await?)
    }

    pub async fn delete_form(&self, guild_id: GuildId, id: FormId) -> Result<bool, crate::Error> {
        Ok(self.connection_manager.clone().hdel(get_forms_key(guild_id), id.to_string()).await?)
    }

    pub async fn get_form_ids(&self, guild_id: GuildId) -> Result<Vec<(FormId, String)>, crate::Error> {
        let forms: Vec<Form> = self.connection_manager.clone().hvals(get_forms_key(guild_id)).await?;
        Ok(forms.into_iter().map(|f| (f.id, f.title.clone())).collect())
    }

    pub async fn get_fields(&self, form_ref: FormRef) -> Result<Option<Vec<FormField>>, crate::Error> {
        Ok(self.get_form(form_ref).await?.map(|f| f.fields))
    }

    pub async fn cooldown(&self, form_ref: FormRef, user_id: UserId) -> Result<Option<Duration>, crate::Error> {
        let ttl: i64 = self.connection_manager.clone().ttl(get_cooldown_key(form_ref, user_id)).await?;
        Ok(match ttl {
            ..=0 => None,
            s => Some(Duration::from_secs(s as u64))
        })
    }

    pub async fn trigger_cooldown(&self, guild_id: GuildId, form: &Form, user_id: UserId) -> Result<(), crate::Error> {
        let Some(duration) = form.cooldown else {
            return Ok(())
        };

        self.connection_manager.clone().set_options(
            get_cooldown_key(FormRef::new(guild_id, form.id), user_id), 1,
            SetOptions::default().with_expiration(SetExpiry::EX(duration.as_secs())),
        ).await?;
        Ok(())
    }

    pub async fn clear_cooldown(&self, form_ref: FormRef, user_id: UserId) -> Result<bool, crate::Error> {
        Ok(self.connection_manager.clone().del(get_cooldown_key(form_ref, user_id)).await?)
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
            .max_length(self.min_length.unwrap_or(FIELD_RESPONSE_MAX_LENGTH))
            .required(self.required);

        if let Some(placeholder) = &self.placeholder {
            builder = builder.placeholder(placeholder);
        }

        if let Some(min_length) = self.min_length {
            builder = builder.min_length(min_length);
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
    cooldown: Option<Duration>,
}

impl FromRedisValue for Form {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        let serialized = <String as FromRedisValue>::from_redis_value(v)?;
        serde_json::from_str(&serialized).map_err(|e| (redis::ErrorKind::ParseError, "not valid form json", e.to_string()).into())
    }
}

impl ToRedisArgs for Form {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        let serialized = serde_json::to_vec(self).expect("failed to serialize form json");
        out.write_arg(&serialized);
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum AddFieldError {
    TooManyFields,
    IllegalAddBefore,
}

impl Display for AddFieldError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AddFieldError::TooManyFields => write!(f, "too many fields"),
            AddFieldError::IllegalAddBefore => write!(f, "illegal add-before target"),
        }
    }
}

impl std::error::Error for AddFieldError {}

#[derive(Debug)]
pub struct ValueTooLong;

impl Display for ValueTooLong {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "field too long")
    }
}

impl std::error::Error for ValueTooLong {}

impl Form {
    pub fn new<C: Into<ChannelId>>(title: String, destination: C) -> Result<Self, ValueTooLong> {
        Self::validate_title(&title)?;
        Ok(Self {
            id: FormId(Uuid::new_v4()),
            title,
            description: None,
            fields: vec![],
            destination: destination.into(),
            mention: None,
            cooldown: None,
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

    pub fn cooldown(&self) -> Option<Duration> {
        self.cooldown
    }

    pub fn set_cooldown(&mut self, cooldown: Option<Duration>) {
        self.cooldown = cooldown
            .map(|d| Duration::from_secs(d.as_secs()))
            .filter(|d| !d.is_zero());
    }

    pub fn fields(&self) -> &[FormField] {
        &self.fields
    }

    pub fn fields_mut(&mut self) -> &mut [FormField] { self.fields.as_mut_slice() }

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

    pub fn move_field(&mut self, index: usize, destination: usize) -> Result<bool, AddFieldError> {
        if index >= self.fields.len() {
            return Ok(false);
        }

        if destination >= self.fields.len() {
            return Err(AddFieldError::IllegalAddBefore);
        }

        let field = self.fields.remove(index);
        self.add_field(field, Some(destination))?;

        Ok(true)
    }

    fn validate_title(title: &str) -> Result<(), ValueTooLong> {
        if title.len() > 256 {
            Err(ValueTooLong)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use serenity::all::{ChannelId, InputTextStyle};
    use crate::state::{AddFieldError, Form, FormField};

    #[test]
    fn move_backward() {
        let mut form = create_form();
        assert!(form.move_field(4, 0).unwrap());
        assert_eq!(
            form.fields().iter().map(|f| &f.name).collect::<Vec<_>>(),
            ["Field 4", "Field 0", "Field 1", "Field 2", "Field 3"],
        );
    }

    #[test]
    fn move_forward() {
        let mut form = create_form();
        assert!(form.move_field(0, 4).unwrap());
        assert_eq!(
            form.fields().iter().map(|f| &f.name).collect::<Vec<_>>(),
            ["Field 1", "Field 2", "Field 3", "Field 4", "Field 0"],
        );
    }

    #[test]
    fn move_same() {
        let mut form = create_form();
        assert!(form.move_field(2, 2).unwrap());
        assert_eq!(
            form.fields().iter().map(|f| &f.name).collect::<Vec<_>>(),
            ["Field 0", "Field 1", "Field 2", "Field 3", "Field 4"],
        );
    }

    #[test]
    fn move_too_far() {
        let mut form = create_form();
        assert_eq!(form.move_field(0, 10), Err(AddFieldError::IllegalAddBefore));
    }

    fn create_form() -> Form {
        let mut form = Form::new("My Title".to_owned(), ChannelId::new(123)).unwrap();
        form.add_field(FormField::new("Field 0".to_owned(), InputTextStyle::Short).unwrap(), None).unwrap();
        form.add_field(FormField::new("Field 1".to_owned(), InputTextStyle::Short).unwrap(), None).unwrap();
        form.add_field(FormField::new("Field 2".to_owned(), InputTextStyle::Short).unwrap(), None).unwrap();
        form.add_field(FormField::new("Field 3".to_owned(), InputTextStyle::Short).unwrap(), None).unwrap();
        form.add_field(FormField::new("Field 4".to_owned(), InputTextStyle::Short).unwrap(), None).unwrap();
        form
    }
}