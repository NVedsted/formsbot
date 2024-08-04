use poise::serenity_prelude as serenity;

use crate::commands::get_commands;
use crate::errors::on_error;
use crate::event_handler::event_handler;
use crate::state::State;

mod commands;
mod event_handler;
mod state;
mod responses;
mod errors;
mod extensions;
mod utils;

type Error = Box<dyn std::error::Error + Send + Sync>;
type ApplicationContext<'a> = poise::ApplicationContext<'a, State, Error>;
type Framework = poise::Framework<State, Error>;
type FrameworkContext<'a> = poise::FrameworkContext<'a, State, Error>;
type Context<'a> = poise::Context<'a, State, Error>;
type FrameworkError<'a> = poise::FrameworkError<'a, State, Error>;

async fn setup(ctx: &serenity::Context, _: &serenity::Ready, framework: &Framework) -> Result<State, Error> {
    poise::builtins::register_globally(ctx, &framework.options().commands).await?;

    let redis_url = std::env::var("REDIS_URL").expect("please provide REDIS_URL");
    let redis_client = redis::Client::open(redis_url).expect("failed to connect to redis");
    let connection_manager = redis_client.get_connection_manager().await.expect("failed to setup redis connection manager");

    Ok(State { connection_manager })
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let token = std::env::var("DISCORD_TOKEN").expect("please provide DISCORD_TOKEN");
    let framework = poise::Framework::new(
        poise::FrameworkOptions {
            commands: get_commands(),
            on_error: |error| Box::pin(on_error(error)),
            event_handler: |ctx, event, framework, _| Box::pin(event_handler(ctx, event, framework)),
            ..Default::default()
        },
        |ctx, ready, framework| Box::pin(setup(ctx, ready, framework)));

    let mut client = serenity::Client::builder(token, serenity::GatewayIntents::non_privileged())
        .framework(framework)
        .await.expect("failed to build client");

    client.start().await.expect("failed running client");
}
