//! Local account authentication. Passwords and opaque session tokens never enter HTML/logs.
use crate::{
    database::Database,
    operations::{AppError, AppResult},
};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use sha2::{Digest, Sha256};
use sqlx::types::Uuid;

#[derive(Clone)]
pub struct Session {
    pub hash: String,
    pub user: Uuid,
    pub tenant: Option<Uuid>,
    pub csrf: String,
}

pub fn random_token() -> AppResult<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| AppError::Internal)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
pub fn digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

pub async fn password_hash(password: String) -> AppResult<String> {
    if !(12..=128).contains(&password.len()) {
        return Err(AppError::Invalid(
            "Use a password between 12 and 128 bytes.",
        ));
    }
    tokio::task::spawn_blocking(move || {
        let mut salt = [0u8; 16];
        getrandom::fill(&mut salt).map_err(|_| AppError::Internal)?;
        let salt = SaltString::encode_b64(&salt).map_err(|_| AppError::Internal)?;
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|_| AppError::Internal)
    })
    .await
    .map_err(|_| AppError::Internal)?
}

pub async fn verify(password: String, hash: String) -> bool {
    tokio::task::spawn_blocking(move || {
        PasswordHash::new(&hash).ok().is_some_and(|hash| {
            Argon2::default()
                .verify_password(password.as_bytes(), &hash)
                .is_ok()
        })
    })
    .await
    .unwrap_or(false)
}

pub async fn login(db: &Database, email: &str, password: String, dummy: &str) -> AppResult<String> {
    if email.len() > 254 || password.len() > 128 {
        return Err(AppError::Unauthorized);
    }
    let mut tx = db.pool().begin().await?;
    let row: Option<(Uuid, String, bool)> = sqlx::query_as(
        "SELECT u.id, c.password_hash, COALESCE(c.locked_until > now(), false) \
         FROM hph_users u JOIN hph_credentials c ON c.user_id=u.id \
         WHERE lower(u.email)=$1 FOR UPDATE OF c",
    )
    .bind(email.trim().to_lowercase())
    .fetch_optional(&mut *tx)
    .await?;
    let hash = row
        .as_ref()
        .map(|r| r.1.as_str())
        .unwrap_or(dummy)
        .to_owned();
    let valid = verify(password, hash).await;
    let Some((user, _, locked)) = row else {
        return Err(AppError::Unauthorized);
    };
    if locked || !valid {
        if !locked {
            sqlx::query("UPDATE hph_credentials SET failed_attempts=CASE WHEN locked_until <= now() THEN 1 ELSE failed_attempts+1 END, locked_until=CASE WHEN failed_attempts >= 4 AND (locked_until IS NULL OR locked_until > now()) THEN now()+interval '15 minutes' ELSE NULL END WHERE user_id=$1")
                .bind(user).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        return Err(AppError::Unauthorized);
    }
    sqlx::query("UPDATE hph_credentials SET failed_attempts=0, locked_until=NULL WHERE user_id=$1")
        .bind(user)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM hph_sessions WHERE expires_at <= now()")
        .execute(&mut *tx)
        .await?;
    let token = random_token()?;
    sqlx::query("INSERT INTO hph_sessions (token_hash,user_id) VALUES ($1,$2)")
        .bind(digest(&token))
        .bind(user)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(token)
}

pub async fn session(db: &Database, token: &str) -> AppResult<Session> {
    if token.len() != 64 || !token.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(AppError::Unauthorized);
    }
    let hash = digest(token);
    let row: Option<(Uuid, Option<Uuid>)> = sqlx::query_as(
        "SELECT user_id, organization_id FROM hph_sessions WHERE token_hash=$1 AND expires_at>now()")
        .bind(&hash).fetch_optional(db.pool()).await?;
    let (user, tenant) = row.ok_or(AppError::Unauthorized)?;
    Ok(Session {
        hash,
        user,
        tenant,
        csrf: digest(&format!(
            "csrf:{token}:{}",
            tenant.map(|id| id.to_string()).unwrap_or_default()
        )),
    })
}

/// Administrative CLI bootstrap uses the separate migration connection, never a web route.
/// Existing users must prove their password before a second company can be added.
pub async fn bootstrap(
    db: &Database,
    email: String,
    password: String,
    name: String,
    company: String,
    slug: String,
) -> AppResult<()> {
    crate::operations::validate_text(&name, 160)?;
    crate::operations::validate_text(&company, 160)?;
    crate::operations::validate_email(&email)?;
    let hash = password_hash(password.clone()).await?;
    let mut tx = db.pool().begin().await?;
    let existing: Option<(Uuid, String)> = sqlx::query_as("SELECT u.id,c.password_hash FROM hph_users u JOIN hph_credentials c ON c.user_id=u.id WHERE lower(u.email)=lower($1) FOR UPDATE OF c")
        .bind(&email).fetch_optional(&mut *tx).await?;
    let user = if let Some((id, existing_hash)) = existing {
        if !verify(password, existing_hash).await {
            return Err(AppError::Unauthorized);
        }
        id
    } else {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO hph_users(email,display_name) VALUES(lower($1),$2) RETURNING id",
        )
        .bind(&email)
        .bind(&name)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO hph_credentials(user_id,password_hash) VALUES($1,$2)")
            .bind(id)
            .bind(hash)
            .execute(&mut *tx)
            .await?;
        id
    };
    let tenant: Uuid =
        sqlx::query_scalar("INSERT INTO hph_organizations(slug,name) VALUES($1,$2) RETURNING id")
            .bind(slug)
            .bind(company)
            .fetch_one(&mut *tx)
            .await?;
    let person: Uuid=sqlx::query_scalar("INSERT INTO hph_people(organization_id,employee_number,display_name,email) VALUES($1,'ADMIN',$2,$3) RETURNING id")
        .bind(tenant).bind(name).bind(email).fetch_one(&mut *tx).await?;
    sqlx::query("INSERT INTO hph_memberships(organization_id,user_id,person_id,role) VALUES($1,$2,$3,'admin')")
        .bind(tenant).bind(user).bind(person).execute(&mut *tx).await?;
    for id in ["people", "attendance", "reports"] {
        sqlx::query("INSERT INTO hph_company_modules(organization_id,module_id) VALUES($1,$2)")
            .bind(tenant)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn passwords_are_salted_and_verified() {
        let password = "long test password!".to_owned();
        let first = password_hash(password.clone()).await.unwrap();
        let second = password_hash(password.clone()).await.unwrap();
        assert_ne!(first, second);
        assert!(verify(password, first.clone()).await);
        assert!(!verify("incorrect".into(), first).await);
        assert!(password_hash("short".into()).await.is_err());
    }
}
