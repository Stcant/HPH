//! Rust-rendered operational views. Dynamic text and attributes pass through `escape`.
//! Standard forms work without JavaScript; the separate Leptos demo keeps WASM hydration.
use crate::{
    auth::Session,
    operations::{self as op, AppError, AppResult, Person, Snapshot},
};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use hph_core::{ModuleId, PunchKind, WorkState};
use sqlx::types::Uuid;
use std::collections::HashMap;

pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn page(title: &str, body: String) -> String {
    format!(
        r##"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="color-scheme" content="light"><title>{} · HPH</title><link rel="stylesheet" href="/workspace.css"></head><body><a class="skip" href="#main">Skip to content</a>{body}</body></html>"##,
        escape(title)
    )
}
fn hidden(name: &str, value: &str) -> String {
    format!(
        r#"<input type="hidden" name="{}" value="{}">"#,
        escape(name),
        escape(value)
    )
}
fn form(action: &str, s: &Session, body: String) -> String {
    format!(
        r#"<form method="post" action="{action}">{}{body}</form>"#,
        hidden("csrf", &s.csrf)
    )
}
fn input(label: &str, name: &str, value: &str, kind: &str, required: bool, max: usize) -> String {
    format!(
        r#"<label>{}<input type="{}" name="{}" value="{}" maxlength="{max}" {}></label>"#,
        escape(label),
        kind,
        name,
        escape(value),
        if required { "required" } else { "" }
    )
}
fn button(text: &str) -> String {
    format!("<button class=\"primary\">{}</button>", escape(text))
}
fn heading(kicker: &str, title: &str, description: &str) -> String {
    format!(
        "<header class=\"heading\"><p class=\"eyebrow\">{}</p><h1>{}</h1><p>{}</p></header>",
        escape(kicker),
        escape(title),
        escape(description)
    )
}
fn duration(seconds: i64) -> String {
    format!("{}h {:02}m", seconds / 3600, seconds % 3600 / 60)
}
fn initials(name: &str) -> String {
    name.split_whitespace()
        .take(2)
        .filter_map(|s| s.chars().next())
        .collect()
}
fn status(state: WorkState) -> String {
    format!(
        "<span class=\"status {}\">{}</span>",
        if state == WorkState::Working {
            "working"
        } else {
            ""
        },
        state.label()
    )
}
pub fn login() -> String {
    page(
        "Welcome back",
        format!(
            r#"<main id="main" class="auth-layout"><section class="auth-story"><a class="brand" href="/">hph<span>.</span></a><p class="eyebrow">WORK, IN HARMONY</p><h1>Good work.<br>Great people.<br>One place.</h1><p>A little less admin.<br>A little more room for what matters.</p><div class="orb"></div><small>YOUR PEOPLE. YOUR RHYTHM. YOUR WORKSPACE.</small></section><section class="auth-form"><div><p class="eyebrow">YOUR WORKDAY STARTS HERE</p><h2>Welcome back.</h2><p>Sign in to your company workspace.</p><form method="post" action="/auth/login"><label>Work email<input type="email" name="email" required maxlength="254" autocomplete="username"></label><label>Password<input type="password" name="password" required maxlength="128" autocomplete="current-password"></label>{}</form><p class="hint">Need access? Contact your company administrator.</p></div><footer>HPH · Built with Rust, made for people.</footer></section></main>"#,
            button("Sign in →")
        ),
    )
}
pub fn error(message: &str) -> String {
    page(
        "Please check your request",
        format!(
            "<main id=\"main\" class=\"standalone\"><a class=\"brand\" href=\"/app\">hph.</a><h1>Let’s try that again.</h1><p role=\"alert\">{}</p><a class=\"primary\" href=\"/app\">Back to workspace</a> <a href=\"/login\">Sign in</a></main>",
            escape(message)
        ),
    )
}
pub fn companies(s: &Session, companies: &[(Uuid, String)]) -> String {
    let cards=companies.iter().map(|(id,name)|form("/company/select",s,format!("{}<button class=\"company-choice\"><span class=\"avatar\">{}</span><strong>{}</strong><span>Open workspace →</span></button>",hidden("company",&id.to_string()),escape(&initials(name)),escape(name)))).collect::<String>();
    page(
        "Choose your workspace",
        format!(
            "<main id=\"main\" class=\"standalone\"><a class=\"brand\" href=\"/\">hph.</a>{}<div class=\"company-list\">{cards}</div>{}{}</main>",
            heading(
                "YOUR COMPANIES",
                "Where are we working today?",
                "Choose a company. Your role and data stay scoped to that workspace."
            ),
            if companies.is_empty() {
                "<p>No active company membership. Contact your administrator.</p>"
            } else {
                ""
            },
            form(
                "/auth/logout",
                s,
                "<button class=\"text-button\">Sign out</button>".into()
            )
        ),
    )
}
pub fn workspace(s: &Session, d: &Snapshot, q: &HashMap<String, String>) -> AppResult<String> {
    let tab = q.get("tab").map(String::as_str).unwrap_or("overview");
    let me = d
        .people
        .iter()
        .find(|p| p.id == d.access.person)
        .ok_or(AppError::Forbidden)?;
    let timezone = d
        .company
        .timezone
        .parse::<chrono_tz::Tz>()
        .map_err(|_| AppError::Internal)?;
    let today = Utc::now().with_timezone(&timezone).date_naive();
    let month = if let Some(value) = q.get("month") {
        NaiveDate::parse_from_str(&format!("{value}-01"), "%Y-%m-%d")
            .map_err(|_| AppError::Invalid("Choose a valid month."))?
    } else {
        today.with_day(1).unwrap()
    };
    let day = q
        .get("day")
        .filter(|s| !s.is_empty())
        .map(|v| {
            NaiveDate::parse_from_str(v, "%Y-%m-%d")
                .map_err(|_| AppError::Invalid("Choose a valid day."))
        })
        .transpose()?;
    let mut nav = String::new();
    for (id, name, icon, allowed) in [
        ("overview", "Overview", "◈", true),
        (
            "people",
            "People",
            "◎",
            d.modules.contains(ModuleId::People),
        ),
        (
            "attendance",
            "Attendance",
            "◷",
            d.modules.contains(ModuleId::Attendance),
        ),
        (
            "reports",
            "Reports",
            "▥",
            d.modules.contains(ModuleId::Reports),
        ),
        (
            "notes",
            "Team notes",
            "✎",
            d.modules.contains(ModuleId::Notes),
        ),
        ("settings", "Workspace", "⚙", d.access.admin),
        ("account", "My account", "○", true),
    ] {
        if allowed {
            nav += &format!(
                "<a class=\"nav-item {}\" href=\"/app?tab={id}\" {}><span aria-hidden=\"true\">{icon}</span>{name}</a>",
                if tab == id { "active" } else { "" },
                if tab == id {
                    "aria-current=\"page\""
                } else {
                    ""
                }
            );
        }
    }
    let content = match tab {
        "overview" => overview(s, d, me, month)?,
        "people" if d.modules.contains(ModuleId::People) => {
            people(s, d, q.get("edit").map(String::as_str))?
        }
        "attendance" if d.modules.contains(ModuleId::Attendance) => attendance(s, d, month, day)?,
        "reports" if d.modules.contains(ModuleId::Reports) => reports(d, month, day)?,
        "settings" if d.access.admin => settings(s, d),
        "notes" if d.modules.contains(ModuleId::Notes) => notes(s, d),
        "account" => account(s, me),
        _ => return Err(AppError::Forbidden),
    };
    let logo = if d.company.logo.is_empty() {
        format!(
            "<span class=\"avatar\">{}</span>",
            escape(&initials(&d.company.name))
        )
    } else {
        format!(
            "<img class=\"company-logo\" src=\"{}\" alt=\"Company logo\">",
            escape(&d.company.logo)
        )
    };
    Ok(page(
        "Workspace",
        format!(
            r#"<div class="workspace" data-theme="{}" style="--accent:{}"><aside class="sidebar"><a class="brand" href="/app">hph<span>.</span></a><a class="organization" href="/companies">{logo}<span><strong>{}</strong><small>Switch company ↗</small></span></a><p class="nav-label">WORKSPACE</p><nav aria-label="Main navigation">{nav}</nav><div class="sidebar-note"><span>✳</span><strong>Room for your ideas.</strong><p>A workspace that grows<br>with your team.</p></div><div class="profile"><span class="avatar">{}</span><span><strong>{}</strong><small>{}</small></span></div>{}</aside><div class="main-wrap"><header class="topbar"><span>Workspace <span class="muted">/</span> <strong>{}</strong></span><span class="date">{}</span></header><main id="main">{}{content}<footer>HPH · Work, in harmony.<span>All changes saved to your company workspace.</span></footer></main></div></div>"#,
            escape(&d.company.theme),
            escape(&d.company.accent),
            escape(&d.company.name),
            escape(&initials(&me.name)),
            escape(&me.name),
            if d.access.admin {
                "Administrator"
            } else {
                "Team member"
            },
            form(
                "/auth/logout",
                s,
                "<button class=\"signout\">Sign out</button>".into()
            ),
            escape(tab),
            today.format("%A, %d %B %Y"),
            if q.contains_key("saved") {
                "<div class=\"notice\" role=\"status\">Your changes have been saved.</div>"
            } else {
                ""
            }
        ),
    ))
}
fn punch_card(s: &Session, d: &Snapshot) -> AppResult<String> {
    let state = op::sequence(&d.events, d.access.person)?;
    let kinds = match state {
        WorkState::Off => vec![PunchKind::In],
        WorkState::Working => vec![PunchKind::BreakStart, PunchKind::Out],
        WorkState::Paused => vec![PunchKind::BreakEnd, PunchKind::Out],
    };
    let mut buttons = String::new();
    for k in kinds {
        buttons += &form(
            "/attendance/punch",
            s,
            format!(
                "{}{}{}",
                hidden("key", &op::new_id()?.to_string()),
                hidden("kind", op::kind_id(k)),
                button(k.label())
            ),
        );
    }
    Ok(format!(
        "<section class=\"panel clock-panel\"><p class=\"eyebrow\">YOUR TIME</p><div class=\"clock-icon\">◷</div>{}<h2>{}</h2><p>One moment. One click.<br>Your workday, recorded.</p><div class=\"actions\">{buttons}</div><small>Server-recorded time · {}</small></section>",
        status(state),
        match state {
            WorkState::Off => "Ready when you are.",
            WorkState::Working => "You’re on the clock.",
            WorkState::Paused => "Take a little breather.",
        },
        escape(&d.company.timezone)
    ))
}
fn overview(s: &Session, d: &Snapshot, me: &Person, month: NaiveDate) -> AppResult<String> {
    let working = d
        .people
        .iter()
        .filter(|p| op::sequence(&d.events, p.id).ok() == Some(WorkState::Working))
        .count();
    let tz = d.company.timezone.parse().map_err(|_| AppError::Internal)?;
    let seconds = op::daily_totals(&d.events, me.id, tz, month)?
        .values()
        .sum();
    let mut stats = String::new();
    for (label, value, note) in [
        (
            "People",
            d.people.iter().filter(|p| p.active).count().to_string(),
            "Active profiles",
        ),
        ("Working now", working.to_string(), "In your visible team"),
        (
            "Your recorded time",
            duration(seconds),
            "Closed intervals this month",
        ),
        (
            "Active modules",
            d.modules.enabled.len().to_string(),
            "Made to fit your work",
        ),
    ] {
        stats += &format!(
            "<section class=\"stat\"><span>{label}</span><strong>{value}</strong><small>{note}</small></section>"
        );
    }
    let people_rows=d.people.iter().filter(|p|p.active).take(5).map(|p|format!("<div class=\"team-row\"><span class=\"avatar\">{}</span><span><strong>{}</strong><small>{}</small></span>{}</div>",escape(&initials(&p.name)),escape(&p.name),escape(&p.job),status(op::sequence(&d.events,p.id).unwrap_or(WorkState::Off)))).collect::<String>();
    Ok(format!(
        "{}<section class=\"hero\"><div><p class=\"eyebrow\">GOOD WORK STARTS HERE</p><h2>Less complexity.<br>More room for people.</h2><p>Your team, your time, your way of working.<br>Everything you need to make today flow.</p><a class=\"light-button\" href=\"/app?tab={}\">{} →</a></div><div class=\"hero-art\" aria-hidden=\"true\"><div class=\"arch one\"></div><div class=\"arch two\"></div><div class=\"arch three\"></div><span>✳</span></div></section><div class=\"stats-grid\">{stats}</div><div class=\"content-grid\"><section class=\"panel\"><p class=\"eyebrow\">BETTER TOGETHER</p><h2>Your team.</h2><p>People at the heart of every day.</p>{people_rows}</section>{}</div>",
        heading(
            "YOUR WORKSPACE",
            &format!(
                "Good to see you, {}.",
                me.name.split_whitespace().next().unwrap_or("there")
            ),
            "A clear view of your day. A little more space to do good work."
        ),
        if d.access.admin {
            "settings"
        } else {
            "account"
        },
        if d.access.admin {
            "Make it yours"
        } else {
            "Your account"
        },
        if d.modules.contains(ModuleId::Attendance) {
            punch_card(s, d)?
        } else {
            "<section class=\"panel\"><h2>A workspace that fits.</h2><p>Attendance is currently disabled for your company.</p></section>".into()
        }
    ))
}
fn person_form(s: &Session, p: Option<&Person>) -> String {
    let field =
        |label, name, value, kind, required, max| input(label, name, value, kind, required, max);
    let body = format!(
        "{}<div class=\"form-grid\">{}{}{}{}{}<label>Role<select name=\"role\"><option value=\"employee\">Employee</option><option value=\"admin\" {}>Administrator</option></select></label></div><label class=\"check\"><input type=\"checkbox\" name=\"active\" {}> Active employee</label>{}{}",
        hidden("id", &p.map(|p| p.id.to_string()).unwrap_or_default()),
        field(
            "Full name",
            "name",
            p.map(|p| p.name.as_str()).unwrap_or(""),
            "text",
            true,
            160
        ),
        field(
            "Employee number",
            "number",
            p.map(|p| p.number.as_str()).unwrap_or(""),
            "text",
            true,
            80
        ),
        field(
            "Contact email",
            "email",
            p.map(|p| p.email.as_str()).unwrap_or(""),
            "email",
            false,
            254
        ),
        field(
            "Department",
            "department",
            p.map(|p| p.department.as_str()).unwrap_or(""),
            "text",
            false,
            160
        ),
        field(
            "Job title",
            "job",
            p.map(|p| p.job.as_str()).unwrap_or(""),
            "text",
            false,
            160
        ),
        if p.is_some_and(|p| p.role == "admin") {
            "selected"
        } else {
            ""
        },
        if p.is_none_or(|p| p.active) {
            "checked"
        } else {
            ""
        },
        if p.is_none() {
            format!(
                "{}<p class=\"hint\">Optional: set a password of at least 12 characters to create a login with this email. Existing accounts are never overwritten. Share credentials through your own secure channel.</p>",
                input(
                    "Initial login password",
                    "password",
                    "",
                    "password",
                    false,
                    128
                )
            )
        } else {
            "<p class=\"hint\">Contact email changes do not change the account’s login email.</p>"
                .into()
        },
        button("Save employee")
    );
    form("/people/save", s, body)
}
fn people(s: &Session, d: &Snapshot, edit: Option<&str>) -> AppResult<String> {
    let mut rows = String::new();
    for p in &d.people {
        rows += &format!(
            "<tr><td><div class=\"person-cell\"><span class=\"avatar\">{}</span><span><strong>{}</strong><small>{}</small></span></div></td><td>{}</td><td>{}</td><td><span class=\"status\">{}</span></td><td>{}</td></tr>",
            escape(&initials(&p.name)),
            escape(&p.name),
            escape(&p.number),
            escape(&p.department),
            escape(&p.role),
            if p.active { "Active" } else { "Inactive" },
            if d.access.admin {
                format!("<a href=\"/app?tab=people&edit={}\">Edit</a>", p.id)
            } else {
                String::new()
            }
        );
    }
    let editor = if d.access.admin {
        let selected = edit
            .map(op::uuid)
            .transpose()?
            .map(|id| {
                d.people
                    .iter()
                    .find(|p| p.id == id)
                    .ok_or(AppError::Forbidden)
            })
            .transpose()?;
        format!(
            "<section class=\"panel\"><h2>{}</h2>{}</section>",
            if selected.is_some() {
                "Edit employee"
            } else {
                "Add someone to the team"
            },
            person_form(s, selected)
        )
    } else {
        String::new()
    };
    Ok(format!(
        "{}<section class=\"panel table-scroll\"><table><caption class=\"sr-only\">Employee directory</caption><thead><tr><th>Person</th><th>Department</th><th>Role</th><th>Status</th><th>Actions</th></tr></thead><tbody>{rows}</tbody></table></section>{editor}",
        heading(
            "PEOPLE",
            "Your team makes the difference.",
            "Keep profiles and responsibilities in one place. Inactive profiles retain their history."
        )
    ))
}
fn filters(tab: &str, month: NaiveDate, day: Option<NaiveDate>) -> String {
    format!(
        "<form class=\"filters\" method=\"get\" action=\"/app\">{}{}{}<button class=\"secondary\">Apply filters</button></form>",
        hidden("tab", tab),
        input(
            "Month",
            "month",
            &month.format("%Y-%m").to_string(),
            "month",
            true,
            7
        ),
        input(
            "Day (optional)",
            "day",
            &day.map(|d| d.to_string()).unwrap_or_default(),
            "date",
            false,
            10
        )
    )
}
fn attendance(
    s: &Session,
    d: &Snapshot,
    month: NaiveDate,
    day: Option<NaiveDate>,
) -> AppResult<String> {
    let tz = d
        .company
        .timezone
        .parse::<chrono_tz::Tz>()
        .map_err(|_| AppError::Internal)?;
    let mut rows = String::new();
    for e in d.events.iter().rev() {
        let local = DateTime::from_timestamp(e.at, 0)
            .ok_or(AppError::Internal)?
            .with_timezone(&tz);
        if local.year() != month.year()
            || local.month() != month.month()
            || day.is_some_and(|d| d != local.date_naive())
        {
            continue;
        }
        let name = d
            .people
            .iter()
            .find(|p| p.id == e.person)
            .map(|p| p.name.as_str())
            .unwrap_or("Former employee");
        let correction = if d.access.admin {
            format!(
                "<details><summary>Correct time</summary>{}</details>",
                form(
                    "/attendance/correct",
                    s,
                    format!(
                        "{}{}{}{}",
                        hidden("event", &e.id.to_string()),
                        input(
                            "Correct timestamp (with offset)",
                            "at",
                            &local.to_rfc3339(),
                            "text",
                            true,
                            40
                        ),
                        input("Reason for correction", "reason", "", "text", true, 500),
                        button("Save correction")
                    )
                )
            )
        } else {
            String::new()
        };
        rows += &format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{correction}</td></tr>",
            escape(name),
            e.kind.label(),
            local.format("%Y-%m-%d %H:%M:%S %:z")
        );
    }
    if rows.is_empty() {
        rows="<tr><td colspan=\"4\" class=\"empty\">No time entries for this period. Your next clock-in will appear here.</td></tr>".into();
    }
    Ok(format!(
        "{}<div class=\"attendance-layout\">{}<section class=\"panel\"><h2>Every moment, accounted for.</h2>{}<div class=\"table-scroll\"><table><caption class=\"sr-only\">Attendance events</caption><thead><tr><th>Person</th><th>Event</th><th>Local time</th><th>Actions</th></tr></thead><tbody>{rows}</tbody></table></div><p class=\"hint\">Corrections require a reason and remain visible in the audit history.</p></section></div>",
        heading(
            "ATTENDANCE",
            "Every day has its own rhythm.",
            &format!(
                "Clock-ins, breaks and clock-outs. Times shown in {}.",
                d.company.timezone
            )
        ),
        punch_card(s, d)?,
        filters("attendance", month, day)
    ))
}
fn reports(d: &Snapshot, month: NaiveDate, day: Option<NaiveDate>) -> AppResult<String> {
    let tz = d.company.timezone.parse().map_err(|_| AppError::Internal)?;
    let mut rows = String::new();
    let mut total = 0;
    for p in &d.people {
        for (date, seconds) in op::daily_totals(&d.events, p.id, tz, month)? {
            if day.is_some_and(|d| d != date) {
                continue;
            }
            total += seconds;
            rows += &format!(
                "<tr><td>{date}</td><td>{}</td><td>{}</td></tr>",
                escape(&p.name),
                duration(seconds)
            );
        }
    }
    if rows.is_empty() {
        rows="<tr><td colspan=\"3\" class=\"empty\">No completed work intervals for this period.</td></tr>".into();
    }
    Ok(format!(
        "{}<section class=\"panel\">{}<div class=\"report-total\"><span>Recorded time</span><strong>{}</strong></div><div class=\"table-scroll\"><table><caption class=\"sr-only\">Daily recorded hours</caption><thead><tr><th>Day</th><th>Person</th><th>Recorded time</th></tr></thead><tbody>{rows}</tbody></table></div><p class=\"hint\">Closed intervals only. Breaks and open intervals are excluded. Overnight work is split at local midnight, respecting daylight saving time. No payroll, overtime or contractual rounding is applied.</p></section>",
        heading(
            "REPORTS",
            "A clearer view of time.",
            &format!("Daily and monthly totals in {}.", d.company.timezone)
        ),
        filters("reports", month, day),
        duration(total)
    ))
}
fn settings(s: &Session, d: &Snapshot) -> String {
    let c = &d.company;
    let settings = form(
        "/settings/save",
        s,
        format!(
            "<div class=\"form-grid\">{}{}<label>Theme<select name=\"theme\"><option value=\"sage\">Sage</option><option value=\"indigo\" {}>Indigo</option></select></label>{}{}</div><p class=\"hint\">Logo files are managed by your installation administrator under public/assets/. Use /assets/your-logo.svg or leave empty for initials.</p>{}",
            input("Company name", "name", &c.name, "text", true, 160),
            input(
                "Company timezone",
                "timezone",
                &c.timezone,
                "text",
                true,
                100
            ),
            if c.theme == "indigo" { "selected" } else { "" },
            input("Accent color", "accent", &c.accent, "color", true, 7),
            input("Logo path", "logo", &c.logo, "text", false, 200),
            button("Save appearance")
        ),
    );
    let mut modules = String::new();
    for (name, id, title, description) in [
        (
            "people",
            ModuleId::People,
            "People",
            "Employee directory and roles. No dependencies.",
        ),
        (
            "attendance",
            ModuleId::Attendance,
            "Time and attendance",
            "Clock-ins, breaks and corrections. Requires People.",
        ),
        (
            "reports",
            ModuleId::Reports,
            "Reports",
            "Daily and monthly totals. Requires Attendance.",
        ),
        (
            "notes",
            ModuleId::Notes,
            "Team notes",
            "A shared noticeboard and working module example. Requires People.",
        ),
    ] {
        modules += &format!(
            "<label class=\"module-option\"><input type=\"checkbox\" name=\"{name}\" {}><span><strong>{title}</strong><small>{description}</small></span></label>",
            if d.modules.contains(id) {
                "checked"
            } else {
                ""
            }
        );
    }
    modules += &button("Save modules");
    let audit = d
        .audit
        .iter()
        .map(|(action, detail, at)| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                escape(at),
                escape(action),
                escape(detail)
            )
        })
        .collect::<String>();
    format!(
        "{}<div class=\"settings-grid\"><section class=\"panel\"><h2>Make it feel like yours.</h2>{settings}</section><section class=\"panel\"><h2>The right tools for your team.</h2><p>Choose the modules your company needs.</p>{}</section></div><section class=\"panel table-scroll\"><h2>Change history</h2><p>Latest 50 events · UTC · append-only</p><table><caption class=\"sr-only\">Company audit history</caption><thead><tr><th>Time</th><th>Action</th><th>Details</th></tr></thead><tbody>{audit}</tbody></table></section>",
        heading(
            "YOUR WORKSPACE",
            "Built around you.",
            "Company identity, appearance and modules. Changes apply to this company only."
        ),
        form("/modules/save", s, modules)
    )
}
fn notes(s: &Session, d: &Snapshot) -> String {
    let rows = d
        .notes
        .iter()
        .map(|(body, at)| {
            format!(
                "<article class=\"panel note\"><small>{} UTC</small><p>{}</p></article>",
                escape(at),
                escape(body)
            )
        })
        .collect::<String>();
    format!(
        "{}<section class=\"panel\">{}</section><div class=\"notes-grid\">{rows}</div>",
        heading(
            "TEAM NOTES",
            "Small updates. Shared context.",
            "A company noticeboard for the things everyone should know."
        ),
        form(
            "/notes/add",
            s,
            format!(
                "<label>Your note<textarea name=\"body\" required maxlength=\"2000\" rows=\"4\" placeholder=\"Something worth sharing…\"></textarea></label>{}",
                button("Post note")
            )
        )
    )
}
fn account(s: &Session, me: &Person) -> String {
    format!(
        "{}<section class=\"panel narrow\"><h2>Change your password</h2><p>Changing your password signs out all your sessions.</p>{}</section>",
        heading("MY ACCOUNT", &me.name, "Keep your account secure."),
        form(
            "/auth/password",
            s,
            format!(
                "{}{}{}",
                input(
                    "Current password",
                    "current_password",
                    "",
                    "password",
                    true,
                    128
                ),
                input(
                    "New password (at least 12 characters)",
                    "new_password",
                    "",
                    "password",
                    true,
                    128
                ),
                button("Change password")
            )
        )
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn user_text_is_escaped() {
        assert_eq!(escape("<script>\"'&"), "&lt;script&gt;&quot;&#39;&amp;");
        assert!(login().contains("lang=\"en\""));
    }
}
