mod process_admin;
mod sync_txn;
mod whosonfirst;

use clap::Parser;
use process_admin::{LoadWhosOnFirst, LoadWhosOnFirstCommand};
use std::path::PathBuf;
use std::sync::Mutex;
use sync_txn::JOIN_HANDLES;

#[derive(Debug, Parser)]
#[command(name = "mvts", about = "MapLibre Vector Tile Search utilities")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Parser)]
enum Commands {
    LoadWof(LoadWhosOnFirst),
}

#[derive(Debug, Parser)]
struct GenerateBitmaps {
    /// PostgreSQL connection string.
    db: String,
    /// Where to write the bitmaps.
    out: PathBuf,
}

#[derive(Debug, Parser)]
struct ReorderBitmaps {
    /// Where to read the bitmaps.
    r#in: PathBuf,
    /// Where to write the bitmaps.
    out: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let cli = Cli::parse();

    match cli.command {
        Commands::LoadWof(load_wof) => {
            let command = LoadWhosOnFirstCommand {
                db: load_wof.db,
                wof: load_wof.wof,
            };
            command.run().await?;
        }
    }

    let handles = JOIN_HANDLES.get_or_init(|| Mutex::new(Vec::new()));
    while handles.lock().unwrap().len() > 0 {
        let handle = handles.lock().unwrap().swap_remove(0);
        handle.into_future().await?;
    }
    Ok(())
}
