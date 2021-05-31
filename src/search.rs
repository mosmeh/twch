use crate::{Auth, TwitchStream};

use actix_web::http::HeaderValue;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct QueryParams<'a> {
    query: &'a str,
    first: usize,
    live_only: bool,
}

#[derive(Deserialize)]
struct ChannelsResponse {
    data: Vec<TwitchChannel>,
}

#[derive(Deserialize)]
struct TwitchChannel {
    broadcaster_login: String,
    display_name: String,
    game_name: String,
    title: String,
}

impl From<TwitchChannel> for TwitchStream {
    fn from(c: TwitchChannel) -> Self {
        Self {
            user_login: c.broadcaster_login,
            user_name: c.display_name,
            game_name: c.game_name,
            title: c.title,
            viewer_count: None,
        }
    }
}

pub async fn search_channels(
    query: &str,
    limit: usize,
    auth: &Auth,
) -> actix_web::Result<Vec<TwitchStream>> {
    let mut client_id = HeaderValue::from_str(&auth.client_id)?;
    client_id.set_sensitive(true);

    let response = actix_web::client::Client::new()
        .get("https://api.twitch.tv/helix/search/channels")
        .query(&QueryParams {
            query,
            first: limit,
            live_only: true,
        })?
        .bearer_auth(&auth.oauth_token)
        .header("client-id", client_id)
        .send()
        .await?
        .json::<ChannelsResponse>()
        .await?;

    let streams = response.data.into_iter().map(Into::into).collect();
    Ok(streams)
}
