// SPDX-License-Identifier: Apache-2.0

use super::*;

impl AdsStore {
    pub async fn create_project(
        &self,
        req: &CreateProjectRequest,
    ) -> Result<ResearchProject, AdsError> {
        self.ensure_researcher_exists(&req.researcher_id).await?;
        let id = Uuid::new_v4();
        let now = Utc::now();
        let duo_json = serde_json::to_string(&req.duo_codes).map_err(map_db_err)?;
        let project = ResearchProject {
            id,
            researcher_id: req.researcher_id.clone(),
            name: req.name.clone(),
            description: req.description.clone(),
            duo_codes: req.duo_codes.clone(),
            created_at: now,
            updated_at: now,
        };
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO research_projects (id, researcher_id, name, description, duo_codes, created_at, updated_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7)",
                )
                .bind(project.id.to_string())
                .bind(&project.researcher_id)
                .bind(&project.name)
                .bind(&project.description)
                .bind(&duo_json)
                .bind(project.created_at.timestamp())
                .bind(project.updated_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO research_projects (id, researcher_id, name, description, duo_codes, created_at, updated_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7)",
                )
                .bind(project.id.to_string())
                .bind(&project.researcher_id)
                .bind(&project.name)
                .bind(&project.description)
                .bind(&duo_json)
                .bind(project.created_at.timestamp())
                .bind(project.updated_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
        }
        Ok(project)
    }

    pub async fn list_projects(&self) -> Result<Vec<ResearchProject>, AdsError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT id, researcher_id, name, description, duo_codes, created_at, updated_at
                     FROM research_projects ORDER BY created_at DESC",
                )
                .fetch_all(pool)
                .await
                .map_err(map_db_err)?;
                rows.into_iter()
                    .map(|row| -> Result<ResearchProject, AdsError> { Ok(parse_project!(&row)) })
                    .collect()
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let rows = sqlx::query(
                    "SELECT id, researcher_id, name, description, duo_codes, created_at, updated_at
                     FROM research_projects ORDER BY created_at DESC",
                )
                .fetch_all(pool)
                .await
                .map_err(map_db_err)?;
                rows.into_iter()
                    .map(|row| -> Result<ResearchProject, AdsError> { Ok(parse_project!(&row)) })
                    .collect()
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }

    pub async fn list_projects_for_researcher(
        &self,
        researcher_id: &str,
    ) -> Result<Vec<ResearchProject>, AdsError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT id, researcher_id, name, description, duo_codes, created_at, updated_at
                     FROM research_projects WHERE researcher_id = $1 ORDER BY created_at DESC",
                )
                .bind(researcher_id)
                .fetch_all(pool)
                .await
                .map_err(map_db_err)?;
                rows.into_iter()
                    .map(|row| -> Result<ResearchProject, AdsError> { Ok(parse_project!(&row)) })
                    .collect()
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let rows = sqlx::query(
                    "SELECT id, researcher_id, name, description, duo_codes, created_at, updated_at
                     FROM research_projects WHERE researcher_id = ? ORDER BY created_at DESC",
                )
                .bind(researcher_id)
                .fetch_all(pool)
                .await
                .map_err(map_db_err)?;
                rows.into_iter()
                    .map(|row| -> Result<ResearchProject, AdsError> { Ok(parse_project!(&row)) })
                    .collect()
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }

    pub async fn list_access_requests_for_researcher(
        &self,
        researcher_id: &str,
    ) -> Result<Vec<AccessRequest>, AdsError> {
        let select = "SELECT id, researcher_id, dataset_id, project_id, status, justification,
                            duo_evaluation, dac_group, created_at, updated_at
                     FROM access_requests WHERE researcher_id = $1 ORDER BY created_at DESC";
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let rows = sqlx::query(select)
                    .bind(researcher_id)
                    .fetch_all(pool)
                    .await
                    .map_err(map_db_err)?;
                rows.into_iter()
                    .map(|row| -> Result<AccessRequest, AdsError> {
                        Ok(parse_access_request!(&row))
                    })
                    .collect()
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let select_sqlite = select.replace("$1", "?");
                let rows = sqlx::query(&select_sqlite)
                    .bind(researcher_id)
                    .fetch_all(pool)
                    .await
                    .map_err(map_db_err)?;
                rows.into_iter()
                    .map(|row| -> Result<AccessRequest, AdsError> {
                        Ok(parse_access_request!(&row))
                    })
                    .collect()
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }

    pub async fn get_project(&self, id: Uuid) -> Result<ResearchProject, AdsError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id, researcher_id, name, description, duo_codes, created_at, updated_at
                     FROM research_projects WHERE id = $1",
                )
                .bind(id.to_string())
                .fetch_optional(pool)
                .await
                .map_err(map_db_err)?;
                row.map(|row| -> Result<ResearchProject, AdsError> { Ok(parse_project!(&row)) })
                    .transpose()?
                    .ok_or(AdsError::NotFound)
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT id, researcher_id, name, description, duo_codes, created_at, updated_at
                     FROM research_projects WHERE id = $1",
                )
                .bind(id.to_string())
                .fetch_optional(pool)
                .await
                .map_err(map_db_err)?;
                row.map(|row| -> Result<ResearchProject, AdsError> { Ok(parse_project!(&row)) })
                    .transpose()?
                    .ok_or(AdsError::NotFound)
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }

    pub async fn update_project(
        &self,
        id: Uuid,
        req: &CreateProjectRequest,
    ) -> Result<ResearchProject, AdsError> {
        let existing = self.get_project(id).await?;
        if req.duo_codes.is_empty() {
            return Err(AdsError::BadRequest(
                "duo_codes must not be empty".to_string(),
            ));
        }
        let now = Utc::now();
        let duo_json = serde_json::to_string(&req.duo_codes).map_err(map_db_err)?;
        let project = ResearchProject {
            id,
            researcher_id: existing.researcher_id,
            name: req.name.clone(),
            description: req.description.clone(),
            duo_codes: req.duo_codes.clone(),
            created_at: existing.created_at,
            updated_at: now,
        };
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "UPDATE research_projects SET name = $1, description = $2, duo_codes = $3, updated_at = $4
                     WHERE id = $5",
                )
                .bind(&project.name)
                .bind(&project.description)
                .bind(&duo_json)
                .bind(project.updated_at.timestamp())
                .bind(project.id.to_string())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "UPDATE research_projects SET name = ?1, description = ?2, duo_codes = ?3, updated_at = ?4
                     WHERE id = ?5",
                )
                .bind(&project.name)
                .bind(&project.description)
                .bind(&duo_json)
                .bind(project.updated_at.timestamp())
                .bind(project.id.to_string())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
            #[allow(unreachable_patterns)]
            _ => return Err(AdsError::Config("no database driver enabled".to_string())),
        }
        Ok(project)
    }
}
