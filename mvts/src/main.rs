mod process_admin;
mod sync_txn;
mod whosonfirst;

use std::{path::PathBuf, sync::Mutex};

use anyhow::Ok;
use clap::Parser;
use process_admin::ProcessAdmin;
use sqlx::PgPool;
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
struct LoadWhosOnFirst {
    /// PostgreSQL connection string.
    db: String,
    /// WhosOnFirst Spatialite database. If downloaded from geocode.earth, the filename should end in .spatial.db
    wof: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let cli = Cli::parse();

    match cli.command {
        Commands::LoadWof(build_tokenizer) => {
            let pool = PgPool::connect(&build_tokenizer.db).await?;
            let wof = whosonfirst::WhosOnFirst::new(&build_tokenizer.wof).await?;

            let mut process_admin = ProcessAdmin::new(pool.begin().await?).await?;
            wof.clone()
                .for_polygon(async move |row| {
                    process_admin.process_admin(&wof, &row).await.unwrap()
                })
                .await?;
        }
    }

    let handles = JOIN_HANDLES.get_or_init(|| Mutex::new(Vec::new()));
    while handles.lock().unwrap().len() > 0 {
        let handle = handles.lock().unwrap().swap_remove(0);
        handle.into_future().await?;
    }
    Ok(())
}
