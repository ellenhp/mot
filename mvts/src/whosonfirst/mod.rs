use anyhow::Result;
use futures_util::stream::TryStreamExt;
use serde::{Deserialize, Serialize};
use sqlx::{
    Pool, Sqlite,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use std::path::Path;
use tracing::trace;

/// Performs simple point-in-polygon queries against the WhosOnFirst database.
/// Queries against the WOF database require the SQLite mod_spatialite extension to be loaded.
/// Requires package: libsqlite3-mod-spatialite on Debian/Ubuntu
#[derive(Clone)]
pub struct WhosOnFirst {
    pool: Pool<Sqlite>,
}

impl WhosOnFirst {
    /// Opens a connection to the WhosOnFirst database.
    /// Requires the SQLite mod_spatialite extension to be loaded.
    pub async fn new(path: &Path) -> Result<Self> {
        trace!("Opening WhosOnFirst database at {:?}", path);

        let opts = SqliteConnectOptions::new()
            .filename(path)
            .journal_mode(SqliteJournalMode::Wal)
            .pragma("cache_size", "2000")
            .pragma("synchronous", "OFF")
            .pragma("temp_store", "MEMORY")
            .pragma("foreign_keys", "OFF")
            .pragma("recursive_triggers", "OFF")
            .pragma("locking_mode", "NORMAL")
            .extension("mod_spatialite");

        // Connections with the total number of physical and virtual cores.
        // The sqlx pool isn't the most efficient, so keep it busy.
        let connections = num_cpus::get().try_into()?;

        let pool = SqlitePoolOptions::new()
            .min_connections(connections)
            .max_connections(connections)
            .connect_with(opts)
            .await?;

        Ok(Self { pool })
    }

    /// Lookup the name of a place by its WOF ID.
    pub async fn place_name_by_id(&self, id: u64) -> Result<Vec<PipPlaceName>> {
        // Convert to i64 for SQLite
        let id: i64 = id.try_into()?;

        // Index for name is on (source, id)
        let rows = sqlx::query_as::<_, PipPlaceName>(
            r"
                SELECT name.lang, name.tag, name.abbr, name.name
                FROM main.name
                WHERE name.source = 'wof'
                AND name.id = ?1
                AND name.tag IN ('preferred', 'default')
                AND name.lang IN (
                    'ara', 'dan', 'deu', 'fra', 'fin', 'hun', 'gre', 'ita', 'nld', 'por',
                    'rus', 'ron', 'spa', 'eng', 'swe', 'tam', 'tur', 'zho'
                )
            ",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Performs an action for each polygon in the db, without materializing the list in RAM.
    pub async fn for_polygon<F: AsyncFnMut(PipWithGeometry) -> ()>(
        &self,
        mut action: F,
    ) -> Result<()> {
        // Geometry is stored as spatialite blob, so decode to WKB (geopackage compatible).
        let mut rows = sqlx::query_as::<_, PipWithGeometry>(
            r"
                SELECT
                    place.source,
                    place.id,
                    place.class,
                    place.type,
                    AsGPB(shard.geom) as geom
                FROM shard
                LEFT JOIN place USING (source, id)
                WHERE place.source IS NOT NULL
                AND (
                    place.type != 'planet'
                    AND place.type != 'marketarea'
                    AND place.type != 'county'
                    AND place.type != 'timezone'
                )
            ",
        )
        .fetch(&self.pool);
        while let Some(row) = rows.try_next().await? {
            action(row).await;
        }

        Ok(())
    }
}

/// A concise representation of a place in the WhosOnFirst database.
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct ConcisePipResponse {
    /// WOF data source, usually wof
    pub source: String,

    /// WOF ID of the place
    pub id: String,

    /// High level bucket of human activity - https://whosonfirst.org/docs/categories/
    /// POINT-OF-VIEW > CLASS > CATEGORY
    pub class: String,

    pub r#type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PipPlaceName {
    pub lang: String,
    pub tag: String,
    // #[allow(dead_code)]
    // pub abbr: bool,
    pub name: String,
}

/// Represents a place in the WhosOnFirst database with a geometry.
#[derive(sqlx::FromRow)]
pub struct PipWithGeometry {
    /// WOF data source, usually wof
    pub source: String,

    /// WOF ID of the place
    pub id: String,

    /// High level bucket of human activity - https://whosonfirst.org/docs/categories/
    /// POINT-OF-VIEW > CLASS > CATEGORY
    pub class: String,

    pub r#type: String,

    pub geom: geozero::wkb::Decode<geo_types::Geometry<f64>>,
}

/// Deconstruct a PipWithGeometry into a geometry and a concise response.
impl From<PipWithGeometry> for (Option<geo_types::Geometry<f64>>, ConcisePipResponse) {
    fn from(value: PipWithGeometry) -> Self {
        (
            value.geom.geometry,
            ConcisePipResponse {
                source: value.source,
                id: value.id,
                class: value.class,
                r#type: value.r#type,
            },
        )
    }
}
