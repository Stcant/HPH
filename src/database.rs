use std::{path::Path, time::Duration};

use sqlx::{
    PgPool, Postgres, Transaction,
    migrate::{MigrateError, Migrator},
    postgres::PgPoolOptions,
    types::Uuid,
};

/// PostgreSQL connection pool shared by server-side application state.
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Open a bounded pool. The URL is read from the environment by the caller
    /// and is never logged here.
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .acquire_timeout(Duration::from_secs(5))
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }

    /// Apply the repository migrations using the configured migration connection.
    pub async fn migrate(&self) -> Result<(), MigrateError> {
        Migrator::new(Path::new("migrations"))
            .await?
            .run(&self.pool)
            .await
    }

    /// Check that PostgreSQL is reachable without exposing connection details.
    pub async fn ping(&self) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map(|_| ())
    }

    /// Start a transaction scoped to a previously verified user and organization.
    ///
    /// Callers must verify an active membership before creating this context.
    /// Both settings are transaction-local so they cannot leak to a later request
    /// that reuses the same pooled connection.
    pub async fn begin_tenant_transaction(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Transaction<'_, Postgres>, sqlx::Error> {
        let mut transaction = self.pool.begin().await?;
        sqlx::query(
            "SELECT set_config('hph.user_id', $1, true), set_config('hph.tenant_id', $2, true)",
        )
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .execute(&mut *transaction)
        .await?;

        Ok(transaction)
    }

    /// Operational requests must never bypass company RLS or own application tables.
    pub async fn verify_runtime_role(&self) -> Result<(), sqlx::Error> {
        let unsafe_role: bool = sqlx::query_scalar(
            "SELECT r.rolsuper OR r.rolbypassrls OR EXISTS (SELECT 1 FROM pg_tables WHERE schemaname='public' AND tablename LIKE 'hph_%' AND tableowner=current_user) FROM pg_roles r WHERE r.rolname=current_user"
        ).fetch_one(&self.pool).await?;
        if unsafe_role {
            return Err(sqlx::Error::Protocol(
                "Use a non-owner PostgreSQL runtime role without SUPERUSER or BYPASSRLS".into(),
            ));
        }
        Ok(())
    }

    pub(crate) fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Close all pooled connections before process shutdown.
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The dedicated CI database exercises the real SQLx migration and context path.
    /// This test returns early outside that job, where no PostgreSQL service is configured.
    #[tokio::test]
    async fn sqlx_pool_migrates_and_scopes_tenant_context() {
        let Ok(database_url) = std::env::var("HPH_TEST_DATABASE_URL") else {
            return;
        };

        let database = Database::connect(&database_url).await.unwrap();
        database.migrate().await.unwrap();
        database.migrate().await.unwrap();
        database.ping().await.unwrap();

        let user_id = Uuid::parse_str("20000000-0000-4000-8000-000000000001").unwrap();
        let tenant_id = Uuid::parse_str("10000000-0000-4000-8000-000000000001").unwrap();
        let mut transaction = database
            .begin_tenant_transaction(user_id, tenant_id)
            .await
            .unwrap();

        let tenant_context: Option<Uuid> = sqlx::query_scalar("SELECT hph_current_tenant_id()")
            .fetch_one(&mut *transaction)
            .await
            .unwrap();
        let user_context: Option<Uuid> = sqlx::query_scalar("SELECT hph_current_user_id()")
            .fetch_one(&mut *transaction)
            .await
            .unwrap();
        assert_eq!(tenant_context, Some(tenant_id));
        assert_eq!(user_context, Some(user_id));

        transaction.commit().await.unwrap();

        let tenant_after_commit: Option<Uuid> =
            sqlx::query_scalar("SELECT hph_current_tenant_id()")
                .fetch_one(&database.pool)
                .await
                .unwrap();
        assert_eq!(tenant_after_commit, None);
        database.close().await;
    }
}
