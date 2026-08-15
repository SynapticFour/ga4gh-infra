// SPDX-License-Identifier: Apache-2.0

use super::*;

impl AdsStore {
    #[instrument(skip(self, body, evaluation))]
    pub async fn create_access_request(
        &self,
        body: &CreateAccessRequestBody,
        evaluation: Option<DuoEvaluationResult>,
    ) -> Result<AccessRequest, AdsError> {
        self.ensure_researcher_exists(&body.researcher_id).await?;
        let dataset = self.get_dataset(body.dataset_id).await?;
        let project = self.get_project(body.project_id).await?;
        if project.researcher_id != body.researcher_id {
            return Err(AdsError::BadRequest(
                "project does not belong to researcher".to_string(),
            ));
        }

        let id = Uuid::new_v4();
        let now = Utc::now();
        let eval_json = evaluation
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(map_db_err)?;

        let mut status = AccessRequestStatus::Pending;
        if dataset.auto_approve_enabled && evaluation.as_ref().is_some_and(|e| e.auto_approvable) {
            status = AccessRequestStatus::Approved;
        }

        let request = AccessRequest {
            id,
            researcher_id: body.researcher_id.clone(),
            dataset_id: body.dataset_id,
            project_id: body.project_id,
            status,
            justification: body.justification.clone(),
            duo_evaluation: evaluation,
            dac_group: dataset.dac_group.clone(),
            created_at: now,
            updated_at: now,
        };

        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO access_requests (id, researcher_id, dataset_id, project_id, status,
                     justification, duo_evaluation, dac_group, created_at, updated_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                )
                .bind(request.id.to_string())
                .bind(&request.researcher_id)
                .bind(request.dataset_id.to_string())
                .bind(request.project_id.to_string())
                .bind(status_str(request.status))
                .bind(&request.justification)
                .bind(eval_json)
                .bind(&request.dac_group)
                .bind(request.created_at.timestamp())
                .bind(request.updated_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO access_requests (id, researcher_id, dataset_id, project_id, status,
                     justification, duo_evaluation, dac_group, created_at, updated_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                )
                .bind(request.id.to_string())
                .bind(&request.researcher_id)
                .bind(request.dataset_id.to_string())
                .bind(request.project_id.to_string())
                .bind(status_str(request.status))
                .bind(&request.justification)
                .bind(eval_json)
                .bind(&request.dac_group)
                .bind(request.created_at.timestamp())
                .bind(request.updated_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
        }

        request_created(
            self,
            request.id,
            &request.researcher_id,
            request.dataset_id,
            request.dac_group.as_deref(),
        )
        .await?;

        if request.status == AccessRequestStatus::Approved {
            self.record_decision(
                request.id,
                AccessDecisionOutcome::Approved,
                "system:duo-auto",
                Some("automatic DUO approval".to_string()),
            )
            .await?;
            self.create_grant_from_request(&request, GrantSource::DuoAutoApproval)
                .await?;
            request_approved(self, &request, "system:duo-auto").await?;
        }

        Ok(request)
    }

    pub async fn get_access_request(&self, id: Uuid) -> Result<AccessRequest, AdsError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id, researcher_id, dataset_id, project_id, status, justification,
                            duo_evaluation, dac_group, created_at, updated_at
                     FROM access_requests WHERE id = $1",
                )
                .bind(id.to_string())
                .fetch_optional(pool)
                .await
                .map_err(map_db_err)?;
                row.map(|row| -> Result<AccessRequest, AdsError> {
                    Ok(parse_access_request!(&row))
                })
                .transpose()?
                .ok_or(AdsError::NotFound)
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT id, researcher_id, dataset_id, project_id, status, justification,
                            duo_evaluation, dac_group, created_at, updated_at
                     FROM access_requests WHERE id = $1",
                )
                .bind(id.to_string())
                .fetch_optional(pool)
                .await
                .map_err(map_db_err)?;
                row.map(|row| -> Result<AccessRequest, AdsError> {
                    Ok(parse_access_request!(&row))
                })
                .transpose()?
                .ok_or(AdsError::NotFound)
            }
            #[allow(unreachable_patterns)]
            _ => Err(AdsError::Config("no database driver enabled".to_string())),
        }
    }

    pub async fn list_dac_requests(
        &self,
        dac_groups: Option<&[String]>,
    ) -> Result<Vec<AccessRequest>, AdsError> {
        if empty_dac_filter(dac_groups) {
            return Ok(vec![]);
        }
        let select = "SELECT id, researcher_id, dataset_id, project_id, status, justification,
                            duo_evaluation, dac_group, created_at, updated_at
                     FROM access_requests
                     WHERE status IN ('pending', 'escalated')";
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let rows = if let Some(groups) = dac_groups.filter(|g| !g.is_empty()) {
                    let placeholders: Vec<String> =
                        (1..=groups.len()).map(|i| format!("${i}")).collect();
                    let sql = format!(
                        "{select} AND dac_group IN ({}) ORDER BY created_at ASC",
                        placeholders.join(", ")
                    );
                    let mut query = sqlx::query(&sql);
                    for group in groups {
                        query = query.bind(group);
                    }
                    query.fetch_all(pool).await.map_err(map_db_err)?
                } else {
                    sqlx::query(&format!("{select} ORDER BY created_at ASC"))
                        .fetch_all(pool)
                        .await
                        .map_err(map_db_err)?
                };
                rows.into_iter()
                    .map(|row| -> Result<AccessRequest, AdsError> {
                        Ok(parse_access_request!(&row))
                    })
                    .collect()
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let rows = if let Some(groups) = dac_groups.filter(|g| !g.is_empty()) {
                    let placeholders = std::iter::repeat_n("?", groups.len())
                        .collect::<Vec<_>>()
                        .join(", ");
                    let sql = format!(
                        "{select} AND dac_group IN ({placeholders}) ORDER BY created_at ASC"
                    );
                    let mut query = sqlx::query(&sql);
                    for group in groups {
                        query = query.bind(group);
                    }
                    query.fetch_all(pool).await.map_err(map_db_err)?
                } else {
                    sqlx::query(&format!("{select} ORDER BY created_at ASC"))
                        .fetch_all(pool)
                        .await
                        .map_err(map_db_err)?
                };
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

    pub async fn dac_approve(
        &self,
        id: Uuid,
        actor: &str,
        reason: Option<String>,
    ) -> Result<AccessRequest, AdsError> {
        let mut request = self.get_access_request(id).await?;
        if !matches!(
            request.status,
            AccessRequestStatus::Pending | AccessRequestStatus::Escalated
        ) {
            return Err(AdsError::Conflict(format!(
                "request is {:?}, not reviewable",
                request.status
            )));
        }
        self.record_decision(id, AccessDecisionOutcome::Approved, actor, reason)
            .await?;
        request.status = AccessRequestStatus::Approved;
        request.updated_at = Utc::now();
        self.update_request_status(&request).await?;
        self.create_grant_from_request(&request, GrantSource::DacApproval)
            .await?;
        request_approved(self, &request, actor).await?;
        Ok(request)
    }

    pub async fn dac_reject(
        &self,
        id: Uuid,
        actor: &str,
        reason: Option<String>,
    ) -> Result<AccessRequest, AdsError> {
        let mut request = self.get_access_request(id).await?;
        if !matches!(
            request.status,
            AccessRequestStatus::Pending | AccessRequestStatus::Escalated
        ) {
            return Err(AdsError::Conflict(format!(
                "request is {:?}, not reviewable",
                request.status
            )));
        }
        self.record_decision(id, AccessDecisionOutcome::Rejected, actor, reason)
            .await?;
        request.status = AccessRequestStatus::Rejected;
        request.updated_at = Utc::now();
        self.update_request_status(&request).await?;
        request_rejected(self, &request, actor).await?;
        Ok(request)
    }

    pub async fn dac_escalate(
        &self,
        id: Uuid,
        actor: &str,
        reason: Option<String>,
    ) -> Result<AccessRequest, AdsError> {
        let mut request = self.get_access_request(id).await?;
        if request.status != AccessRequestStatus::Pending {
            return Err(AdsError::Conflict(format!(
                "request is {:?}, cannot escalate",
                request.status
            )));
        }
        self.record_decision(id, AccessDecisionOutcome::Escalated, actor, reason)
            .await?;
        request.status = AccessRequestStatus::Escalated;
        request.updated_at = Utc::now();
        self.update_request_status(&request).await?;
        Ok(request)
    }

    async fn record_decision(
        &self,
        request_id: Uuid,
        outcome: AccessDecisionOutcome,
        actor: &str,
        reason: Option<String>,
    ) -> Result<AccessDecision, AdsError> {
        let decision = AccessDecision {
            id: Uuid::new_v4(),
            request_id,
            outcome,
            actor: actor.to_string(),
            reason,
            decided_at: Utc::now(),
        };
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO access_decisions (id, request_id, outcome, actor, reason, decided_at)
                     VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind(decision.id.to_string())
                .bind(decision.request_id.to_string())
                .bind(outcome_str(decision.outcome))
                .bind(&decision.actor)
                .bind(&decision.reason)
                .bind(decision.decided_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO access_decisions (id, request_id, outcome, actor, reason, decided_at)
                     VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind(decision.id.to_string())
                .bind(decision.request_id.to_string())
                .bind(outcome_str(decision.outcome))
                .bind(&decision.actor)
                .bind(&decision.reason)
                .bind(decision.decided_at.timestamp())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
        }
        Ok(decision)
    }

    async fn update_request_status(&self, request: &AccessRequest) -> Result<(), AdsError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "UPDATE access_requests SET status = $1, updated_at = $2 WHERE id = $3",
                )
                .bind(status_str(request.status))
                .bind(request.updated_at.timestamp())
                .bind(request.id.to_string())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "UPDATE access_requests SET status = $1, updated_at = $2 WHERE id = $3",
                )
                .bind(status_str(request.status))
                .bind(request.updated_at.timestamp())
                .bind(request.id.to_string())
                .execute(pool)
                .await
                .map_err(map_db_err)?;
            }
        }
        Ok(())
    }

    async fn create_grant_from_request(
        &self,
        request: &AccessRequest,
        source: GrantSource,
    ) -> Result<Grant, AdsError> {
        let dataset = self.get_dataset(request.dataset_id).await?;
        let grant = Grant {
            id: Uuid::new_v4(),
            researcher_id: request.researcher_id.clone(),
            dataset_id: request.dataset_id,
            request_id: Some(request.id),
            source,
            duo_codes: dataset.duo_codes.clone(),
            resource_scope: dataset.external_id.clone(),
            expires_at: None,
            revoked_at: None,
            created_at: Utc::now(),
        };
        self.insert_grant(&grant).await?;
        grant_created(
            self,
            grant.id,
            &grant.researcher_id,
            grant.dataset_id,
            dataset.dac_group.as_deref(),
        )
        .await?;
        Ok(grant)
    }
}
