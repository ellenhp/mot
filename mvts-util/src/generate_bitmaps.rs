use anyhow::Result;
use futures_util::TryStreamExt;
use roaring::RoaringBitmap;
use sqlx::{PgPool, query};
use std::{collections::HashMap, fs::File, io::Write, path::PathBuf};
use tokenizers::Tokenizer;
use tracing::{debug, warn};

pub struct GenerateBitmapsCommand {
    pub db: String,
    pub out: PathBuf,
}

impl GenerateBitmapsCommand {
    pub async fn run(&self) -> Result<()> {
        let mut bitmaps: HashMap<u32, RoaringBitmap> = HashMap::new();
        let tokenizer =
            Tokenizer::from_pretrained("google-bert/bert-base-multilingual-uncased", None)
                .map_err(|err| anyhow::anyhow!("Failed to download tokenizer {}", err))?;
        let pool = PgPool::connect(&self.db).await?;
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
        let mut writer = File::create(&self.out)?;
        writer.write_all(&buf)?;
        Ok(())
    }
}
