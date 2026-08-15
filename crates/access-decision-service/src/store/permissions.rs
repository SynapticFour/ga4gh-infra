// SPDX-License-Identifier: Apache-2.0

use super::*;

impl AdsStore {
    pub async fn create_visa_source(
        &self,
        req: &CreateVisaSourceRequest,
    ) -> Result<VisaSource, AdsError> {
        let source = VisaSource {
            id: Uuid::new_v4(),
            name: req.name.clone(),
            issuer_url: req.issuer_url.clone(),
            visa_type: req.visa_type.clone(),
            enabled: req.enabled,
            created_at: Utc::now(),
        };
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO visa_sources (id, name, issuer_url, visa_type, enabled, created_at)
                     VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind(source.id.to_string())
                .bind(&source.name)
                .bind(&source.issuer_url)
                .bind(source.visa_type.to_string())
                .bind(i64::from(source.enabled))
                .bind(source.created_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO visa_sources (id, name, issuer_url, visa_type, enabled, created_at)
                     VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind(source.id.to_string())
                .bind(&source.name)
                .bind(&source.issuer_url)
                .bind(source.visa_type.to_string())
                .bind(i64::from(source.enabled))
                .bind(source.created_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
        }
        Ok(source)
    }

    pub async fn create_permission_source(
        &self,
        req: &CreatePermissionSourceRequest,
    ) -> Result<PermissionSource, AdsError> {
        let source = PermissionSource {
            id: Uuid::new_v4(),
            name: req.name.clone(),
            oidc_issuer: req.oidc_issuer.clone(),
            claim_path: req.claim_path.clone(),
            created_at: Utc::now(),
        };
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO permission_sources (id, name, oidc_issuer, claim_path, created_at)
                     VALUES ($1, $2, $3, $4, $5)",
                )
                .bind(source.id.to_string())
                .bind(&source.name)
                .bind(&source.oidc_issuer)
                .bind(&source.claim_path)
                .bind(source.created_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO permission_sources (id, name, oidc_issuer, claim_path, created_at)
                     VALUES ($1, $2, $3, $4, $5)",
                )
                .bind(source.id.to_string())
                .bind(&source.name)
                .bind(&source.oidc_issuer)
                .bind(&source.claim_path)
                .bind(source.created_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
        }
        Ok(source)
    }

    pub async fn create_permission_mapping(
        &self,
        req: &CreatePermissionMappingRequest,
    ) -> Result<PermissionMapping, AdsError> {
        let _ = self.get_dataset(req.dataset_id).await?;
        let mapping = PermissionMapping {
            id: Uuid::new_v4(),
            source_id: req.source_id,
            claim_value: req.claim_value.clone(),
            dataset_id: req.dataset_id,
            grant_lifetime_seconds: req.grant_lifetime_seconds,
            created_at: Utc::now(),
        };
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO permission_mappings (id, source_id, claim_value, dataset_id,
                     grant_lifetime_seconds, created_at)
                     VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind(mapping.id.to_string())
                .bind(mapping.source_id.to_string())
                .bind(&mapping.claim_value)
                .bind(mapping.dataset_id.to_string())
                .bind(mapping.grant_lifetime_seconds.map(|v| v as i64))
                .bind(mapping.created_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO permission_mappings (id, source_id, claim_value, dataset_id,
                     grant_lifetime_seconds, created_at)
                     VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind(mapping.id.to_string())
                .bind(mapping.source_id.to_string())
                .bind(&mapping.claim_value)
                .bind(mapping.dataset_id.to_string())
                .bind(mapping.grant_lifetime_seconds.map(|v| v as i64))
                .bind(mapping.created_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
        }
        Ok(mapping)
    }

    pub async fn list_permission_sources(&self) -> Result<Vec<PermissionSource>, AdsError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT id, name, oidc_issuer, claim_path, created_at
                     FROM permission_sources ORDER BY created_at DESC",
                )
                .fetch_all(pool)
                .await
                .map_err(map_db_err)?;
                rows.into_iter()
                    .map(|row| -> Result<PermissionSource, AdsError> {
                        Ok(PermissionSource {
                            id: Uuid::parse_str(
                                &row.try_get::<String, _>("id").map_err(map_db_err)?,
                            )
                            .map_err(map_db_err)?,
                            name: row.try_get("name").map_err(map_db_err)?,
                            oidc_issuer: row.try_get("oidc_issuer").map_err(map_db_err)?,
                            claim_path: row.try_get("claim_path").map_err(map_db_err)?,
                            created_at: chrono::DateTime::from_timestamp(
                                row.try_get::<i64, _>("created_at").map_err(map_db_err)?,
                                0,
                            )
                            .ok_or_else(|| {
                                AdsError::Internal("invalid permission source timestamp".into())
                            })?
                            .with_timezone(&Utc),
                        })
                    })
                    .collect()
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let rows = sqlx::query(
                    "SELECT id, name, oidc_issuer, claim_path, created_at
                     FROM permission_sources ORDER BY created_at DESC",
                )
                .fetch_all(pool)
                .await
                .map_err(map_db_err)?;
                rows.into_iter()
                    .map(|row| -> Result<PermissionSource, AdsError> {
                        Ok(PermissionSource {
                            id: Uuid::parse_str(
                                &row.try_get::<String, _>("id").map_err(map_db_err)?,
                            )
                            .map_err(map_db_err)?,
                            name: row.try_get("name").map_err(map_db_err)?,
                            oidc_issuer: row.try_get("oidc_issuer").map_err(map_db_err)?,
                            claim_path: row.try_get("claim_path").map_err(map_db_err)?,
                            created_at: chrono::DateTime::from_timestamp(
                                row.try_get::<i64, _>("created_at").map_err(map_db_err)?,
                                0,
                            )
                            .ok_or_else(|| {
                                AdsError::Internal("invalid permission source timestamp".into())
                            })?
                            .with_timezone(&Utc),
                        })
                    })
                    .collect()
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }

    pub async fn list_permission_mappings(&self) -> Result<Vec<PermissionMapping>, AdsError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT id, source_id, claim_value, dataset_id, grant_lifetime_seconds, created_at
                     FROM permission_mappings ORDER BY created_at DESC",
                )
                .fetch_all(pool)
                .await
                .map_err(map_db_err)?;
                rows.into_iter()
                    .map(|row| -> Result<PermissionMapping, AdsError> {
                        Ok(PermissionMapping {
                            id: Uuid::parse_str(
                                &row.try_get::<String, _>("id").map_err(map_db_err)?,
                            )
                            .map_err(map_db_err)?,
                            source_id: Uuid::parse_str(
                                &row.try_get::<String, _>("source_id").map_err(map_db_err)?,
                            )
                            .map_err(map_db_err)?,
                            claim_value: row.try_get("claim_value").map_err(map_db_err)?,
                            dataset_id: Uuid::parse_str(
                                &row.try_get::<String, _>("dataset_id").map_err(map_db_err)?,
                            )
                            .map_err(map_db_err)?,
                            grant_lifetime_seconds: row
                                .try_get::<Option<i64>, _>("grant_lifetime_seconds")
                                .map_err(map_db_err)?
                                .map(|value| value as u64),
                            created_at: chrono::DateTime::from_timestamp(
                                row.try_get::<i64, _>("created_at").map_err(map_db_err)?,
                                0,
                            )
                            .ok_or_else(|| {
                                AdsError::Internal("invalid permission mapping timestamp".into())
                            })?
                            .with_timezone(&Utc),
                        })
                    })
                    .collect()
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let rows = sqlx::query(
                    "SELECT id, source_id, claim_value, dataset_id, grant_lifetime_seconds, created_at
                     FROM permission_mappings ORDER BY created_at DESC",
                )
                .fetch_all(pool)
                .await
                .map_err(map_db_err)?;
                rows.into_iter()
                    .map(|row| -> Result<PermissionMapping, AdsError> {
                        Ok(PermissionMapping {
                            id: Uuid::parse_str(
                                &row.try_get::<String, _>("id").map_err(map_db_err)?,
                            )
                            .map_err(map_db_err)?,
                            source_id: Uuid::parse_str(
                                &row.try_get::<String, _>("source_id").map_err(map_db_err)?,
                            )
                            .map_err(map_db_err)?,
                            claim_value: row.try_get("claim_value").map_err(map_db_err)?,
                            dataset_id: Uuid::parse_str(
                                &row.try_get::<String, _>("dataset_id").map_err(map_db_err)?,
                            )
                            .map_err(map_db_err)?,
                            grant_lifetime_seconds: row
                                .try_get::<Option<i64>, _>("grant_lifetime_seconds")
                                .map_err(map_db_err)?
                                .map(|value| value as u64),
                            created_at: chrono::DateTime::from_timestamp(
                                row.try_get::<i64, _>("created_at").map_err(map_db_err)?,
                                0,
                            )
                            .ok_or_else(|| {
                                AdsError::Internal("invalid permission mapping timestamp".into())
                            })?
                            .with_timezone(&Utc),
                        })
                    })
                    .collect()
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }

    pub async fn delete_permission_mapping(&self, id: Uuid) -> Result<(), AdsError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let result = sqlx::query("DELETE FROM permission_mappings WHERE id = $1")
                    .bind(id.to_string())
                    .execute(pool)
                    .await
                    .map_err(map_db_err)?;
                if result.rows_affected() == 0 {
                    return Err(AdsError::NotFound);
                }
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let result = sqlx::query("DELETE FROM permission_mappings WHERE id = $1")
                    .bind(id.to_string())
                    .execute(pool)
                    .await
                    .map_err(map_db_err)?;
                if result.rows_affected() == 0 {
                    return Err(AdsError::NotFound);
                }
            }
            #[allow(unreachable_patterns)]
            _ => return Err(AdsError::Config("no database driver enabled".to_string())),
        }
        Ok(())
    }

    pub async fn apply_institutional_mappings(
        &self,
        researcher_id: &str,
        claims: &BTreeMap<String, serde_json::Value>,
    ) -> Result<Vec<Grant>, AdsError> {
        use crate::permissions::{claim_values, grant_from_mapping};

        let mappings = self.list_active_permission_mappings().await?;
        let mut existing: HashSet<Uuid> = self
            .list_grants(Some(researcher_id), None)
            .await?
            .into_iter()
            .map(|grant| grant.dataset_id)
            .collect();
        let mut dataset_cache: HashMap<Uuid, Dataset> = HashMap::new();
        let mut created = Vec::new();
        for mapping in mappings {
            let values = claim_values(claims, &mapping.claim_path);
            if !values.iter().any(|value| value == &mapping.claim_value) {
                continue;
            }
            if existing.contains(&mapping.dataset_id) {
                continue;
            }
            let dataset = if let Some(dataset) = dataset_cache.get(&mapping.dataset_id) {
                dataset.clone()
            } else {
                let dataset = self.get_dataset(mapping.dataset_id).await?;
                dataset_cache.insert(mapping.dataset_id, dataset.clone());
                dataset
            };
            let grant = grant_from_mapping(
                researcher_id,
                mapping.dataset_id,
                dataset.duo_codes.clone(),
                dataset.external_id.clone(),
                mapping.grant_lifetime_seconds,
            );
            self.insert_grant(&grant).await?;
            grant_created(
                self,
                grant.id,
                researcher_id,
                grant.dataset_id,
                dataset.dac_group.as_deref(),
            )
            .await?;
            existing.insert(grant.dataset_id);
            created.push(grant);
        }
        Ok(created)
    }

    async fn list_active_permission_mappings(
        &self,
    ) -> Result<Vec<ActivePermissionMapping>, AdsError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT ps.claim_path, pm.claim_value, pm.dataset_id, pm.grant_lifetime_seconds
                     FROM permission_mappings pm
                     JOIN permission_sources ps ON ps.id = pm.source_id",
                )
                .fetch_all(pool)
                .await
                .map_err(map_db_err)?;
                rows.into_iter()
                    .map(|row| -> Result<ActivePermissionMapping, AdsError> {
                        Ok(ActivePermissionMapping {
                            claim_path: row.try_get("claim_path").map_err(map_db_err)?,
                            claim_value: row.try_get("claim_value").map_err(map_db_err)?,
                            dataset_id: Uuid::parse_str(
                                &row.try_get::<String, _>("dataset_id").map_err(map_db_err)?,
                            )
                            .map_err(map_db_err)?,
                            grant_lifetime_seconds: row
                                .try_get::<Option<i64>, _>("grant_lifetime_seconds")
                                .map_err(map_db_err)?
                                .map(|value| value as u64),
                        })
                    })
                    .collect()
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let rows = sqlx::query(
                    "SELECT ps.claim_path, pm.claim_value, pm.dataset_id, pm.grant_lifetime_seconds
                     FROM permission_mappings pm
                     JOIN permission_sources ps ON ps.id = pm.source_id",
                )
                .fetch_all(pool)
                .await
                .map_err(map_db_err)?;
                rows.into_iter()
                    .map(|row| -> Result<ActivePermissionMapping, AdsError> {
                        Ok(ActivePermissionMapping {
                            claim_path: row.try_get("claim_path").map_err(map_db_err)?,
                            claim_value: row.try_get("claim_value").map_err(map_db_err)?,
                            dataset_id: Uuid::parse_str(
                                &row.try_get::<String, _>("dataset_id").map_err(map_db_err)?,
                            )
                            .map_err(map_db_err)?,
                            grant_lifetime_seconds: row
                                .try_get::<Option<i64>, _>("grant_lifetime_seconds")
                                .map_err(map_db_err)?
                                .map(|value| value as u64),
                        })
                    })
                    .collect()
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }
}
