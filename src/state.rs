use std::time::Duration;
use poise::async_trait;
use poise::serenity_prelude::*;
use tokio::sync::RwLock;

pub struct State {
    pub connection_manager: redis::aio::ConnectionManager,
    pub forms: RwLock<Vec<Form>>, // TODO: add persistence
}

impl State {
    pub async fn get_form(&self, id: usize) -> Option<Form> {
        self.forms.read().await.get(id).cloned()
    }

    pub async fn create_form(&self, title: String, destination: ChannelId, mention: Option<SerializableMention>) {
        self.forms.write().await.push(Form { title, fields: vec![], destination, mention });
    }

    pub async fn save_form(&self, id: usize, new_form: Form) {
        let mut forms = self.forms.write().await;

        if let Some(form) = forms.get_mut(id) {
            *form = new_form;
        }
    }

    pub async fn delete_form(&self, id: usize) -> bool {
        let mut forms = self.forms.write().await;

        if id < forms.len() {
            forms.remove(id);
            true
        } else {
            false
        }
    }

    pub async fn get_forms(&self) -> Vec<(usize, String)> {
        self.forms.read().await.iter().map(|f| &f.title).cloned().enumerate().collect()
    }

    pub async fn get_fields(&self, id: usize) -> Option<Vec<(usize, String)>> {
        Some(self.get_form(id).await?.fields.into_iter().enumerate().map(|(i, f)| (i, f.name)).collect())
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
    async fn extract(
        ctx: &Context,
        interaction: &CommandInteraction,
        value: &ResolvedValue<'_>,
    ) -> Result<Self, poise::SlashArgError> {
        if let ResolvedValue::Role(_) = value {
            RoleId::extract(ctx, interaction, value)
                .await
                .map(|r| SerializableMention::Role(r))
        } else {
            UserId::extract(ctx, interaction, value)
                .await
                .map(|u| SerializableMention::User(u))
        }
    }

    fn create(builder: CreateCommandOption) -> CreateCommandOption {
        builder.kind(CommandOptionType::Mentionable)
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct FormField {
    name: String,
    style: InputTextStyle,
    placeholder: Option<String>,
    min_length: Option<u16>,
    max_length: Option<u16>,
    required: Option<bool>,
    inline: Option<bool>,
}

impl FormField {
    fn input_text<T: Into<String>>(&self, custom_id: T) -> CreateInputText {
        let mut builder = CreateInputText::new(self.style, &self.name, custom_id);

        if let Some(placeholder) = &self.placeholder {
            builder = builder.placeholder(placeholder);
        }

        if let Some(min_length) = self.min_length {
            builder = builder.min_length(min_length);
        }

        if let Some(max_length) = self.max_length {
            builder = builder.max_length(max_length);
        }

        if let Some(required) = self.required {
            builder = builder.required(required);
        }

        builder
    }

    pub fn apply_to_embed(&self, embed: CreateEmbed, value: String) -> CreateEmbed {
        embed.field(&self.name, value, self.inline.unwrap_or(false))
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Form {
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
            required,
            inline,
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