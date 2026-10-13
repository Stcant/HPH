//! Tenant-scoped lookup of an authenticated user's active company membership.
use hph_core::{Actor, Role};
use sqlx::types::Uuid;
use std::{error::Error, fmt};

use crate::database::Database;

pub struct MembershipRepository<'a> {
    database: &'a Database,
}

#[derive(Debug)]
pub enum MembershipError {
    Database(sqlx::Error),
    Unauthorized,
}

impl fmt::Display for MembershipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(_) => f.write_str("Errore durante la verifica dell'appartenenza aziendale."),
            Self::Unauthorized => f.write_str("Nessuna appartenenza attiva per l'azienda selezionata."),
        }
    }
}

impl Error for MembershipError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            Self::Unauthorized => None,
        }
    }
}

impl From<sqlx::Error> for MembershipError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

impl<'a> MembershipRepository<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    /// Resolve an actor from a previously authenticated user and an explicitly selected tenant.
    ///
    /// The IDs must come from a verified session and validated tenant selection; this method
    /// checks the active membership again inside the transaction that establishes RLS context.
    pub async fn resolve_actor(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Actor, MembershipError> {
        let mut transaction = self
            .database
            .begin_tenant_transaction(user_id, tenant_id)
            .await?;

        let membership: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT person_id, role FROM hph_memberships \
             WHERE organization_id = $1 AND user_id = $2 AND active",
        )
        .bind(tenant_id)
        .bind(user_id)
        .fetch_optional(&mut *transaction)
        .await?;

        let Some((person_id, role)) = membership else {
            return Err(MembershipError::Unauthorized);
        };
        let role = match role.as_str() {
            "admin" => Role::Admin,
            "employee" => Role::Employee,
            _ => return Err(MembershipError::Unauthorized),
        };

        transaction.commit().await?;
        Ok(Actor {
            tenant_id: tenant_id.to_string(),
            person_id: person_id.to_string(),
            role,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;

    /// CI runs this using a non-owner role with RLS; admin is used only for fixtures.
    #[tokio::test]
    async fn actor_resolution_uses_only_active_membership_in_selected_tenant() {
        let Ok(database_url) = std::env::var("HPH_TEST_DATABASE_URL") else {
            return;
        };
        let admin_url =
            std::env::var("HPH_TEST_ADMIN_DATABASE_URL").unwrap_or_else(|_| database_url.clone());

        let admin = Database::connect(&admin_url).await.unwrap();
        admin.migrate().await.unwrap();
        let runtime = Database::connect(&database_url).await.unwrap();

        let slug_a: String =
            sqlx::query_scalar("SELECT 'membership-a-' || substr(md5(random()::text), 1, 16)")
                .fetch_one(admin.pool())
                .await
                .unwrap();
        let slug_b: String =
            sqlx::query_scalar("SELECT 'membership-b-' || substr(md5(random()::text), 1, 16)")
                .fetch_one(admin.pool())
                .await
                .unwrap();
        let tenant_a: Uuid = sqlx::query_scalar(
            "INSERT INTO hph_organizations (slug, name) VALUES ($1, 'Membership A') RETURNING id",
        )
        .bind(slug_a)
        .fetch_one(admin.pool())
        .await
        .unwrap();
        let tenant_b: Uuid = sqlx::query_scalar(
            "INSERT INTO hph_organizations (slug, name) VALUES ($1, 'Membership B') RETURNING id",
        )
        .bind(slug_b)
        .fetch_one(admin.pool())
        .await
        .unwrap();
        let user: Uuid = sqlx::query_scalar(
            "INSERT INTO hph_users (display_name) VALUES ('Membership test user') RETURNING id",
        )
        .fetch_one(admin.pool())
        .await
        .unwrap();
        let outsider: Uuid = sqlx::query_scalar(
            "INSERT INTO hph_users (display_name) VALUES ('Tenant B only user') RETURNING id",
        )
        .fetch_one(admin.pool())
        .await
        .unwrap();

        let person_a: Uuid = sqlx::query_scalar(
            "INSERT INTO hph_people (organization_id, employee_number, display_name) \
             VALUES ($1, 'membership-a', 'Membership A person') RETURNING id",
        )
        .bind(tenant_a)
        .fetch_one(admin.pool())
        .await
        .unwrap();
        let person_b: Uuid = sqlx::query_scalar(
            "INSERT INTO hph_people (organization_id, employee_number, display_name) \
             VALUES ($1, 'membership-b', 'Membership B person') RETURNING id",
        )
        .bind(tenant_b)
        .fetch_one(admin.pool())
        .await
        .unwrap();
        let person_b2: Uuid = sqlx::query_scalar(
            "INSERT INTO hph_people (organization_id, employee_number, display_name) \
             VALUES ($1, 'membership-b2', 'Tenant B only person') RETURNING id",
        )
        .bind(tenant_b)
        .fetch_one(admin.pool())
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO hph_memberships (organization_id, user_id, person_id, role) \
             VALUES ($1, $2, $3, 'admin'), ($4, $2, $5, 'employee'), ($4, $6, $7, 'admin')",
        )
        .bind(tenant_a)
        .bind(user)
        .bind(person_a)
        .bind(tenant_b)
        .bind(person_b)
        .bind(outsider)
        .bind(person_b2)
        .execute(admin.pool())
        .await
        .unwrap();

        let repository = MembershipRepository::new(&runtime);
        let actor_a = repository.resolve_actor(user, tenant_a).await.unwrap();
        assert_eq!(actor_a.tenant_id, tenant_a.to_string());
        assert_eq!(actor_a.person_id, person_a.to_string());
        assert_eq!(actor_a.role, Role::Admin);

        let actor_b = repository.resolve_actor(user, tenant_b).await.unwrap();
        assert_eq!(actor_b.tenant_id, tenant_b.to_string());
        assert_eq!(actor_b.person_id, person_b.to_string());
        assert_eq!(actor_b.role, Role::Employee);

        assert!(matches!(
            repository.resolve_actor(outsider, tenant_a).await,
            Err(MembershipError::Unauthorized)
        ));

        sqlx::query(
            "UPDATE hph_memberships SET active = false \
             WHERE organization_id = $1 AND user_id = $2",
        )
        .bind(tenant_a)
        .bind(user)
        .execute(admin.pool())
        .await
        .unwrap();
        assert!(matches!(
            repository.resolve_actor(user, tenant_a).await,
            Err(MembershipError::Unauthorized)
        ));

        sqlx::query("DELETE FROM hph_organizations WHERE id IN ($1, $2)")
            .bind(tenant_a)
            .bind(tenant_b)
            .execute(admin.pool())
            .await
            .unwrap();
        sqlx::query("DELETE FROM hph_users WHERE id IN ($1, $2)")
            .bind(user)
            .bind(outsider)
            .execute(admin.pool())
            .await
            .unwrap();
        runtime.close().await;
        admin.close().await;
    }
}
