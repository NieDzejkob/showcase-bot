use anyhow::{Context as _, Result};
use serde::Deserialize;
use serenity::{async_trait, model::prelude::*, prelude::*};
use tokio::fs;

#[derive(Debug, Deserialize)]
struct Config {
    token: String,
    server: GuildId,
    target_channel: ChannelId,
    allowed_role: RoleId,
    trigger_emoji: String,
}

struct Handler(Config);

impl Handler {
    async fn handle_reaction(&self, ctx: Context, react: Reaction) -> Result<()> {
        match &react.emoji {
            ReactionType::Custom {
                name: Some(name), ..
            } if name == &self.0.trigger_emoji => (),
            _ => return Ok(()),
        }

        let user = react.user(&ctx).await.context("Couldn't fetch user")?;
        if !user
            .has_role(&ctx, self.0.server, self.0.allowed_role)
            .await
            .context("Couldn't check roles")?
        {
            return Ok(());
        }

        let msg = react
            .channel_id
            .message(&ctx, react.message_id)
            .await
            .context("Couldn't fetch message")?;

        dbg!(&msg);

        self.0
            .target_channel
            .send_message(&ctx, |m| {
                m.embed(|e| {
                    e.author(|a| {
                        a.name(&msg.author.name);
                        let avatar = msg
                            .author
                            .avatar_url()
                            .unwrap_or_else(|| msg.author.default_avatar_url());
                        a.icon_url(avatar)
                    });

                    e.description(&msg.content);

                    for attachment in msg.attachments.iter() {
                        e.image(&attachment.url);
                    }

                    e
                });
                m
            })
            .await
            .context("Couldn't send message")?;
        Ok(())
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn reaction_add(&self, ctx: Context, react: Reaction) {
        if let Err(why) = self.handle_reaction(ctx, react).await {
            println!("An error occured: {}", why);
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = fs::read_to_string("config.toml")
        .await
        .context("Cannot read config.toml")?;
    let mut config: Config = toml::de::from_str(&config).context("Cannot parse config.toml")?;
    let token = std::mem::take(&mut config.token);
    let mut client = Client::builder(&token)
        .event_handler(Handler(config))
        .await
        .context("Couldn't create serenity::Client")?;
    client.start().await?;
    Ok(())
}
