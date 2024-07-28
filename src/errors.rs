use std::fmt::{Display, Formatter};

use crate::FrameworkError;

#[derive(Debug)]
pub struct UserFriendlyError(String);

impl UserFriendlyError {
    pub fn new<T: Into<String>>(message: T) -> Self {
        UserFriendlyError(message.into())
    }
}

impl Display for UserFriendlyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0)
    }
}

impl std::error::Error for UserFriendlyError {}

pub async fn on_error(error: FrameworkError<'_>) {
    match error {
        FrameworkError::Command { ctx, error, .. } if error.is::<UserFriendlyError>() => {
            let error = error.downcast::<UserFriendlyError>().expect("not a user-friendly error");

            if let Err(e) = ctx.say(error.to_string()).await {
                tracing::error!("Error while handling user-friendly error: {}", e);
            }
        }
        _ => {
            if let Err(e) = poise::builtins::on_error(error).await {
                tracing::error!("Error while handling error: {}", e);
            }
        }
    }
}