use std::{
    collections::HashMap,
    fs::File,
    io::{Cursor, Read},
    path::PathBuf,
};

use clap::Parser;
use roaring::RoaringBitmap;
use std::ops::BitAnd;
use tokenizers::Tokenizer;
use tracing::debug;

#[derive(Debug, Parser)]
pub struct QueryBitmaps {
    r#in: PathBuf,
    query: String,
}

impl QueryBitmaps {
    pub fn query(&self) -> Result<(), anyhow::Error> {
        let mut input: Vec<u8> = Vec::new();
        File::open(&self.r#in)?.read_to_end(&mut input)?;
        let input: Vec<Vec<u8>> = rmp_serde::from_slice(&input)?;
        let bitmaps: HashMap<usize, RoaringBitmap> = input
            .iter()
            .map(|bitmap| {
                RoaringBitmap::deserialize_from(Cursor::new(bitmap)).unwrap_or(RoaringBitmap::new())
            })
            .enumerate()
            .collect();

        let zoom = f32::log(bitmaps.len() as f32, 4.0) as u32;

        debug!("Bitmaps correspond to zoom level {zoom}");

        let tokenizer =
            Tokenizer::from_pretrained("google-bert/bert-base-multilingual-uncased", None)
                .map_err(|err| anyhow::anyhow!("Failed to download tokenizer {}", err))?;

        let query = self.query.clone();
        let encoded = tokenizer
            .encode(query, false)
            .map_err(|err| anyhow::anyhow!("Failed to tokenize {}", err))?;
        let queried_bitmaps: Vec<&RoaringBitmap> = encoded
            .get_ids()
            .iter()
            .map(|id| bitmaps.get(&(*id as usize)).unwrap())
            .collect();
        let mut possible_tiles = RoaringBitmap::full();
        for bitmap in &queried_bitmaps {
            possible_tiles = possible_tiles.bitand(*bitmap);
        }
        dbg!(possible_tiles);
        Ok(())
    }
}
