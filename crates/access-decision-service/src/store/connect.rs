// SPDX-License-Identifier: Apache-2.0

use super::*;

impl AdsStore {
    pub async fn connect(
        database: &DatabaseConfig,
        url: &str,
        webhook_urls: Vec<String>,
    ) -> Result<Self, AdsError> {
        Self::connect_with_pepper(database, url, webhook_urls, String::new()).await
    }

    pub async fn connect_with_pepper(
        database: &DatabaseConfig,
        url: &str,
        webhook_urls: Vec<String>,
        api_key_pepper: String,
    ) -> Result<Self, AdsError> {
        match database.driver {
            #[cfg(feature = "postgres")]
            DatabaseDriver::Postgres => {
                let pool = PgPool::connect(url).await.map_err(map_db_err)?;
                if database.auto_migrate {
                    sqlx::migrate!().run(&pool).await.map_err(map_db_err)?;
                }
                Ok(Self {
                    pool: DbPool::Postgres(pool),
                    webhook_urls: Arc::new(webhook_urls),
                    http: webhook_http_client()?,
                    api_key_pepper,
                })
            }
            #[cfg(feature = "sqlite")]
            DatabaseDriver::Sqlite => Self::connect_sqlite(url, webhook_urls, api_key_pepper).await,
            #[cfg(not(feature = "postgres"))]
            DatabaseDriver::Postgres => Err(AdsError::Config(
                "ADS was built without the `postgres` feature".to_string(),
            )),
            #[cfg(not(feature = "sqlite"))]
            DatabaseDriver::Sqlite => Err(AdsError::Config(
                "ADS was built without the `sqlite` feature".to_string(),
            )),
        }
    }

    #[cfg(feature = "sqlite")]
    async fn connect_sqlite(
        url: &str,
        webhook_urls: Vec<String>,
        api_key_pepper: String,
    ) -> Result<Self, AdsError> {
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
        use std::str::FromStr;

        let options = SqliteConnectOptions::from_str(url)
            .map_err(|err| AdsError::Database(format!("invalid SQLite URL: {err}")))?
            .create_if_missing(true);

        if !options.get_filename().as_os_str().is_empty() {
            if let Some(parent) = options.get_filename().parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|err| {
                        AdsError::Database(format!(
                            "creating SQLite directory `{}`: {err}",
                            parent.display()
                        ))
                    })?;
                }
            }
        }

        let pool = SqlitePoolOptions::new()
            .connect_with(options)
            .await
            .map_err(map_db_err)?;
        sqlx::migrate!().run(&pool).await.map_err(map_db_err)?;
        Ok(Self {
            pool: DbPool::Sqlite(pool),
            webhook_urls: Arc::new(webhook_urls),
            http: webhook_http_client()?,
            api_key_pepper,
        })
    }

    pub async fn ensure_bootstrap_api_key(
        &self,
        raw_key: &str,
        name: &str,
    ) -> Result<(), AdsError> {
        if self.count_active_api_keys().await? > 0 {
            return Ok(());
        }
        self.insert_api_key(name, raw_key).await
    }

    async fn count_active_api_keys(&self) -> Result<i64, AdsError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let row =
                    sqlx::query("SELECT COUNT(*) AS count FROM api_keys WHERE revoked_at IS NULL")
                        .fetch_one(pool)
                        .await
                        .map_err(map_db_err)?;
                Ok(row.get::<i64, _>("count"))
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let row =
                    sqlx::query("SELECT COUNT(*) AS count FROM api_keys WHERE revoked_at IS NULL")
                        .fetch_one(pool)
                        .await
                        .map_err(map_db_err)?;
                Ok(row.get::<i64, _>("count"))
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }

    async fn insert_api_key(&self, name: &str, raw_key: &str) -> Result<(), AdsError> {
        let id = Uuid::new_v4().to_string();
        let key_hash = ga4gh_http::hash_api_key(raw_key, &self.api_key_pepper);
        let now = unix_now();
        with_pool!(self, pool, {
            sqlx::query(
                "INSERT INTO api_keys (id, name, key_hash, created_at) VALUES ($1, $2, $3, $4)",
            )
            .bind(&id)
            .bind(name)
            .bind(&key_hash)
            .bind(now)
            .execute(pool)
            .await
            .map(|_| ())
            .map_err(map_db_err)
        })
    }

    pub async fn verify_api_key(&self, raw_key: &str) -> Result<Option<String>, AdsError> {
        for key_hash in ga4gh_http::lookup_hashes(raw_key, &self.api_key_pepper) {
            let name = with_pool!(self, pool, {
                sqlx::query("SELECT name FROM api_keys WHERE key_hash = $1 AND revoked_at IS NULL")
                    .bind(&key_hash)
                    .fetch_optional(pool)
                    .await
                    .map_err(map_db_err)
                    .map(|row| row.map(|row| row.get("name")))
            })?;
            if name.is_some() {
                return Ok(name);
            }
        }
        Ok(None)
    }
}
