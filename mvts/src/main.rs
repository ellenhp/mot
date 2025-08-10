use anyhow::Ok;
use clap::Parser;
use sqlx::{Connection, PgConnection};

#[derive(Debug, Parser)]
#[command(name = "mvts", about = "MapLibre Vector Tile Search utilities")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Parser)]
enum Commands {
    BuildTokenizer(BuildTokenizer),
}

#[derive(Debug, Parser)]
struct BuildTokenizer {
    /// PostgreSQL connection string.
    db: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::BuildTokenizer(build_tokenizer) => {
            let mut _conn = PgConnection::connect(&build_tokenizer.db).await?;
        }
    }

    Ok(())
}
