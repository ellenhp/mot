mod process_admin;
mod sync_txn;
mod whosonfirst;

use std::{collections::HashMap, fs::File, path::PathBuf, sync::Mutex};

use base64::{Engine, prelude::BASE64_STANDARD};
use clap::Parser;
use futures_util::TryStreamExt;
use process_admin::ProcessAdmin;
use roaring::RoaringBitmap;
use sqlx::{PgPool, query};
use sync_txn::JOIN_HANDLES;
use tokenizers::Tokenizer;
use tracing::{debug, info, warn};

#[derive(Debug, Parser)]
#[command(name = "mvts", about = "MapLibre Vector Tile Search utilities")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Parser)]
enum Commands {
    LoadWof(LoadWhosOnFirst),
    GenerateBitmaps(GenerateBitmaps),
}

#[derive(Debug, Parser)]
struct LoadWhosOnFirst {
    /// PostgreSQL connection string.
    db: String,
    /// WhosOnFirst Spatialite database. If downloaded from geocode.earth, the filename should end in .spatial.db
    wof: PathBuf,
}

#[derive(Debug, Parser)]
struct GenerateBitmaps {
    /// PostgreSQL connection string.
    db: String,
    /// Where to write the bitmaps.
    out: PathBuf,
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
        Commands::GenerateBitmaps(generate_bitmaps) => {
            let mut bitmaps: HashMap<u32, RoaringBitmap> = HashMap::new();
            let tokenizer =
                Tokenizer::from_pretrained("google-bert/bert-base-multilingual-uncased", None)
                    .map_err(|err| anyhow::anyhow!("Failed to download tokenizer {}", err))?;
            let pool = PgPool::connect(&generate_bitmaps.db).await?;
            let mut tags =
                query!("select poi.tags, tiles.idx FROM poi JOIN tiles ON poi.geom && tiles.geom")
                    .fetch(&pool);
            while let Ok(Some(row)) = tags.try_next().await {
                let tags = if let Some(tags) = row.tags {
                    tags
                } else {
                    warn!("POI missing tags");
                    continue;
                };
                for (key, value) in tags.as_object().unwrap() {
                    if key.contains("name") || key.contains("addr:") {
                        let encoding = tokenizer
                            .encode(value.as_str().unwrap(), false)
                            .map_err(|err| anyhow::anyhow!("Failed to tokenize string: {}", err))?;
                        for id in encoding.get_ids() {
                            if !bitmaps.contains_key(id) {
                                bitmaps.insert(*id, RoaringBitmap::new());
                            }
                            bitmaps.get_mut(id).unwrap().insert(row.idx.unwrap() as u32);
                        }
                    }
                }
            }
            let mut files = HashMap::new();
            for (token, bitmap) in &bitmaps {
                let mut buf = Vec::new();
                let token_word = tokenizer.id_to_token(*token).unwrap();
                debug!("Processing token {token_word}");
                bitmap.serialize_into(&mut buf)?;
                info!("Serialized size of {token_word}: {}", buf.len());
                files.insert(token_word, BASE64_STANDARD.encode(&buf));
            }
            let writer = File::create(&generate_bitmaps.out)?;
            serde_json::to_writer(&writer, &files)?;
        }
    }

    let handles = JOIN_HANDLES.get_or_init(|| Mutex::new(Vec::new()));
    while handles.lock().unwrap().len() > 0 {
        let handle = handles.lock().unwrap().swap_remove(0);
        handle.into_future().await?;
    }
    Ok(())
}
