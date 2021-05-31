use crate::{Auth, TwitchStream};

use actix_web::http::HeaderValue;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct QueryParams {
    first: usize,
}

#[derive(Deserialize)]
struct StreamsResponse {
    data: Vec<TwitchStream>,
}

pub async fn get_streams(limit: usize, auth: &Auth) -> actix_web::Result<Vec<TwitchStream>> {
    let mut client_id = HeaderValue::from_str(&auth.client_id)?;
    client_id.set_sensitive(true);

    let response = actix_web::client::Client::new()
        .get("https://api.twitch.tv/helix/streams")
        .query(&QueryParams { first: limit })?
        .bearer_auth(&auth.oauth_token)
        .header("client-id", client_id)
        .send()
        .await?
        .json::<StreamsResponse>()
        .await?;

    Ok(response.data)
}
