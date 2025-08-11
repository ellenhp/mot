mod process_admin;
mod sync_txn;
mod whosonfirst;

use clap::Parser;
use futures_util::TryStreamExt;
use process_admin::ProcessAdmin;
use roaring::RoaringBitmap;
use sqlx::{PgPool, query};
use std::ops::BitXor;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{Cursor, Read, Write},
    path::PathBuf,
    sync::Mutex,
};
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
    ReorderBitmaps(ReorderBitmaps),
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
            let mut files = vec![Vec::new(); tokenizer.get_vocab_size(false)];
            for (token, bitmap) in &bitmaps {
                let mut buf = Vec::new();
                let token_word = tokenizer.id_to_token(*token).unwrap();
                debug!("Processing token {token_word}");
                bitmap.serialize_into(&mut buf)?;
                files[*token as usize] = buf;
            }
            let buf = rmp_serde::to_vec(&files)?;
            let mut writer = File::create(&generate_bitmaps.out)?;
            writer.write_all(&buf)?;
        }
        Commands::ReorderBitmaps(ReorderBitmaps { r#in, out }) => {
            let mut input = Vec::new();
            File::open(r#in)?.read_to_end(&mut input)?;
            let input: Vec<Vec<u8>> = rmp_serde::from_slice(&input)?;
            let bitmaps: HashMap<usize, RoaringBitmap> = input
                .iter()
                .map(|bitmap| {
                    RoaringBitmap::deserialize_from(Cursor::new(bitmap))
                        .unwrap_or(RoaringBitmap::new())
                })
                .enumerate()
                .collect();
            dbg!(
                bitmaps
                    .iter()
                    .map(|(_, bitmap)| bitmap.serialized_size())
                    .sum::<usize>()
            );
            let mut out_bitmaps = Vec::new();
            let first = *bitmaps
                .iter()
                .min_by_key(|(_token, bitmap)| bitmap.serialized_size())
                .unwrap()
                .0;
            let mut remaining: HashSet<usize> = bitmaps.keys().cloned().collect();
            remaining.remove(&first);
            out_bitmaps.push((first, bitmaps[&first].clone()));
            let mut last = bitmaps[&first].clone();
            while remaining.len() > 0 {
                if remaining.len() % 100 == 0 {
                    info!("Writing bitmap. Remaining: {}", remaining.len());
                }
                let last_clone = last.clone();
                let (best, best_bitmap) = {
                    let equals = bitmaps
                        .iter()
                        .filter(|(idx, _bitmap)| remaining.contains(idx))
                        .filter(|(_idx, bitmap)| bitmap == &&last)
                        .next();
                    if let Some(equals) = equals {
                        equals
                    } else {
                        bitmaps
                            .iter()
                            .filter(|(idx, _bitmap)| remaining.contains(idx))
                            .min_by_key(|(_idx, bitmap)| {
                                bitmap.bitxor(&last_clone).serialized_size()
                            })
                            .unwrap()
                    }
                };
                out_bitmaps.push((*best, best_bitmap.bitxor(last)));
                last = bitmaps[best].clone();
                remaining.remove(best);
            }
            let out_serialized: HashMap<usize, Vec<u8>> = out_bitmaps
                .iter()
                .map(|(idx, bitmap)| {
                    let mut buf = Vec::new();
                    bitmap.serialize_into(&mut buf).unwrap();
                    (*idx, buf)
                })
                .collect();

            let buf = rmp_serde::to_vec(&out_serialized)?;
            let mut writer = File::create(&out)?;
            writer.write_all(&buf)?;
        }
    }

    let handles = JOIN_HANDLES.get_or_init(|| Mutex::new(Vec::new()));
    while handles.lock().unwrap().len() > 0 {
        let handle = handles.lock().unwrap().swap_remove(0);
        handle.into_future().await?;
    }
    Ok(())
}
