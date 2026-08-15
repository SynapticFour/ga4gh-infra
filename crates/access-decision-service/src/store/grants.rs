// SPDX-License-Identifier: Apache-2.0

use super::*;

impl AdsStore {
    pub async fn insert_grant(&self, grant: &Grant) -> Result<(), AdsError> {
        let duo_json = serde_json::to_string(&grant.duo_codes).map_err(map_db_err)?;
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO grants (id, researcher_id, dataset_id, request_id, source, duo_codes,
                     resource_scope, expires_at, revoked_at, created_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                )
                .bind(grant.id.to_string())
                .bind(&grant.researcher_id)
                .bind(grant.dataset_id.to_string())
                .bind(grant.request_id.map(|id| id.to_string()))
                .bind(grant_source_str(&grant.source))
                .bind(&duo_json)
                .bind(&grant.resource_scope)
                .bind(grant.expires_at.map(|dt| dt.timestamp()))
                .bind(grant.revoked_at.map(|dt| dt.timestamp()))
                .bind(grant.created_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO grants (id, researcher_id, dataset_id, request_id, source, duo_codes,
                     resource_scope, expires_at, revoked_at, created_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                )
                .bind(grant.id.to_string())
                .bind(&grant.researcher_id)
                .bind(grant.dataset_id.to_string())
                .bind(grant.request_id.map(|id| id.to_string()))
                .bind(grant_source_str(&grant.source))
                .bind(&duo_json)
                .bind(&grant.resource_scope)
                .bind(grant.expires_at.map(|dt| dt.timestamp()))
                .bind(grant.revoked_at.map(|dt| dt.timestamp()))
                .bind(grant.created_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
        }
        Ok(())
    }

    pub async fn list_grants(
        &self,
        researcher_id: Option<&str>,
        dac_groups: Option<&[String]>,
    ) -> Result<Vec<Grant>, AdsError> {
        if researcher_id.is_none() && empty_dac_filter(dac_groups) {
            return Ok(vec![]);
        }
        let grant_cols = "g.id, g.researcher_id, g.dataset_id, g.request_id, g.source, g.duo_codes,
                                g.resource_scope, g.expires_at, g.revoked_at, g.created_at";
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let rows = match (researcher_id, dac_groups.filter(|g| !g.is_empty())) {
                    (Some(sub), Some(groups)) => {
                        let placeholders: Vec<String> =
                            (2..=groups.len() + 1).map(|i| format!("${i}")).collect();
                        let sql = format!(
                            "SELECT {grant_cols} FROM grants g
                             INNER JOIN datasets d ON g.dataset_id = d.id
                             WHERE g.researcher_id = $1 AND g.revoked_at IS NULL
                             AND d.dac_group IN ({})",
                            placeholders.join(", ")
                        );
                        let mut query = sqlx::query(&sql).bind(sub);
                        for group in groups {
                            query = query.bind(group);
                        }
                        query.fetch_all(pool).await.map_err(map_db_err)?
                    }
                    (Some(sub), None) => sqlx::query(&format!(
                        "SELECT {grant_cols} FROM grants g
                             WHERE g.researcher_id = $1 AND g.revoked_at IS NULL"
                    ))
                    .bind(sub)
                    .fetch_all(pool)
                    .await
                    .map_err(map_db_err)?,
                    (None, Some(groups)) => {
                        let placeholders: Vec<String> =
                            (1..=groups.len()).map(|i| format!("${i}")).collect();
                        let sql = format!(
                            "SELECT {grant_cols} FROM grants g
                             INNER JOIN datasets d ON g.dataset_id = d.id
                             WHERE g.revoked_at IS NULL AND d.dac_group IN ({})",
                            placeholders.join(", ")
                        );
                        let mut query = sqlx::query(&sql);
                        for group in groups {
                            query = query.bind(group);
                        }
                        query.fetch_all(pool).await.map_err(map_db_err)?
                    }
                    (None, None) => sqlx::query(&format!(
                        "SELECT {grant_cols} FROM grants g WHERE g.revoked_at IS NULL"
                    ))
                    .fetch_all(pool)
                    .await
                    .map_err(map_db_err)?,
                };
                rows.into_iter()
                    .map(|row| -> Result<Grant, AdsError> { Ok(parse_grant!(&row)) })
                    .collect()
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let rows = match (researcher_id, dac_groups.filter(|g| !g.is_empty())) {
                    (Some(sub), Some(groups)) => {
                        let placeholders = std::iter::repeat_n("?", groups.len())
                            .collect::<Vec<_>>()
                            .join(", ");
                        let sql = format!(
                            "SELECT {grant_cols} FROM grants g
                             INNER JOIN datasets d ON g.dataset_id = d.id
                             WHERE g.researcher_id = ? AND g.revoked_at IS NULL
                             AND d.dac_group IN ({placeholders})"
                        );
                        let mut query = sqlx::query(&sql).bind(sub);
                        for group in groups {
                            query = query.bind(group);
                        }
                        query.fetch_all(pool).await.map_err(map_db_err)?
                    }
                    (Some(sub), None) => sqlx::query(&format!(
                        "SELECT {grant_cols} FROM grants g
                             WHERE g.researcher_id = ? AND g.revoked_at IS NULL"
                    ))
                    .bind(sub)
                    .fetch_all(pool)
                    .await
                    .map_err(map_db_err)?,
                    (None, Some(groups)) => {
                        let placeholders = std::iter::repeat_n("?", groups.len())
                            .collect::<Vec<_>>()
                            .join(", ");
                        let sql = format!(
                            "SELECT {grant_cols} FROM grants g
                             INNER JOIN datasets d ON g.dataset_id = d.id
                             WHERE g.revoked_at IS NULL AND d.dac_group IN ({placeholders})"
                        );
                        let mut query = sqlx::query(&sql);
                        for group in groups {
                            query = query.bind(group);
                        }
                        query.fetch_all(pool).await.map_err(map_db_err)?
                    }
                    (None, None) => sqlx::query(&format!(
                        "SELECT {grant_cols} FROM grants g WHERE g.revoked_at IS NULL"
                    ))
                    .fetch_all(pool)
                    .await
                    .map_err(map_db_err)?,
                };
                rows.into_iter()
                    .map(|row| -> Result<Grant, AdsError> { Ok(parse_grant!(&row)) })
                    .collect()
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }

    pub async fn get_grant(&self, id: Uuid) -> Result<Grant, AdsError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id, researcher_id, dataset_id, request_id, source, duo_codes,
                            resource_scope, expires_at, revoked_at, created_at
                     FROM grants WHERE id = $1",
                )
                .bind(id.to_string())
                .fetch_optional(pool)
                .await
                .map_err(map_db_err)?;
                row.map(|row| -> Result<Grant, AdsError> { Ok(parse_grant!(&row)) })
                    .transpose()?
                    .ok_or(AdsError::NotFound)
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT id, researcher_id, dataset_id, request_id, source, duo_codes,
                            resource_scope, expires_at, revoked_at, created_at
                     FROM grants WHERE id = $1",
                )
                .bind(id.to_string())
                .fetch_optional(pool)
                .await
                .map_err(map_db_err)?;
                row.map(|row| -> Result<Grant, AdsError> { Ok(parse_grant!(&row)) })
                    .transpose()?
                    .ok_or(AdsError::NotFound)
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }

    pub async fn revoke_grant(&self, id: Uuid) -> Result<Grant, AdsError> {
        let mut grant = self.get_grant(id).await?;
        if grant.revoked_at.is_some() {
            return Err(AdsError::Conflict("grant already revoked".to_string()));
        }
        grant.revoked_at = Some(Utc::now());
        let revoked_at = grant.revoked_at.unwrap().timestamp();
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query("UPDATE grants SET revoked_at = $1 WHERE id = $2")
                    .bind(revoked_at)
                    .bind(id.to_string())
                    .execute(pool)
                    .await
                    .map_err(map_db_err)?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query("UPDATE grants SET revoked_at = $1 WHERE id = $2")
                    .bind(revoked_at)
                    .bind(id.to_string())
                    .execute(pool)
                    .await
                    .map_err(map_db_err)?;
            }
        }
        grant_revoked(self, &grant).await?;
        Ok(grant)
    }

    pub async fn active_grants_for_resource(
        &self,
        researcher_id: &str,
        dataset_id: Option<Uuid>,
        resource: &str,
    ) -> Result<Vec<Grant>, AdsError> {
        let grants = self.list_grants(Some(researcher_id), None).await?;
        Ok(grants
            .into_iter()
            .filter(|g| {
                if g.revoked_at.is_some() {
                    return false;
                }
                if let Some(exp) = g.expires_at {
                    if exp <= Utc::now() {
                        return false;
                    }
                }
                if let Some(ds) = dataset_id {
                    if g.dataset_id != ds {
                        return false;
                    }
                }
                if let Some(scope) = &g.resource_scope {
                    let bare = scope.strip_prefix("drs:").unwrap_or(scope.as_str());
                    return scope == resource
                        || resource.contains(scope.as_str())
                        || resource == bare
                        || format!("drs:{resource}") == *scope;
                }
                true
            })
            .collect())
    }
}
