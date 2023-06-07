use poise::serenity_prelude as serenity;
use poise::{Framework, FrameworkOptions};
use anyhow::{Result, Error};
use mobc::Pool;
use mobc_redis::RedisConnectionManager;
use mobc_redis::redis as mredis;
use mobc_redis::redis::AsyncCommands;


const MESSAGE_SET_KEY: &'static str = "parallel-bot-messages";

struct Data {
    pool: Pool<RedisConnectionManager>
}

impl  Data {
    async fn new() -> Result<Data> {
        let redis_url = std::env::var("REDIS_URL")?;
        tracing::info!("The redis URL is {}", redis_url);
        let client = mredis::Client::open(("127.0.0.1", 36379))?;
        let manager = RedisConnectionManager::new(client);
        let pool = Pool::builder().max_open(5).build(manager);
        pool.get().await?.get("test_key").await?;
        Ok(Data { pool })
    }
}
type Context<'a> = poise::Context<'a, Data, Error>;

#[poise::command(slash_command, prefix_command)]
async fn age(ctx: Context<'_>, 
             #[description = "Selected user"]
             user: Option<serenity::User>) -> Result<()> {
    let mut conn = ctx.data().pool.clone().get().await?;
    let res: i32 = conn.sismember(MESSAGE_SET_KEY, ctx.id()).await?;
    if res == 1 {
        tracing::info!("The message {} is already being processed", ctx.id());
        return Ok(());
    }
    conn.sadd(MESSAGE_SET_KEY, ctx.id()).await?;
    let u = user.as_ref().unwrap_or_else(|| ctx.author());
    let response = format!("{} was created at {}", u.name, u.created_at());
    tracing::info!("the response is going to be {}", response);
    ctx.say(response).await?;
    Ok(())
}


#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let token = std::env::var("DISCORD_TOKEN").expect("missing discord token");
    let framework = Framework::builder().options(
        FrameworkOptions {
            commands: vec![age()],
            ..Default::default()
        }
    ).token(token).intents(serenity::GatewayIntents::non_privileged())
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Data::new().await
            })
        });
    framework.run().await.unwrap();
}
