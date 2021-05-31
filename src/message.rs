use ansi_term::{Color, Style};
use irc::client::prelude::*;
use itertools::Itertools;
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};
use std::{
    convert::TryFrom,
    fmt::{Display, Write},
    ops::Range,
    str::FromStr,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Missing value: {0}")]
    MissingValue(&'static str),
    #[error("Invalid value: {0}")]
    InvalidValue(&'static str),
}

#[derive(PartialEq, Debug)]
pub struct TwitchMessage {
    user_id: u64,
    nick_name: String,
    content: String,
    display_name: Option<String>,
    color: Option<TwitchColor>,
    is_action: bool,
    emotes: Vec<Emote>,
}

impl TwitchMessage {
    pub fn user_id(&self) -> u64 {
        self.user_id
    }

    pub fn color(&self) -> &Option<TwitchColor> {
        &self.color
    }

    pub fn set_color(&mut self, color: TwitchColor) {
        self.color = Some(color);
    }
}

impl TryFrom<irc::proto::Message> for TwitchMessage {
    type Error = ParseError;

    fn try_from(msg: irc::proto::Message) -> Result<Self, Self::Error> {
        if let Command::PRIVMSG(_, content) = &msg.command {
            let nick_name = msg
                .source_nickname()
                .ok_or(ParseError::MissingValue("nick name"))?;

            let mut user_id = None;
            let mut display_name = None;
            let mut color = None;
            let mut is_action = false;
            let mut emotes = Vec::new();

            let content = if let Some(stripped) = content.strip_prefix("\u{1}ACTION ") {
                is_action = true;
                stripped.strip_suffix('\u{1}').unwrap_or(stripped)
            } else {
                content
            };

            let tags = msg.tags.as_ref().ok_or(ParseError::MissingValue("tags"))?;
            for tag in tags {
                if let Some(value) = &tag.1 {
                    if value.is_empty() {
                        continue;
                    }

                    match tag.0.as_str() {
                        "user-id" => {
                            user_id = Some(
                                value
                                    .parse()
                                    .map_err(|_| ParseError::InvalidValue("user-id"))?,
                            )
                        }
                        "display-name" => display_name = Some(value.clone()),
                        "color" => {
                            color = Some(
                                value
                                    .parse()
                                    .map_err(|_| ParseError::InvalidValue("color"))?,
                            )
                        }
                        "emotes" => {
                            emotes = value
                                .split('/')
                                .map(|x| x.parse::<Emote>())
                                .collect::<Result<Vec<_>, _>>()
                                .map_err(|_| ParseError::InvalidValue("emotes"))?;
                        }
                        _ => (),
                    }
                }
            }

            Ok(Self {
                user_id: user_id.ok_or(ParseError::MissingValue("user-id"))?,
                nick_name: nick_name.to_owned(),
                content: content.to_owned(),
                display_name,
                color,
                is_action,
                emotes,
            })
        } else {
            Err(ParseError::InvalidValue("not a PRIVMSG"))
        }
    }
}

impl Display for TwitchMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match &self.display_name {
            Some(display) if display.eq_ignore_ascii_case(&self.nick_name) => display.clone(),
            Some(display) => format!("{} ({})", display, self.nick_name),
            None => self.nick_name.clone(),
        };

        if let Some(color) = self.color {
            write!(f, "{}", Color::from(color).paint(name))?;
        } else {
            f.write_str(&name)?;
        }

        let mut style = Style::new();
        if self.is_action {
            style = style.italic();
            f.write_char(' ')?;
        } else {
            f.write_str(": ")?;
        }

        let ranges = self
            .emotes
            .iter()
            .flat_map(|emote| &emote.ranges)
            .sorted_by_key(|range| range.start);

        let mut prev_end = 0;
        for range in ranges {
            if prev_end < range.start {
                let text: String = self
                    .content
                    .chars()
                    .skip(prev_end)
                    .take(range.start - prev_end)
                    .collect();
                write!(f, "{}", style.paint(text))?;
            }
            if range.start < range.end {
                let text: String = self
                    .content
                    .chars()
                    .skip(range.start)
                    .take(range.end - range.start)
                    .collect();
                write!(f, "{}", style.underline().paint(text))?;
            }
            prev_end = range.end;
        }
        if prev_end < self.content.chars().count() {
            let text: String = self.content.chars().skip(prev_end).collect();
            write!(f, "{}", style.paint(text))?;
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TwitchColor(u8, u8, u8);

impl FromStr for TwitchColor {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !(s.is_ascii() && s.len() == 7 && s.starts_with('#')) {
            return Err(());
        }

        let to_u8 = |src| u8::from_str_radix(src, 16);
        let r = to_u8(&s[1..3]);
        let g = to_u8(&s[3..5]);
        let b = to_u8(&s[5..7]);

        if let (Ok(r), Ok(g), Ok(b)) = (r, g, b) {
            Ok(Self(r, g, b))
        } else {
            Err(())
        }
    }
}

impl From<TwitchColor> for Color {
    fn from(c: TwitchColor) -> Self {
        Self::RGB(c.0, c.1, c.2)
    }
}

// See https://static.twitchcdn.net/assets/pages.channel.components.channel-shell.components.chat-shell.components.chat-live-*.js
// var u=["#FF0000","#0000FF","#008000","#B22222","#FF7F50","#9ACD32","#FF4500","#2E8B57","#DAA520","#D2691E","#5F9EA0","#1E90FF","#FF69B4","#8A2BE2","#00FF7F"]
// function f(e,n){return void 0===n&&(n=15),"number"!=typeof c[e]&&(c[e]=Math.floor(Math.random()*n)),u[c[e]]}

#[derive(Clone, Copy)]
pub struct FallbackColor(u8);

impl FallbackColor {
    const NUM_COLORS: usize = 15;
}

impl Distribution<FallbackColor> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> FallbackColor {
        FallbackColor(rng.gen_range(0..FallbackColor::NUM_COLORS as u8))
    }
}

impl From<FallbackColor> for TwitchColor {
    fn from(c: FallbackColor) -> Self {
        const COLORS: [TwitchColor; FallbackColor::NUM_COLORS] = [
            Self(255, 0, 0),
            Self(0, 0, 255),
            Self(0, 128, 0),
            Self(178, 34, 34),
            Self(255, 127, 80),
            Self(154, 205, 50),
            Self(255, 69, 0),
            Self(46, 139, 87),
            Self(218, 165, 32),
            Self(210, 105, 30),
            Self(95, 158, 160),
            Self(30, 144, 255),
            Self(255, 105, 180),
            Self(138, 43, 226),
            Self(0, 255, 127),
        ];
        COLORS[c.0 as usize]
    }
}

#[derive(Debug, PartialEq)]
struct Emote {
    id: usize,
    ranges: Vec<Range<usize>>,
}

impl FromStr for Emote {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (id_str, indices_str) = s.split_once(':').ok_or(())?;
        let ranges = indices_str
            .split(',')
            .map(|range_str| {
                let (start, end) = range_str.split_once('-').ok_or(())?;
                let start = start.parse().map_err(|_| ())?;
                let end: usize = end.parse().map_err(|_| ())?;
                Ok(start..end + 1)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            id: id_str.parse().map_err(|_| ())?,
            ranges,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_message() {
        let msg = "@badge-info=;badges=global_mod/1,turbo/1;color=#0D4200;display-name=ronni;emotes=25:0-4,12-16/1902:6-10;id=b34ccfc7-4977-403a-8a94-33c6bac34fb8;mod=0;room-id=1337;subscriber=0;tmi-sent-ts=1507246572675;turbo=1;user-id=1337;user-type=global_mod :ronni!ronni@ronni.tmi.twitch.tv PRIVMSG #ronni :Kappa Keepo Kappa";
        let msg = irc::proto::Message::from(msg);
        let msg = TwitchMessage::try_from(msg).unwrap();
        assert_eq!(
            msg,
            TwitchMessage {
                user_id: 1337,
                nick_name: "ronni".to_owned(),
                content: "Kappa Keepo Kappa".to_owned(),
                display_name: Some("ronni".to_owned()),
                color: Some(TwitchColor(13, 66, 0)),
                is_action: false,
                emotes: vec![
                    Emote {
                        id: 25,
                        ranges: vec![0..5, 12..17]
                    },
                    Emote {
                        id: 1902,
                        ranges: vec![6..11,]
                    }
                ],
            }
        );

        let underline = Style::new().underline();
        assert_eq!(
            msg.to_string(),
            format!(
                "{}: {} {} {}",
                Color::RGB(13, 66, 0).paint("ronni"),
                underline.paint("Kappa"),
                underline.paint("Keepo"),
                underline.paint("Kappa")
            ),
        );
    }
}
