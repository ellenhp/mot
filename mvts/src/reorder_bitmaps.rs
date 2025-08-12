use anyhow::Result;
use roaring::RoaringBitmap;
use std::ops::BitXor;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{Cursor, Read, Write},
    path::PathBuf,
};
use tracing::{debug, info};

pub struct ReorderBitmapsCommand {
    pub r#in: PathBuf,
    pub out: PathBuf,
}

impl ReorderBitmapsCommand {
    pub async fn run(&self) -> Result<()> {
        let mut input = Vec::new();
        File::open(&self.r#in)?.read_to_end(&mut input)?;
        let input: Vec<Vec<u8>> = rmp_serde::from_slice(&input)?;
        let bitmaps: HashMap<usize, RoaringBitmap> = input
            .iter()
            .map(|bitmap| {
                RoaringBitmap::deserialize_from(Cursor::new(bitmap)).unwrap_or(RoaringBitmap::new())
            })
            .enumerate()
            .collect();
        debug!(
            "Total serialized size: {}",
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
                        .min_by_key(|(_idx, bitmap)| bitmap.bitxor(&last_clone).serialized_size())
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
        let mut writer = File::create(&self.out)?;
        writer.write_all(&buf)?;
        Ok(())
    }
}
