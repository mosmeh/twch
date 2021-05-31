use crate::message::{FallbackColor, TwitchMessage};

use futures::{Stream, StreamExt};
use irc::client::{prelude::*, ClientStream};
use rand::Rng;
use std::{
    collections::HashMap,
    convert::TryFrom,
    pin::Pin,
    task::{Context, Poll},
};

pub struct TwitchChannelStream {
    client_stream: ClientStream,
    color_map: HashMap<u64, FallbackColor>,
}

impl TwitchChannelStream {
    pub async fn new(channel: &str) -> irc::error::Result<Self> {
        let channel = format!("#{}", channel.to_ascii_lowercase());

        let config = Config {
            server: Some("irc.chat.twitch.tv".to_owned()),
            nickname: Some("justinfan1234".to_owned()),
            channels: vec![channel],
            ..Config::default()
        };

        let mut client = Client::from_config(config).await?;
        client.send_cap_req(&[Capability::Custom("twitch.tv/tags")])?;
        client.identify()?;

        Ok(Self {
            client_stream: client.stream()?,
            color_map: HashMap::new(),
        })
    }
}

impl Stream for TwitchChannelStream {
    type Item = TwitchMessage;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        while let Poll::Ready(maybe_msg) = self.client_stream.poll_next_unpin(cx) {
            match maybe_msg {
                Some(Ok(msg)) => {
                    if let Ok(mut msg) = TwitchMessage::try_from(msg) {
                        if msg.color().is_none() {
                            let color = *self
                                .color_map
                                .entry(msg.user_id())
                                .or_insert_with(|| rand::thread_rng().gen());
                            msg.set_color(color.into());
                        }
                        return Poll::Ready(Some(msg));
                    }
                }
                Some(Err(_)) | None => {
                    return Poll::Ready(None);
                }
            }
        }

        Poll::Pending
    }
}
