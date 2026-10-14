//! Company operations share one transaction and company lock, including authorization.
use crate::{auth::Session, database::Database};
use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use hph_core::{ModuleId, ModuleSet, PunchKind, WorkState};
use sqlx::{Postgres, Transaction, types::Uuid};
use std::collections::BTreeMap;

#[derive(Debug)]
pub enum AppError {
    Unauthorized,
    Forbidden,
    Invalid(&'static str),
    Internal,
}
pub type AppResult<T> = Result<T, AppError>;
impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        if let sqlx::Error::Database(ref e) = error {
            if e.is_unique_violation() {
                return Self::Invalid("That identifier or email is already in use.");
            }
        }
        eprintln!(
            "HPH database operation failed (details withheld): {}",
            match error {
                sqlx::Error::RowNotFound => "row not found",
                _ => "database error",
            }
        );
        Self::Internal
    }
}
impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unauthorized => "Sign in again. The credentials or session are not valid.",
            Self::Forbidden => "You do not have permission for this operation.",
            Self::Invalid(message) => message,
            Self::Internal => "The operation could not be completed. Please try again.",
        })
    }
}
impl std::error::Error for AppError {}

pub fn validate_text(value: &str, max: usize) -> AppResult<()> {
    if value.trim().is_empty() || value.len() > max || value.chars().any(|c| c.is_control()) {
        return Err(AppError::Invalid(
            "Check the required text fields and their length.",
        ));
    }
    Ok(())
}
pub fn validate_email(value: &str) -> AppResult<()> {
    validate_text(value, 254)?;
    if !value.contains('@') || value.contains(char::is_whitespace) {
        return Err(AppError::Invalid("Enter a valid email address."));
    }
    Ok(())
}
pub fn uuid(value: &str) -> AppResult<Uuid> {
    Uuid::parse_str(value).map_err(|_| AppError::Invalid("Invalid identifier."))
}
pub fn new_id() -> AppResult<Uuid> {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes).map_err(|_| AppError::Internal)?;
    Ok(Uuid::from_bytes(bytes))
}
pub fn module_id(value: &str) -> AppResult<ModuleId> {
    match value {
        "people" => Ok(ModuleId::People),
        "attendance" => Ok(ModuleId::Attendance),
        "reports" => Ok(ModuleId::Reports),
        "notes" => Ok(ModuleId::Notes),
        _ => Err(AppError::Invalid("Unknown module.")),
    }
}
pub fn kind(value: &str) -> AppResult<PunchKind> {
    match value {
        "in" => Ok(PunchKind::In),
        "out" => Ok(PunchKind::Out),
        "break_start" => Ok(PunchKind::BreakStart),
        "break_end" => Ok(PunchKind::BreakEnd),
        _ => Err(AppError::Invalid("Unknown time event.")),
    }
}
pub fn kind_id(value: PunchKind) -> &'static str {
    match value {
        PunchKind::In => "in",
        PunchKind::Out => "out",
        PunchKind::BreakStart => "break_start",
        PunchKind::BreakEnd => "break_end",
    }
}

pub struct Access {
    pub tenant: Uuid,
    pub person: Uuid,
    pub admin: bool,
}
pub async fn begin<'a>(
    db: &'a Database,
    s: &Session,
) -> AppResult<(Transaction<'a, Postgres>, Access)> {
    let tenant = s.tenant.ok_or(AppError::Forbidden)?;
    let mut tx = db.begin_tenant_transaction(s.user, tenant).await?;
    // Serializes mutations, membership revocations and reads into a consistent company snapshot.
    let exists: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM hph_organizations WHERE id=$1 FOR UPDATE")
            .bind(tenant)
            .fetch_optional(&mut *tx)
            .await?;
    if exists.is_none() {
        return Err(AppError::Forbidden);
    }
    let member:Option<(Uuid,String)>=sqlx::query_as("SELECT m.person_id,m.role FROM hph_memberships m JOIN hph_people p ON p.organization_id=m.organization_id AND p.id=m.person_id WHERE m.organization_id=$1 AND m.user_id=$2 AND m.active AND p.active")
        .bind(tenant).bind(s.user).fetch_optional(&mut *tx).await?;
    let (person, role) = member.ok_or(AppError::Forbidden)?;
    Ok((
        tx,
        Access {
            tenant,
            person,
            admin: role == "admin",
        },
    ))
}
fn admin(a: &Access) -> AppResult<()> {
    if a.admin {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}
pub async fn modules(tx: &mut Transaction<'_, Postgres>, tenant: Uuid) -> AppResult<ModuleSet> {
    let names: Vec<String> = sqlx::query_scalar(
        "SELECT module_id FROM hph_company_modules WHERE organization_id=$1 AND enabled",
    )
    .bind(tenant)
    .fetch_all(&mut **tx)
    .await?;
    ModuleSet::from_enabled(
        names
            .iter()
            .map(|s| module_id(s))
            .collect::<AppResult<Vec<_>>>()?,
    )
    .map_err(|_| AppError::Invalid("Invalid company module dependencies."))
}
pub async fn require_module(
    tx: &mut Transaction<'_, Postgres>,
    tenant: Uuid,
    id: ModuleId,
) -> AppResult<()> {
    if modules(tx, tenant).await?.contains(id) {
        Ok(())
    } else {
        Err(AppError::Invalid("This module is disabled."))
    }
}
async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    s: &Session,
    action: &str,
    detail: String,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO hph_audit(organization_id,actor_user_id,action,detail) VALUES($1,$2,$3,$4)",
    )
    .bind(s.tenant)
    .bind(s.user)
    .bind(action)
    .bind(detail)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub struct Company {
    pub name: String,
    pub timezone: String,
    pub theme: String,
    pub logo: String,
    pub accent: String,
}
pub struct Person {
    pub id: Uuid,
    pub number: String,
    pub name: String,
    pub email: String,
    pub department: String,
    pub job: String,
    pub active: bool,
    pub role: String,
}
#[derive(Clone)]
pub struct Event {
    pub id: Uuid,
    pub person: Uuid,
    pub kind: PunchKind,
    pub at: i64,
}
pub struct Snapshot {
    pub company: Company,
    pub access: Access,
    pub modules: ModuleSet,
    pub people: Vec<Person>,
    pub events: Vec<Event>,
    pub notes: Vec<(String, String)>,
    pub audit: Vec<(String, String, String)>,
}

pub async fn events(tx: &mut Transaction<'_, Postgres>, tenant: Uuid) -> AppResult<Vec<Event>> {
    let rows:Vec<(Uuid,Uuid,String,i64)>=sqlx::query_as("SELECT id,person_id,kind,extract(epoch FROM occurred_at)::bigint FROM hph_attendance_events WHERE organization_id=$1 ORDER BY occurred_at,created_at,id")
        .bind(tenant).fetch_all(&mut **tx).await?;
    rows.into_iter()
        .map(|(id, person, k, at)| {
            Ok(Event {
                id,
                person,
                kind: kind(&k)?,
                at,
            })
        })
        .collect()
}
pub fn sequence(events: &[Event], person: Uuid) -> AppResult<WorkState> {
    events
        .iter()
        .filter(|e| e.person == person)
        .try_fold(WorkState::Off, |state, e| {
            state.next(e.kind).map_err(|_| {
                AppError::Invalid(
                    "The change would create an invalid clock-in, break or clock-out sequence.",
                )
            })
        })
}
pub async fn snapshot(db: &Database, s: &Session) -> AppResult<Snapshot> {
    let (mut tx, access) = begin(db, s).await?;
    let (name, timezone, theme, logo, accent): (String, String, String, String, String) =
        sqlx::query_as(
            "SELECT name,timezone,theme,logo_path,accent FROM hph_organizations WHERE id=$1",
        )
        .bind(access.tenant)
        .fetch_one(&mut *tx)
        .await?;
    let modules = modules(&mut tx, access.tenant).await?;
    let rows:Vec<(Uuid,String,String,String,String,String,bool,String)>=sqlx::query_as("SELECT p.id,p.employee_number,p.display_name,COALESCE(p.email,''),COALESCE(p.department,''),COALESCE(p.job_title,''),p.active,COALESCE(m.role,'') FROM hph_people p LEFT JOIN hph_memberships m ON m.organization_id=p.organization_id AND m.person_id=p.id WHERE p.organization_id=$1 AND ($2 OR p.id=$3) ORDER BY p.display_name")
        .bind(access.tenant).bind(access.admin).bind(access.person).fetch_all(&mut *tx).await?;
    let people = rows
        .into_iter()
        .map(
            |(id, number, name, email, department, job, active, role)| Person {
                id,
                number,
                name,
                email,
                department,
                job,
                active,
                role,
            },
        )
        .collect();
    let events = events(&mut tx, access.tenant)
        .await?
        .into_iter()
        .filter(|e| access.admin || e.person == access.person)
        .collect();
    let notes = if modules.contains(ModuleId::Notes) {
        sqlx::query_as("SELECT body,to_char(created_at AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI') FROM hph_notes WHERE organization_id=$1 ORDER BY created_at DESC LIMIT 50").bind(access.tenant).fetch_all(&mut *tx).await?
    } else {
        vec![]
    };
    let audit = if access.admin {
        sqlx::query_as("SELECT a.action,u.display_name || ': ' || a.detail,to_char(a.created_at AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI:SS') FROM hph_audit a JOIN hph_users u ON u.id=a.actor_user_id WHERE a.organization_id=$1 ORDER BY a.id DESC LIMIT 50").bind(access.tenant).fetch_all(&mut *tx).await?
    } else {
        vec![]
    };
    tx.commit().await?;
    Ok(Snapshot {
        company: Company {
            name,
            timezone,
            theme,
            logo,
            accent,
        },
        access,
        modules,
        people,
        events,
        notes,
        audit,
    })
}

pub async fn companies(db: &Database, s: &Session) -> AppResult<Vec<(Uuid, String)>> {
    let mut tx = db.begin_tenant_transaction(s.user, Uuid::nil()).await?;
    let ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT organization_id FROM hph_memberships WHERE user_id=$1 AND active",
    )
    .bind(s.user)
    .fetch_all(&mut *tx)
    .await?;
    let mut result = vec![];
    for id in ids {
        sqlx::query("SELECT set_config('hph.tenant_id',$1,true)")
            .bind(id.to_string())
            .execute(&mut *tx)
            .await?;
        let name:Option<String>=sqlx::query_scalar("SELECT o.name FROM hph_organizations o JOIN hph_memberships m ON m.organization_id=o.id JOIN hph_people p ON p.organization_id=m.organization_id AND p.id=m.person_id WHERE o.id=$1 AND m.user_id=$2 AND m.active AND p.active")
            .bind(id).bind(s.user).fetch_optional(&mut *tx).await?;
        if let Some(name) = name {
            result.push((id, name));
        }
    }
    tx.commit().await?;
    Ok(result)
}
pub async fn select_company(db: &Database, s: &Session, id: Uuid) -> AppResult<()> {
    let mut selected = s.clone();
    selected.tenant = Some(id);
    let (mut tx, _) = begin(db, &selected).await?;
    sqlx::query("UPDATE hph_sessions SET organization_id=$1 WHERE token_hash=$2 AND user_id=$3 AND expires_at>now()")
        .bind(id).bind(&s.hash).bind(s.user).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub struct PersonInput {
    pub id: Option<Uuid>,
    pub number: String,
    pub name: String,
    pub email: String,
    pub department: String,
    pub job: String,
    pub active: bool,
    pub role: String,
    pub password: String,
}
pub async fn save_person(db: &Database, s: &Session, input: PersonInput) -> AppResult<()> {
    validate_text(&input.number, 80)?;
    validate_text(&input.name, 160)?;
    if !input.email.is_empty() {
        validate_email(&input.email)?;
    }
    if input.department.len() > 160
        || input.job.len() > 160
        || !["employee", "admin"].contains(&input.role.as_str())
    {
        return Err(AppError::Invalid("Invalid employee details."));
    }
    // Expensive hashing happens only after permission checks and outside the company lock.
    let (tx, a) = begin(db, s).await?;
    admin(&a)?;
    drop(tx);
    let hash = if input.id.is_none() && !input.password.is_empty() {
        Some(crate::auth::password_hash(input.password).await?)
    } else {
        None
    };
    let (mut tx, a) = begin(db, s).await?;
    admin(&a)?;
    require_module(&mut tx, a.tenant, ModuleId::People).await?;
    let id = if let Some(id) = input.id {
        if id == a.person && (!input.active || input.role != "admin") {
            return Err(AppError::Invalid(
                "You cannot deactivate or demote your own administrator account.",
            ));
        }
        let all = events(&mut tx, a.tenant).await?;
        if !input.active && sequence(&all, id)? != WorkState::Off {
            return Err(AppError::Invalid(
                "Close the employee's work session before deactivating them.",
            ));
        }
        let n=sqlx::query("UPDATE hph_people SET employee_number=$1,display_name=$2,email=$3,department=$4,job_title=$5,active=$6 WHERE organization_id=$7 AND id=$8")
            .bind(&input.number).bind(&input.name).bind(&input.email).bind(&input.department).bind(&input.job).bind(input.active).bind(a.tenant).bind(id).execute(&mut *tx).await?.rows_affected();
        if n != 1 {
            return Err(AppError::Forbidden);
        }
        sqlx::query("UPDATE hph_memberships SET active=$1,role=$2 WHERE organization_id=$3 AND person_id=$4")
            .bind(input.active).bind(&input.role).bind(a.tenant).bind(id).execute(&mut *tx).await?;
        id
    } else {
        let id:Uuid=sqlx::query_scalar("INSERT INTO hph_people(organization_id,employee_number,display_name,email,department,job_title,active) VALUES($1,$2,$3,$4,$5,$6,$7) RETURNING id")
            .bind(a.tenant).bind(&input.number).bind(&input.name).bind(&input.email).bind(&input.department).bind(&input.job).bind(input.active).fetch_one(&mut *tx).await?;
        if let Some(hash) = hash {
            validate_email(&input.email)?;
            // Never overwrite an existing global user's password from a company admin screen.
            let user: Uuid = sqlx::query_scalar(
                "INSERT INTO hph_users(email,display_name) VALUES(lower($1),$2) RETURNING id",
            )
            .bind(&input.email)
            .bind(&input.name)
            .fetch_one(&mut *tx)
            .await?;
            sqlx::query("INSERT INTO hph_credentials(user_id,password_hash) VALUES($1,$2)")
                .bind(user)
                .bind(hash)
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO hph_memberships(organization_id,user_id,person_id,role,active) VALUES($1,$2,$3,$4,$5)")
                .bind(a.tenant).bind(user).bind(id).bind(input.role).bind(input.active).execute(&mut *tx).await?;
        }
        id
    };
    audit(
        &mut tx,
        s,
        "person.saved",
        format!("person={id}; active={}", input.active),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn punch(db: &Database, s: &Session, k: PunchKind, key: Uuid) -> AppResult<()> {
    let (mut tx, a) = begin(db, s).await?;
    require_module(&mut tx, a.tenant, ModuleId::Attendance).await?;
    let existing:Option<String>=sqlx::query_scalar("SELECT kind FROM hph_attendance_events WHERE organization_id=$1 AND person_id=$2 AND idempotency_key=$3")
        .bind(a.tenant).bind(a.person).bind(key).fetch_optional(&mut *tx).await?;
    if let Some(existing) = existing {
        return if existing == kind_id(k) {
            Ok(())
        } else {
            Err(AppError::Invalid(
                "This request key was already used for a different event.",
            ))
        };
    }
    let all = events(&mut tx, a.tenant).await?;
    sequence(&all, a.person)?.next(k).map_err(|_| {
        AppError::Invalid("Refresh the page: this time entry is not valid for your current status.")
    })?;
    let at = Utc::now().timestamp();
    if all.iter().any(|e| e.person == a.person && e.at >= at) {
        return Err(AppError::Invalid(
            "Wait one second before recording the next event.",
        ));
    }
    let id:Uuid=sqlx::query_scalar("INSERT INTO hph_attendance_events(organization_id,person_id,created_by_user_id,kind,occurred_at,idempotency_key) VALUES($1,$2,$3,$4,to_timestamp($5),$6) RETURNING id")
        .bind(a.tenant).bind(a.person).bind(s.user).bind(kind_id(k)).bind(at as f64).bind(key).fetch_one(&mut *tx).await?;
    audit(
        &mut tx,
        s,
        "attendance.recorded",
        format!("event={id}; person={}; kind={}", a.person, kind_id(k)),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Correct the timestamp of an existing event only; preserve original values in immutable audit.
pub async fn correct(db: &Database, s: &Session, id: Uuid, at: i64, reason: &str) -> AppResult<()> {
    validate_text(reason, 500)?;
    if at > Utc::now().timestamp() || at < 0 {
        return Err(AppError::Invalid(
            "The corrected time must be a valid past timestamp.",
        ));
    }
    let (mut tx, a) = begin(db, s).await?;
    admin(&a)?;
    require_module(&mut tx, a.tenant, ModuleId::Attendance).await?;
    let mut all = events(&mut tx, a.tenant).await?;
    let e = all
        .iter_mut()
        .find(|e| e.id == id)
        .ok_or(AppError::Forbidden)?;
    let old = e.at;
    let person = e.person;
    e.at = at;
    all.sort_by_key(|e| e.at);
    if all
        .iter()
        .filter(|e| e.person == person && e.at == at)
        .count()
        > 1
    {
        return Err(AppError::Invalid(
            "Two events cannot have the same timestamp.",
        ));
    }
    sequence(&all, person)?;
    sqlx::query("UPDATE hph_attendance_events SET occurred_at=to_timestamp($1) WHERE organization_id=$2 AND id=$3")
        .bind(at as f64).bind(a.tenant).bind(id).execute(&mut *tx).await?;
    audit(
        &mut tx,
        s,
        "attendance.corrected",
        format!("event={id}; from={old}; to={at}; reason={reason}"),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
pub async fn set_modules(db: &Database, s: &Session, names: Vec<String>) -> AppResult<()> {
    let selected = ModuleSet::from_enabled(
        names
            .iter()
            .map(|n| module_id(n))
            .collect::<AppResult<Vec<_>>>()?,
    )
    .map_err(|_| AppError::Invalid("Enable each module's dependencies first."))?;
    let (mut tx, a) = begin(db, s).await?;
    admin(&a)?;
    if !selected.contains(ModuleId::Attendance) {
        let all = events(&mut tx, a.tenant).await?;
        let people: std::collections::HashSet<_> = all.iter().map(|event| event.person).collect();
        for person in people {
            if sequence(&all, person)? != WorkState::Off {
                return Err(AppError::Invalid(
                    "Close all work sessions before disabling attendance.",
                ));
            }
        }
    }
    for name in ["people", "attendance", "reports", "notes"] {
        sqlx::query("INSERT INTO hph_company_modules(organization_id,module_id,enabled) VALUES($1,$2,$3) ON CONFLICT(organization_id,module_id) DO UPDATE SET enabled=excluded.enabled,updated_at=now()")
            .bind(a.tenant).bind(name).bind(selected.contains(module_id(name)?)).execute(&mut *tx).await?;
    }
    audit(&mut tx, s, "modules.saved", names.join(", ")).await?;
    tx.commit().await?;
    Ok(())
}
pub async fn settings(db: &Database, s: &Session, c: Company) -> AppResult<()> {
    validate_text(&c.name, 160)?;
    c.timezone
        .parse::<Tz>()
        .map_err(|_| AppError::Invalid("Use an IANA timezone such as Europe/Rome."))?;
    if !["sage", "indigo"].contains(&c.theme.as_str())
        || c.accent.len() != 7
        || !c.accent.starts_with('#')
        || !c.accent[1..].bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(AppError::Invalid(
            "Choose a supported theme and a six-digit hex color.",
        ));
    }
    // Logos must be deployment-owned local assets, not remote trackers or executable data URLs.
    if !c.logo.is_empty()
        && (!c.logo.starts_with("/assets/")
            || c.logo.contains("..")
            || !c
                .logo
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"/._-".contains(&b)))
    {
        return Err(AppError::Invalid("Use a local logo path under /assets/."));
    }
    let (mut tx, a) = begin(db, s).await?;
    admin(&a)?;
    sqlx::query("UPDATE hph_organizations SET name=$1,timezone=$2,theme=$3,logo_path=$4,accent=$5 WHERE id=$6")
        .bind(c.name).bind(c.timezone).bind(c.theme).bind(c.logo).bind(c.accent).bind(a.tenant).execute(&mut *tx).await?;
    audit(
        &mut tx,
        s,
        "company.settings",
        "Updated company appearance and timezone".into(),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
pub async fn add_note(db: &Database, s: &Session, body: &str) -> AppResult<()> {
    validate_text(body, 2000)?;
    let (mut tx, a) = begin(db, s).await?;
    require_module(&mut tx, a.tenant, ModuleId::Notes).await?;
    sqlx::query("INSERT INTO hph_notes(organization_id,author_user_id,body) VALUES($1,$2,$3)")
        .bind(a.tenant)
        .bind(s.user)
        .bind(body)
        .execute(&mut *tx)
        .await?;
    audit(&mut tx, s, "note.created", "Company note added".into()).await?;
    tx.commit().await?;
    Ok(())
}

/// Split closed work intervals at local calendar boundaries, including DST and overnight work.
pub fn daily_totals(
    events: &[Event],
    person: Uuid,
    tz: Tz,
    month: NaiveDate,
) -> AppResult<BTreeMap<NaiveDate, i64>> {
    let mut result = BTreeMap::new();
    let mut start = None;
    for e in events.iter().filter(|e| e.person == person) {
        match e.kind {
            PunchKind::In | PunchKind::BreakEnd => start = Some(e.at),
            PunchKind::BreakStart | PunchKind::Out => {
                if let Some(mut from) = start.take() {
                    while from < e.at {
                        let date = DateTime::from_timestamp(from, 0)
                            .ok_or(AppError::Internal)?
                            .with_timezone(&tz)
                            .date_naive();
                        let next = date.succ_opt().ok_or(AppError::Internal)?;
                        // A local midnight may not exist in historical timezone transitions.
                        let mut naive = next.and_hms_opt(0, 0, 0).ok_or(AppError::Internal)?;
                        let boundary = loop {
                            if let Some(t) = tz.from_local_datetime(&naive).earliest() {
                                break t.timestamp();
                            }
                            naive += chrono::Duration::minutes(1);
                        };
                        let until = e.at.min(boundary);
                        if date.year() == month.year() && date.month() == month.month() {
                            *result.entry(date).or_insert(0) += until - from;
                        }
                        from = until;
                    }
                }
            }
        }
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reports_split_midnight_and_daylight_saving() {
        let person = Uuid::nil();
        let tz: Tz = "Europe/Rome".parse().unwrap();
        let event = |kind, at: &str| Event {
            id: Uuid::nil(),
            person,
            kind,
            at: DateTime::parse_from_rfc3339(at).unwrap().timestamp(),
        };
        let rows = vec![
            event(PunchKind::In, "2026-03-28T23:00:00+01:00"),
            event(PunchKind::Out, "2026-03-29T04:00:00+02:00"),
        ];
        let totals = daily_totals(
            &rows,
            person,
            tz,
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
        )
        .unwrap();
        assert_eq!(
            totals.values().copied().collect::<Vec<_>>(),
            vec![3600, 10800]
        );
        let rows = vec![event(PunchKind::In, "2026-03-29T04:00:00+02:00")];
        assert!(
            daily_totals(
                &rows,
                person,
                tz,
                NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()
            )
            .unwrap()
            .is_empty()
        );
    }
}
