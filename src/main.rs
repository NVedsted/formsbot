use poise::serenity_prelude as serenity;

use crate::commands::get_commands;
use crate::event_handler::event_handler;
use crate::state::State;

mod commands;
mod event_handler;
mod state;
mod responses;

type Error = Box<dyn std::error::Error + Send + Sync>;
type ApplicationContext<'a> = poise::ApplicationContext<'a, State, Error>;
type FrameworkContext<'a> = poise::FrameworkContext<'a, State, Error>;
type Context<'a> = poise::Context<'a, State, Error>;


// TODO: what intents are needed?
// TODO: what permissions are needed?
// TODO: what if form is too long?

#[tokio::main]
async fn main() {
    env_logger::init();

    let token = std::env::var("DISCORD_TOKEN").expect("please provide DISCORD_TOKEN");
    let redis_client = redis::Client::open("redis://127.0.0.1/").expect("failed to connect to redis");

    let connection_manager = redis_client.get_connection_manager().await.expect("failed to setup redis connection manager");

    let framework = poise::Framework::new(
        poise::FrameworkOptions {
            commands: get_commands(),
            event_handler: |ctx, event, framework, _| Box::pin(event_handler(ctx, event, framework)),
            ..Default::default()
        },
        |_, _, _| Box::pin(async move { Ok(State { connection_manager, forms: Default::default() }) }));

    let mut client = serenity::Client::builder(token, serenity::GatewayIntents::non_privileged())
        .framework(framework)
        .await.expect("failed to build client");

    client.start().await.expect("failed running client");
}
