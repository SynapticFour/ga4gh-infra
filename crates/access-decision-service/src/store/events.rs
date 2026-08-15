// SPDX-License-Identifier: Apache-2.0

use super::*;

impl AdsStore {
    pub fn webhook_urls(&self) -> &[String] {
        &self.webhook_urls
    }

    pub fn webhook_http(&self) -> &reqwest::Client {
        &self.http
    }

    pub async fn insert_event(&self, event: &AdsEvent) -> Result<(), AdsError> {
        let payload = serde_json::to_string(&event.payload).map_err(map_db_err)?;
        with_pool!(self, pool, {
            sqlx::query(
                "INSERT INTO audit_events (id, event_type, payload, occurred_at)
                     VALUES ($1, $2, $3, $4)",
            )
            .bind(event.id.to_string())
            .bind(event_type_str(&event.event_type))
            .bind(&payload)
            .bind(event.occurred_at.timestamp())
            .execute(pool)
            .await
            .map(|_| ())
            .map_err(map_db_err)
        })
    }

    pub async fn list_audit_events(
        &self,
        limit: u32,
        dac_groups: Option<&[String]>,
    ) -> Result<Vec<AdsEvent>, AdsError> {
        if empty_dac_filter(dac_groups) {
            return Ok(vec![]);
        }
        let mut events: Vec<AdsEvent> = match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT id, event_type, payload, occurred_at
                     FROM audit_events ORDER BY occurred_at DESC LIMIT $1",
                )
                .bind(limit as i64)
                .fetch_all(pool)
                .await
                .map_err(map_db_err)?;
                rows.into_iter()
                    .map(|row| -> Result<AdsEvent, AdsError> { Ok(parse_audit_event!(&row)) })
                    .collect()
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let rows = sqlx::query(
                    "SELECT id, event_type, payload, occurred_at
                     FROM audit_events ORDER BY occurred_at DESC LIMIT $1",
                )
                .bind(limit as i64)
                .fetch_all(pool)
                .await
                .map_err(map_db_err)?;
                rows.into_iter()
                    .map(|row| -> Result<AdsEvent, AdsError> { Ok(parse_audit_event!(&row)) })
                    .collect()
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }?;
        if let Some(groups) = dac_groups.filter(|g| !g.is_empty()) {
            events.retain(|event| {
                event
                    .payload
                    .get("dac_group")
                    .and_then(|v| v.as_str())
                    .is_some_and(|g| groups.iter().any(|allowed| allowed == g))
            });
        }
        Ok(events)
    }

    pub(crate) async fn ensure_researcher_exists(&self, id: &str) -> Result<(), AdsError> {
        let now = Utc::now();
        let researcher = Researcher {
            id: id.to_string(),
            display_name: None,
            email: None,
            affiliations: vec![],
            created_at: now,
            updated_at: now,
        };
        self.upsert_researcher(&researcher).await
    }
}
