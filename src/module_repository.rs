//! Tenant-scoped persistence for company module selection.
use hph_core::{ModuleId, ModuleSet};
use sqlx::types::Uuid;
use std::{error::Error, fmt};

use crate::database::Database;

/// Reads and updates module selection inside an RLS-scoped transaction.
///
/// IDs must come from a verified session and selected active membership.
/// Writes additionally require the membership's admin role.
pub struct CompanyModuleRepository<'a> {
    database: &'a Database,
}

#[derive(Debug)]
pub enum CompanyModuleError {
    Database(sqlx::Error),
    Unauthorized,
    InvalidConfiguration(String),
}

impl fmt::Display for CompanyModuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(_) => f.write_str("Could not access company modules."),
            Self::Unauthorized => f.write_str("Company membership is not authorized."),
            Self::InvalidConfiguration(_) => f.write_str("Invalid module configuration."),
        }
    }
}

impl Error for CompanyModuleError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            Self::Unauthorized | Self::InvalidConfiguration(_) => None,
        }
    }
}

impl From<sqlx::Error> for CompanyModuleError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

impl<'a> CompanyModuleRepository<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    /// Return the saved selection, or None when the company has not saved one.
    pub async fn load(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Option<ModuleSet>, CompanyModuleError> {
        let mut transaction = self
            .database
            .begin_tenant_transaction(user_id, tenant_id)
            .await?;

        let active: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM hph_memberships \
             WHERE organization_id = $1 AND user_id = $2 AND active)",
        )
        .bind(tenant_id)
        .bind(user_id)
        .fetch_one(&mut *transaction)
        .await?;
        if !active {
            return Err(CompanyModuleError::Unauthorized);
        }

        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT module_id FROM hph_company_modules \
             WHERE organization_id = $1 AND enabled ORDER BY module_id",
        )
        .bind(tenant_id)
        .fetch_all(&mut *transaction)
        .await?;

        let result = if rows.is_empty() {
            let configured: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM hph_company_modules WHERE organization_id = $1)",
            )
            .bind(tenant_id)
            .fetch_one(&mut *transaction)
            .await?;
            if configured {
                Some(
                    ModuleSet::from_enabled(vec![])
                        .map_err(CompanyModuleError::InvalidConfiguration)?,
                )
            } else {
                None
            }
        } else {
            let enabled = rows
                .iter()
                .map(|id| match id.as_str() {
                    "people" => Ok(ModuleId::People),
                    "attendance" => Ok(ModuleId::Attendance),
                    "reports" => Ok(ModuleId::Reports),
                    _ => Err(CompanyModuleError::InvalidConfiguration(
                        "Unknown module ID.".into(),
                    )),
                })
                .collect::<Result<Vec<_>, _>>()?;
            Some(
                ModuleSet::from_enabled(enabled)
                    .map_err(CompanyModuleError::InvalidConfiguration)?,
            )
        };

        transaction.commit().await?;
        Ok(result)
    }

    /// Atomically persist every module flag after validating dependencies and admin access.
    pub async fn save(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        modules: &ModuleSet,
    ) -> Result<(), CompanyModuleError> {
        let checked = ModuleSet::from_enabled(modules.enabled.clone())
            .map_err(CompanyModuleError::InvalidConfiguration)?;
        let mut transaction = self
            .database
            .begin_tenant_transaction(user_id, tenant_id)
            .await?;

        let admin: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM hph_memberships \
             WHERE organization_id = $1 AND user_id = $2 AND active AND role = 'admin')",
        )
        .bind(tenant_id)
        .bind(user_id)
        .fetch_one(&mut *transaction)
        .await?;
        if !admin {
            return Err(CompanyModuleError::Unauthorized);
        }

        for (id, name) in [
            (ModuleId::People, "people"),
            (ModuleId::Attendance, "attendance"),
            (ModuleId::Reports, "reports"),
        ] {
            sqlx::query(
                "INSERT INTO hph_company_modules (organization_id, module_id, enabled) \
                 VALUES ($1, $2, $3) \
                 ON CONFLICT (organization_id, module_id) DO UPDATE \
                 SET enabled = EXCLUDED.enabled, updated_at = now()",
            )
            .bind(tenant_id)
            .bind(name)
            .bind(checked.contains(id))
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;

    /// Runs with a non-owner runtime role in CI; fixture setup uses a separate admin pool.
    #[tokio::test]
    async fn company_module_selection_is_tenant_scoped_and_admin_only() {
        let Ok(database_url) = std::env::var("HPH_TEST_DATABASE_URL") else {
            return;
        };
        let admin_url =
            std::env::var("HPH_TEST_ADMIN_DATABASE_URL").unwrap_or_else(|_| database_url.clone());

        let admin_database = Database::connect(&admin_url).await.unwrap();
        admin_database.migrate().await.unwrap();
        let runtime_database = Database::connect(&database_url).await.unwrap();
        let slug: String =
            sqlx::query_scalar("SELECT 'repo-test-' || substr(md5(random()::text), 1, 16)")
                .fetch_one(admin_database.pool())
                .await
                .unwrap();
        let tenant_id: Uuid = sqlx::query_scalar(
            "INSERT INTO hph_organizations (slug, name) VALUES ($1, 'Repository test') RETURNING id",
        )
        .bind(&slug)
        .fetch_one(admin_database.pool())
        .await
        .unwrap();
        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO hph_users (display_name) VALUES ('Repository test user') RETURNING id",
        )
        .fetch_one(admin_database.pool())
        .await
        .unwrap();
        let person_id: Uuid = sqlx::query_scalar(
            "INSERT INTO hph_people (organization_id, employee_number, display_name) \
             VALUES ($1, 'repo-test', 'Repository test user') RETURNING id",
        )
        .bind(tenant_id)
        .fetch_one(admin_database.pool())
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO hph_memberships (organization_id, user_id, person_id, role) \
             VALUES ($1, $2, $3, 'admin')",
        )
        .bind(tenant_id)
        .bind(user_id)
        .bind(person_id)
        .execute(admin_database.pool())
        .await
        .unwrap();

        let repository = CompanyModuleRepository::new(&runtime_database);
        assert!(repository.load(user_id, tenant_id).await.unwrap().is_none());
        let selected = ModuleSet::preset("complete").unwrap();
        repository
            .save(user_id, tenant_id, &selected)
            .await
            .unwrap();
        let loaded = repository.load(user_id, tenant_id).await.unwrap().unwrap();
        assert!(loaded.contains(ModuleId::Reports));

        sqlx::query(
            "UPDATE hph_memberships SET role = 'employee' \
             WHERE organization_id = $1 AND user_id = $2",
        )
        .bind(tenant_id)
        .bind(user_id)
        .execute(admin_database.pool())
        .await
        .unwrap();
        assert!(matches!(
            repository.save(user_id, tenant_id, &selected).await,
            Err(CompanyModuleError::Unauthorized)
        ));

        sqlx::query("DELETE FROM hph_organizations WHERE id = $1")
            .bind(tenant_id)
            .execute(admin_database.pool())
            .await
            .unwrap();
        sqlx::query("DELETE FROM hph_users WHERE id = $1")
            .bind(user_id)
            .execute(admin_database.pool())
            .await
            .unwrap();
        runtime_database.close().await;
        admin_database.close().await;
    }
}
