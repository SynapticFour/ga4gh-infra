// SPDX-License-Identifier: Apache-2.0

use super::*;

impl AdsStore {
    pub async fn upsert_researcher(&self, researcher: &Researcher) -> Result<(), AdsError> {
        let affiliations = serde_json::to_string(&researcher.affiliations).map_err(map_db_err)?;
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO researchers (id, display_name, email, affiliations, created_at, updated_at)
                     VALUES ($1, $2, $3, $4, $5, $6)
                     ON CONFLICT(id) DO UPDATE SET
                       display_name = EXCLUDED.display_name,
                       email = EXCLUDED.email,
                       affiliations = EXCLUDED.affiliations,
                       updated_at = EXCLUDED.updated_at",
                )
                .bind(&researcher.id)
                .bind(&researcher.display_name)
                .bind(&researcher.email)
                .bind(&affiliations)
                .bind(researcher.created_at.timestamp())
                .bind(researcher.updated_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO researchers (id, display_name, email, affiliations, created_at, updated_at)
                     VALUES ($1, $2, $3, $4, $5, $6)
                     ON CONFLICT(id) DO UPDATE SET
                       display_name = excluded.display_name,
                       email = excluded.email,
                       affiliations = excluded.affiliations,
                       updated_at = excluded.updated_at",
                )
                .bind(&researcher.id)
                .bind(&researcher.display_name)
                .bind(&researcher.email)
                .bind(&affiliations)
                .bind(researcher.created_at.timestamp())
                .bind(researcher.updated_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
        }
        Ok(())
    }

    pub async fn get_researcher(&self, id: &str) -> Result<Researcher, AdsError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id, display_name, email, affiliations, created_at, updated_at
                     FROM researchers WHERE id = $1",
                )
                .bind(id)
                .fetch_optional(pool)
                .await
                .map_err(map_db_err)?;
                row.map(|row| -> Result<Researcher, AdsError> { Ok(parse_researcher!(&row)) })
                    .transpose()?
                    .ok_or(AdsError::NotFound)
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT id, display_name, email, affiliations, created_at, updated_at
                     FROM researchers WHERE id = $1",
                )
                .bind(id)
                .fetch_optional(pool)
                .await
                .map_err(map_db_err)?;
                row.map(|row| -> Result<Researcher, AdsError> { Ok(parse_researcher!(&row)) })
                    .transpose()?
                    .ok_or(AdsError::NotFound)
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }
}
