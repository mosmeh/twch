use futures::StreamExt;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opt {
    #[structopt(subcommand)]
    cmd: Option<Command>,
}

#[derive(StructOpt)]
enum Command {
    List {
        #[structopt(short, default_value = "10")]
        n: usize,
    },
    Search {
        query: String,

        #[structopt(short, default_value = "10")]
        n: usize,
    },
    View {
        channel: String,
    },
}

impl Default for Command {
    fn default() -> Self {
        Self::List { n: 10 }
    }
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let opt = Opt::from_args();
    let auth = twch::Auth::from_env()?;

    match opt.cmd.unwrap_or_default() {
        Command::List { n } => {
            println!(
                "{}",
                twch::get_streams(n, &auth)
                    .await
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?
                    .into_iter()
                    .map(|stream| format!("{}\n", stream))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
        Command::Search { query, n } => {
            println!(
                "{}",
                twch::search_channels(&query, n, &auth)
                    .await
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?
                    .into_iter()
                    .map(|stream| format!("{}\n", stream))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
        Command::View { channel } => {
            twch::TwitchChannelStream::new(&channel)
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string()))?
                .for_each(|msg| {
                    println!("{}", msg);
                    futures::future::ready(())
                })
                .await;
        }
    }

    Ok(())
}
