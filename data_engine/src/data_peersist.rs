use anyhow::{Result};
use log::{debug,error, info};
use sqlx::{QueryBuilder, Postgres};
use crate::persistable::Persistable;

impl super::DataService {

    pub async fn persist_data<T: Persistable>(&self, items: &[T]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }

        let table = T::table_name();
        debug!("Persisting batch of {} items to {}", items.len(), table);

        // We chunk to avoid Postgres parameter limits (65,535)
        for (i, chunk) in items.chunks(1000).enumerate() {
            let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(format!(
                "INSERT INTO {} ({}) ", 
                table, 
                T::column_names()
            ));

            builder.push_values(chunk, |mut b, item| {
                item.bind_values(&mut b);
            });

            builder.push(format!(
                " ON CONFLICT ({}) DO UPDATE SET {}",
                T::conflict_keys(),
                T::update_columns()
            ));

            builder
                .build()
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    error!("Database error on table {} during chunk {}: {}", table, i, e);
                    anyhow::anyhow!("Failed to persist batch to {}: {}", table, e)
                })?;
        }

        info!("Successfully persisted {} items to {}", items.len(), table);
        Ok(())
    }

}