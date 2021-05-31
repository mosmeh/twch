mod channel_stream;
pub mod message;
mod search;
mod streams;

pub use channel_stream::TwitchChannelStream;
pub use search::search_channels;
pub use streams::get_streams;

use ansi_term::Color;
use serde::Deserialize;
use std::fmt::Display;

#[derive(Clone)]
pub struct Auth {
    client_id: String,
    oauth_token: String,
}

impl Auth {
    pub fn from_env() -> Result<Self, std::env::VarError> {
        let client_id = std::env::var("CLIENT_ID")?;

        let oauth_token = std::env::var("OAUTH_TOKEN")?;
        let oauth_token = oauth_token.strip_prefix("oauth:").unwrap_or(&oauth_token);

        Ok(Self {
            client_id,
            oauth_token: oauth_token.to_owned(),
        })
    }
}

#[derive(Deserialize)]
pub struct TwitchStream {
    user_login: String,
    user_name: String,
    game_name: String,
    title: String,
    viewer_count: Option<usize>,
}

impl Display for TwitchStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} /{}",
            Color::Green.paint(&self.user_name),
            self.user_login
        )?;

        if !self.game_name.is_empty() {
            write!(f, " - {}", Color::Blue.paint(&self.game_name))?;
        }
        if let Some(viewer_count) = self.viewer_count {
            write!(f, " ({} viewers)", viewer_count)?;
        }

        let title = self.title.trim();
        if !title.is_empty() {
            write!(f, "\n{}", title)?;
        }

        Ok(())
    }
}
