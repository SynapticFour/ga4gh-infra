// SPDX-License-Identifier: Apache-2.0

use super::*;

impl AdsStore {
    pub async fn create_dataset(&self, req: &CreateDatasetRequest) -> Result<Dataset, AdsError> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let duo_json = serde_json::to_string(&req.duo_codes).map_err(map_db_err)?;
        let dataset = Dataset {
            id,
            name: req.name.clone(),
            description: req.description.clone(),
            duo_codes: req.duo_codes.clone(),
            external_id: req.external_id.clone(),
            auto_approve_enabled: req.auto_approve_enabled,
            auto_approve_threshold: req.auto_approve_threshold,
            dac_group: req.dac_group.clone(),
            visibility: req.visibility,
            resource_type: req.resource_type,
            remote_drs_base_url: req.remote_drs_base_url.clone(),
            created_at: now,
            updated_at: now,
        };
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO datasets (id, name, description, duo_codes, external_id,
                     auto_approve_enabled, auto_approve_threshold, dac_group, visibility, resource_type, remote_drs_base_url, created_at, updated_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
                )
                .bind(dataset.id.to_string())
                .bind(&dataset.name)
                .bind(&dataset.description)
                .bind(&duo_json)
                .bind(&dataset.external_id)
                .bind(i64::from(dataset.auto_approve_enabled))
                .bind(i64::from(dataset.auto_approve_threshold))
                .bind(&dataset.dac_group)
                .bind(visibility_str(dataset.visibility))
                .bind(resource_type_str(dataset.resource_type))
                .bind(&dataset.remote_drs_base_url)
                .bind(dataset.created_at.timestamp())
                .bind(dataset.updated_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO datasets (id, name, description, duo_codes, external_id,
                     auto_approve_enabled, auto_approve_threshold, dac_group, visibility, resource_type, remote_drs_base_url, created_at, updated_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
                )
                .bind(dataset.id.to_string())
                .bind(&dataset.name)
                .bind(&dataset.description)
                .bind(&duo_json)
                .bind(&dataset.external_id)
                .bind(i64::from(dataset.auto_approve_enabled))
                .bind(i64::from(dataset.auto_approve_threshold))
                .bind(&dataset.dac_group)
                .bind(visibility_str(dataset.visibility))
                .bind(resource_type_str(dataset.resource_type))
                .bind(&dataset.remote_drs_base_url)
                .bind(dataset.created_at.timestamp())
                .bind(dataset.updated_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
        }
        Ok(dataset)
    }

    pub async fn update_dataset(
        &self,
        id: Uuid,
        req: &CreateDatasetRequest,
    ) -> Result<Dataset, AdsError> {
        let existing = self.get_dataset(id).await?;
        if req.duo_codes.is_empty() {
            return Err(AdsError::BadRequest(
                "duo_codes must not be empty".to_string(),
            ));
        }
        let now = Utc::now();
        let duo_json = serde_json::to_string(&req.duo_codes).map_err(map_db_err)?;
        let dataset = Dataset {
            id,
            name: req.name.clone(),
            description: req.description.clone(),
            duo_codes: req.duo_codes.clone(),
            external_id: req.external_id.clone(),
            auto_approve_enabled: req.auto_approve_enabled,
            auto_approve_threshold: req.auto_approve_threshold,
            dac_group: req.dac_group.clone(),
            visibility: req.visibility,
            resource_type: req.resource_type,
            remote_drs_base_url: req.remote_drs_base_url.clone(),
            created_at: existing.created_at,
            updated_at: now,
        };
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "UPDATE datasets SET name = $1, description = $2, duo_codes = $3, external_id = $4,
                     auto_approve_enabled = $5, auto_approve_threshold = $6, dac_group = $7, visibility = $8, resource_type = $9, remote_drs_base_url = $10, updated_at = $11
                     WHERE id = $12",
                )
                .bind(&dataset.name)
                .bind(&dataset.description)
                .bind(&duo_json)
                .bind(&dataset.external_id)
                .bind(i64::from(dataset.auto_approve_enabled))
                .bind(i64::from(dataset.auto_approve_threshold))
                .bind(&dataset.dac_group)
                .bind(visibility_str(dataset.visibility))
                .bind(resource_type_str(dataset.resource_type))
                .bind(&dataset.remote_drs_base_url)
                .bind(dataset.updated_at.timestamp())
                .bind(dataset.id.to_string())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "UPDATE datasets SET name = ?1, description = ?2, duo_codes = ?3, external_id = ?4,
                     auto_approve_enabled = ?5, auto_approve_threshold = ?6, dac_group = ?7, visibility = ?8, resource_type = ?9, remote_drs_base_url = ?10, updated_at = ?11
                     WHERE id = ?12",
                )
                .bind(&dataset.name)
                .bind(&dataset.description)
                .bind(&duo_json)
                .bind(&dataset.external_id)
                .bind(i64::from(dataset.auto_approve_enabled))
                .bind(i64::from(dataset.auto_approve_threshold))
                .bind(&dataset.dac_group)
                .bind(visibility_str(dataset.visibility))
                .bind(resource_type_str(dataset.resource_type))
                .bind(&dataset.remote_drs_base_url)
                .bind(dataset.updated_at.timestamp())
                .bind(dataset.id.to_string())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
            #[allow(unreachable_patterns)]
            _ => return Err(AdsError::Config("no database driver enabled".to_string())),
        }
        Ok(dataset)
    }

    pub async fn list_datasets(
        &self,
        dac_groups: Option<&[String]>,
    ) -> Result<Vec<Dataset>, AdsError> {
        if empty_dac_filter(dac_groups) {
            return Ok(vec![]);
        }
        let select = "SELECT id, name, description, duo_codes, external_id,
                            auto_approve_enabled, auto_approve_threshold, dac_group, visibility, resource_type, remote_drs_base_url, created_at, updated_at
                     FROM datasets";
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let rows = if let Some(groups) = dac_groups.filter(|g| !g.is_empty()) {
                    let placeholders: Vec<String> =
                        (1..=groups.len()).map(|i| format!("${i}")).collect();
                    let sql = format!(
                        "{select} WHERE dac_group IN ({}) ORDER BY created_at DESC",
                        placeholders.join(", ")
                    );
                    let mut query = sqlx::query(&sql);
                    for group in groups {
                        query = query.bind(group);
                    }
                    query.fetch_all(pool).await.map_err(map_db_err)?
                } else {
                    sqlx::query(&format!("{select} ORDER BY created_at DESC"))
                        .fetch_all(pool)
                        .await
                        .map_err(map_db_err)?
                };
                rows.into_iter()
                    .map(|row| -> Result<Dataset, AdsError> { Ok(parse_dataset!(&row)) })
                    .collect()
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let rows = if let Some(groups) = dac_groups.filter(|g| !g.is_empty()) {
                    let placeholders = std::iter::repeat_n("?", groups.len())
                        .collect::<Vec<_>>()
                        .join(", ");
                    let sql = format!(
                        "{select} WHERE dac_group IN ({placeholders}) ORDER BY created_at DESC"
                    );
                    let mut query = sqlx::query(&sql);
                    for group in groups {
                        query = query.bind(group);
                    }
                    query.fetch_all(pool).await.map_err(map_db_err)?
                } else {
                    sqlx::query(&format!("{select} ORDER BY created_at DESC"))
                        .fetch_all(pool)
                        .await
                        .map_err(map_db_err)?
                };
                rows.into_iter()
                    .map(|row| -> Result<Dataset, AdsError> { Ok(parse_dataset!(&row)) })
                    .collect()
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }

    /// Datasets visible in the researcher catalog (excludes `draft`).
    pub async fn list_catalog_datasets(
        &self,
        include_institute: bool,
        resource_type: Option<AdsResourceType>,
    ) -> Result<Vec<Dataset>, AdsError> {
        let select = "SELECT id, name, description, duo_codes, external_id,
                            auto_approve_enabled, auto_approve_threshold, dac_group, visibility, resource_type, remote_drs_base_url, created_at, updated_at
                     FROM datasets";
        let visibilities: Vec<&str> = if include_institute {
            vec!["public", "institute"]
        } else {
            vec!["public"]
        };
        let resource_type_str = resource_type.map(resource_type_str);
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let vis_placeholders: Vec<String> =
                    (1..=visibilities.len()).map(|i| format!("${i}")).collect();
                let bind_idx = visibilities.len() + 1;
                let mut sql = format!(
                    "{select} WHERE visibility IN ({})",
                    vis_placeholders.join(", ")
                );
                if resource_type_str.is_some() {
                    sql.push_str(&format!(" AND resource_type = ${bind_idx}"));
                }
                sql.push_str(" ORDER BY created_at DESC");
                let mut query = sqlx::query(&sql);
                for v in &visibilities {
                    query = query.bind(*v);
                }
                if let Some(rt) = resource_type_str.as_ref() {
                    query = query.bind(rt);
                }
                let rows = query.fetch_all(pool).await.map_err(map_db_err)?;
                rows.into_iter()
                    .map(|row| -> Result<Dataset, AdsError> { Ok(parse_dataset!(&row)) })
                    .collect()
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let vis_placeholders = std::iter::repeat_n("?", visibilities.len())
                    .collect::<Vec<_>>()
                    .join(", ");
                let mut sql = format!("{select} WHERE visibility IN ({vis_placeholders})");
                if resource_type_str.is_some() {
                    sql.push_str(" AND resource_type = ?");
                }
                sql.push_str(" ORDER BY created_at DESC");
                let mut query = sqlx::query(&sql);
                for v in &visibilities {
                    query = query.bind(*v);
                }
                if let Some(rt) = resource_type_str.as_ref() {
                    query = query.bind(rt);
                }
                let rows = query.fetch_all(pool).await.map_err(map_db_err)?;
                rows.into_iter()
                    .map(|row| -> Result<Dataset, AdsError> { Ok(parse_dataset!(&row)) })
                    .collect()
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }

    pub async fn get_dataset(&self, id: Uuid) -> Result<Dataset, AdsError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id, name, description, duo_codes, external_id,
                            auto_approve_enabled, auto_approve_threshold, dac_group, visibility, resource_type, remote_drs_base_url, created_at, updated_at
                     FROM datasets WHERE id = $1",
                )
                .bind(id.to_string())
                .fetch_optional(pool)
                .await
                .map_err(map_db_err)?;
                row.map(|row| -> Result<Dataset, AdsError> { Ok(parse_dataset!(&row)) })
                    .transpose()?
                    .ok_or(AdsError::NotFound)
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT id, name, description, duo_codes, external_id,
                            auto_approve_enabled, auto_approve_threshold, dac_group, visibility, resource_type, remote_drs_base_url, created_at, updated_at
                     FROM datasets WHERE id = $1",
                )
                .bind(id.to_string())
                .fetch_optional(pool)
                .await
                .map_err(map_db_err)?;
                row.map(|row| -> Result<Dataset, AdsError> { Ok(parse_dataset!(&row)) })
                    .transpose()?
                    .ok_or(AdsError::NotFound)
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }
}
