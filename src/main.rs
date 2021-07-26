use std::env;

use serenity::{
    async_trait,
    model::prelude::*,
    prelude::*,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use serenity::client::bridge::gateway::GatewayIntents;
use log::{info, warn, error};

const DEFAULT_SEARCH_DUR: Duration = Duration::from_secs(30);
const MIN_USERS_JOIN: usize = 8; // minimum of users to join in given duration to trigger actions
const DEFAULT_ACTION: Action = Action::Kick;

#[derive(Debug)]
enum Action {
    Kick,
    Ban,
}

// Captures a moment in time that a user joins the guild.
struct UserJoinMoment {
    instant: Instant,
    member: Member,
    action_taken: bool, // Whether this member has already been kicked / banned
}

struct GuildJoinsWatcher;

impl TypeMapKey for GuildJoinsWatcher {
    type Value = Arc<RwLock<HashMap<GuildId, Vec<UserJoinMoment>>>>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    // Called when a user joins the server.
    async fn guild_member_addition(&self, _ctx: Context, _guild_id: GuildId, _new_member: Member) {
        let guilds_lock = {
            let data_read = _ctx.data.read().await;
            data_read.get::<GuildJoinsWatcher>().expect("Expected GuildJoinsWatcher in TypeMap").clone()
        };

        let mut guilds_map = guilds_lock.write().await;
        let joins = guilds_map.entry(_guild_id).or_insert(Vec::new());
        let now = Instant::now();
        joins.push(UserJoinMoment{instant: now, member: _new_member, action_taken: false});

        // Clean up any values that are out of the required time span
        joins.retain(|v| now - v.instant <= DEFAULT_SEARCH_DUR);
        info!("{} users have joined within the search duration", joins.len());

        // Check if we can start banning users
        if joins.len() >= MIN_USERS_JOIN {
            info!("Stopping a raid in server {:?}", _guild_id);
            // Yeah, looks like a raid. Time to take action.
            for join in joins.iter_mut() {
                if !join.action_taken {
                    let _ = match DEFAULT_ACTION {
                        Action::Kick => join.member.kick_with_reason(_ctx.http.clone(), "Suspected bot; performing raid defense").await,
                        Action::Ban => join.member.ban_with_reason(_ctx.http.clone(), 7, "Suspected bot; performing raid defense").await,
                    };
                    join.action_taken = true;
                    info!("{:?} {}", DEFAULT_ACTION, join.member);
                }
            }
        }
    }

    // Called when a member who has previously joined is kicked, banned, or leaves.
    async fn guild_member_removal(&self, _ctx: Context, _guild_id: GuildId, _kicked: User) {
        let guilds_lock = {
            let data_read = _ctx.data.read().await;
            data_read.get::<GuildJoinsWatcher>().expect("Expected GuildJoinsWatcher in TypeMap").clone()
        };

        let mut guilds_map = guilds_lock.write().await;
        let joins = guilds_map.entry(_guild_id).or_insert(Vec::new());
        // Remove the member from the list if they are NOT recently kicked by raid-blocking
        joins.retain(|v| v.action_taken || v.member.user != _kicked);
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.mentions_user_id(ctx.http.get_current_user().await.expect("Could not find my own user").id) {
            if let Err(why) = msg.channel_id.say(&ctx.http, "That's me!").await {
                warn!("Error sending message: {:?}", why);
            }
        }
    }

    // Set a handler to be called on the `ready` event. This is called when a
    // shard is booted, and a READY payload is sent by Discord. This payload
    // contains data like the current user's guild Ids, current user data,
    // private channels, and more.
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
        ctx.set_activity(Activity::watching("for robots")).await;
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .intents(GatewayIntents::GUILD_MESSAGES | GatewayIntents::GUILD_MEMBERS) // events we want to receive
        .await.expect("Error creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<GuildJoinsWatcher>(Arc::new(RwLock::new(HashMap::new())));
    }

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}
