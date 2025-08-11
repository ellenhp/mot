use std::sync::Mutex;

use crate::{
    sync_txn::JOIN_HANDLES,
    whosonfirst::{PipWithGeometry, WhosOnFirst},
};
use anyhow::Ok;
use sqlx::{Postgres, Transaction, query};
use tokio::spawn;
use tracing::info;

/// Horrifying hack to commit a transaction on drop, which we need because we use this in a `async move` closure.
pub struct ProcessAdmin {
    inner: Option<Transaction<'static, Postgres>>,
}

impl ProcessAdmin {
    pub async fn new(
        mut txn: Transaction<'static, Postgres>,
    ) -> Result<ProcessAdmin, anyhow::Error> {
        query!("DELETE FROM wof_admins").execute(&mut *txn).await?;

        Ok(ProcessAdmin { inner: Some(txn) })
    }

    pub async fn process_admin(
        &mut self,
        wof: &WhosOnFirst,
        row: &PipWithGeometry,
    ) -> Result<(), anyhow::Error> {
        let row_id = row.id.parse::<u64>().map_err(|err| anyhow::anyhow!(err))?;
        let place_names = wof.place_name_by_id(row_id).await?;
        let geom = row
            .geom
            .geometry
            .clone()
            .map(|geom| geozero::wkb::Encode(geom));
        let query = sqlx::query(
            "INSERT INTO wof_admins (id, geom, admin_level, names) VALUES($1, ST_SetSRID($2, 4326), $3, $4::jsonb)",
        );
        let txn = self.inner.iter_mut().next().unwrap();
        query
            .bind(row_id as i64)
            .bind(geom)
            .bind(row.r#type.clone())
            .bind(serde_json::to_string(&place_names)?)
            .execute(&mut **txn)
            .await?;
        Ok(())
    }
}

impl Drop for ProcessAdmin {
    fn drop(&mut self) {
        let txn = self.inner.take().unwrap();
        JOIN_HANDLES
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .unwrap()
            .push(spawn(async move {
                txn.commit().await.expect("Failed to commit transaction");
                info!("Committed transaction");
            }));
    }
}
