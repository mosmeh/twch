use twch::Auth;

use actix_web::{
    error, get, http::header, middleware, web, App, HttpResponse, HttpServer, Responder,
};
use futures::StreamExt;
use serde::Deserialize;
use std::{task::Poll, time::Duration};

#[derive(Clone)]
struct Config {
    auth: Auth,
    heartbeat_interval: Duration,
}

impl Config {
    fn from_env() -> anyhow::Result<Self> {
        let auth = Auth::from_env()?;
        let heartbeat_interval = if let Ok(value) = std::env::var("HEARTBEAT_INTERVAL") {
            value.parse()?
        } else {
            10
        };

        Ok(Self {
            auth,
            heartbeat_interval: Duration::from_secs(heartbeat_interval),
        })
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let config = Config::from_env().unwrap();

    let http_addr = std::env::var("HTTP_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_owned());
    eprintln!("Listening on http://{}", http_addr);

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .wrap(middleware::NormalizePath::new(
                middleware::normalize::TrailingSlash::Trim,
            ))
            .wrap(
                middleware::DefaultHeaders::default()
                    .header(header::CONTENT_TYPE, "text/plain; charset=utf-8"),
            )
            .data(config.clone())
            .service(get_streams)
            .service(search_channels)
            .service(start_channel_stream)
    })
    .bind(http_addr)?
    .run()
    .await
}

#[derive(Deserialize)]
#[serde(default)]
struct GetStreamsQueryParams {
    limit: usize,
}

impl Default for GetStreamsQueryParams {
    fn default() -> Self {
        Self { limit: 10 }
    }
}

#[get("/")]
async fn get_streams(
    params: web::Query<GetStreamsQueryParams>,
    config: web::Data<Config>,
) -> actix_web::Result<impl Responder> {
    let body = twch::get_streams(params.limit, &config.auth)
        .await?
        .into_iter()
        .map(|stream| format!("{}\n", stream))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(HttpResponse::Ok().body(body))
}

#[derive(Deserialize)]
struct SearchChannelsQueryParams {
    #[serde(rename = "q")]
    query: String,
    limit: Option<usize>,
}

#[get("/search")]
async fn search_channels(
    params: web::Query<SearchChannelsQueryParams>,
    config: web::Data<Config>,
) -> actix_web::Result<impl Responder> {
    let body = twch::search_channels(&params.query, params.limit.unwrap_or(10), &config.auth)
        .await?
        .into_iter()
        .map(|stream| format!("{}\n", stream))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(HttpResponse::Ok().body(body))
}

#[get("/{channel:[a-zA-Z0-9_]+}")]
async fn start_channel_stream(
    web::Path((channel,)): web::Path<(String,)>,
    config: web::Data<Config>,
) -> actix_web::Result<impl Responder> {
    let mut stream = twch::TwitchChannelStream::new(&channel)
        .await
        .map_err(error::ErrorInternalServerError)?;
    let mut interval = actix_web::rt::time::interval(config.heartbeat_interval);

    let stream =
        futures::stream::poll_fn(move |cx| -> Poll<Option<actix_web::Result<web::Bytes>>> {
            let mut buf = String::new();

            while let Poll::Ready(maybe_msg) = stream.poll_next_unpin(cx) {
                match maybe_msg {
                    Some(msg) => {
                        buf.push_str(&msg.to_string());
                        buf.push('\n');
                    }
                    None => return Poll::Ready(None),
                }
            }

            if !buf.is_empty() {
                return Poll::Ready(Some(Ok(web::Bytes::from(buf))));
            }

            match interval.poll_next_unpin(cx) {
                Poll::Ready(_) => {
                    // space + backspace
                    Poll::Ready(Some(Ok(web::Bytes::from(" \x08"))))
                }
                Poll::Pending => Poll::Pending,
            }
        });

    let response = HttpResponse::Ok()
        .header(header::CACHE_CONTROL, "no-cache")
        .header("X-Accel-Buffering", "no")
        .streaming(stream);
    Ok(response)
}
