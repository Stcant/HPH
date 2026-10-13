//! Framework-independent module, permission and attendance contracts.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleId {
    People,
    Attendance,
    Reports,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleDefinition {
    pub id: ModuleId,
    pub name: String,
    pub description: String,
    pub dependencies: Vec<ModuleId>,
}

pub fn catalog() -> Vec<ModuleDefinition> {
    vec![
        ModuleDefinition {
            id: ModuleId::People,
            name: "People".into(),
            description: "Your team, departments and contacts. A shared starting point.".into(),
            dependencies: vec![],
        },
        ModuleDefinition {
            id: ModuleId::Attendance,
            name: "Time and attendance".into(),
            description: "Clock-ins, breaks and clock-outs. A clear view of the working day.".into(),
            dependencies: vec![ModuleId::People],
        },
        ModuleDefinition {
            id: ModuleId::Reports,
            name: "Reports".into(),
            description: "Summaries of recorded hours and completed work sessions.".into(),
            dependencies: vec![ModuleId::Attendance],
        },
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Employee,
    Admin,
}

/// A user identity established by an authentication layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserIdentity {
    pub id: String,
}

/// An explicit link between a user and one company.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Membership {
    pub user_id: String,
    pub tenant_id: String,
    pub person_id: String,
    pub role: Role,
    pub active: bool,
}

impl UserIdentity {
    /// Resolve the actor only from an active membership in the selected company.
    pub fn actor_for(&self, tenant_id: &str, memberships: &[Membership]) -> Result<Actor, String> {
        memberships
            .iter()
            .find(|membership| {
                membership.active
                    && membership.user_id == self.id
                    && membership.tenant_id == tenant_id
            })
            .map(|membership| Actor {
                tenant_id: membership.tenant_id.clone(),
                person_id: membership.person_id.clone(),
                role: membership.role,
            })
            .ok_or_else(|| "No active membership for the selected company.".into())
    }
}

#[derive(Clone, Debug)]
pub struct Actor {
    pub tenant_id: String,
    pub person_id: String,
    pub role: Role,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleSet {
    pub enabled: Vec<ModuleId>,
}

impl ModuleSet {
    pub fn preset(name: &str) -> Result<Self, String> {
        let enabled = match name {
            "essential" => vec![ModuleId::People],
            "team" => vec![ModuleId::People, ModuleId::Attendance],
            "complete" => vec![ModuleId::People, ModuleId::Attendance, ModuleId::Reports],
            _ => return Err("Unrecognized module configuration.".into()),
        };
        Self::from_enabled(enabled)
    }
    /// Build a company-specific module selection and validate all dependencies.
    pub fn from_enabled(enabled: Vec<ModuleId>) -> Result<Self, String> {
        let mut unique = Vec::with_capacity(enabled.len());
        for id in &enabled {
            if unique.contains(id) {
                return Err("The configuration contains a duplicate module.".into());
            }
            unique.push(*id);
        }

        let modules = Self { enabled };
        for definition in catalog()
            .iter()
            .filter(|module| modules.contains(module.id))
        {
            if definition
                .dependencies
                .iter()
                .any(|dependency| !modules.contains(*dependency))
            {
                return Err(format!(
                    "{} requires its dependencies to be enabled. Enable the dependencies first or disable this module first.",
                    definition.name
                ));
            }
        }
        Ok(modules)
    }
    pub fn contains(&self, id: ModuleId) -> bool {
        self.enabled.contains(&id)
    }
    pub fn set(&mut self, id: ModuleId, enabled: bool) -> Result<(), String> {
        let mut next_enabled = self.enabled.clone();
        next_enabled.retain(|value| *value != id);
        if enabled {
            next_enabled.push(id);
        }
        *self = Self::from_enabled(next_enabled)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PunchKind {
    In,
    BreakStart,
    BreakEnd,
    Out,
}
impl PunchKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::In => "Clock in",
            Self::BreakStart => "Start break",
            Self::BreakEnd => "End break",
            Self::Out => "Clock out",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkState {
    Off,
    Working,
    Paused,
}
impl WorkState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off duty",
            Self::Working => "Working",
            Self::Paused => "On break",
        }
    }
    pub fn next(self, kind: PunchKind) -> Result<Self, String> {
        match (self, kind) {
            (Self::Off, PunchKind::In) => Ok(Self::Working),
            (Self::Working, PunchKind::BreakStart) => Ok(Self::Paused),
            (Self::Paused, PunchKind::BreakEnd) => Ok(Self::Working),
            (Self::Working | Self::Paused, PunchKind::Out) => Ok(Self::Off),
            _ => Err(
                "Invalid time entry for the current status. Refresh the page and try again.".into(),
            ),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Person {
    pub id: String,
    pub name: String,
    pub initials: String,
    pub department: String,
    pub job: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Punch {
    pub person_id: String,
    pub kind: PunchKind,
    pub timestamp: i64,
    pub label: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Workspace {
    pub tenant_id: String,
    pub organization: String,
    pub theme: String,
    pub modules: ModuleSet,
    pub people: Vec<Person>,
    pub punches: Vec<Punch>,
}
impl Workspace {
    pub fn authorize(&self, actor: &Actor) -> Result<(), String> {
        if actor.tenant_id != self.tenant_id || !self.people.iter().any(|p| p.id == actor.person_id)
        {
            return Err("Access to this company is not authorized.".into());
        }
        Ok(())
    }
    pub fn state(&self, person: &str) -> WorkState {
        match self
            .punches
            .iter()
            .rev()
            .find(|p| p.person_id == person)
            .map(|p| p.kind)
        {
            Some(PunchKind::In | PunchKind::BreakEnd) => WorkState::Working,
            Some(PunchKind::BreakStart) => WorkState::Paused,
            _ => WorkState::Off,
        }
    }
    pub fn set_module(&mut self, actor: &Actor, id: ModuleId, enabled: bool) -> Result<(), String> {
        self.authorize(actor)?;
        if actor.role != Role::Admin {
            return Err("Only an administrator can configure modules.".into());
        }
        if !enabled
            && id == ModuleId::Attendance
            && self
                .people
                .iter()
                .any(|p| self.state(&p.id) != WorkState::Off)
        {
            return Err("Close all open work sessions before disabling attendance.".into());
        }
        self.modules.set(id, enabled)
    }
    pub fn punch(
        &mut self,
        actor: &Actor,
        kind: PunchKind,
        timestamp: i64,
        label: String,
    ) -> Result<(), String> {
        self.authorize(actor)?;
        if !self.modules.contains(ModuleId::Attendance) {
            return Err("The attendance module is disabled.".into());
        }
        self.state(&actor.person_id).next(kind)?;
        if self
            .punches
            .iter()
            .rev()
            .find(|p| p.person_id == actor.person_id)
            .is_some_and(|p| p.timestamp > timestamp)
        {
            return Err("The timestamp is earlier than the last time entry.".into());
        }
        self.punches.push(Punch {
            person_id: actor.person_id.clone(),
            kind,
            timestamp,
            label,
        });
        Ok(())
    }
    /// Sum closed work intervals; an active interval is excluded until a pause/exit.
    /// UTC timestamps make overnight work and clock offset changes unambiguous.
    pub fn recorded_seconds(&self, person: &str) -> i64 {
        let mut start = None;
        let mut total = 0;
        for event in self.punches.iter().filter(|p| p.person_id == person) {
            match event.kind {
                PunchKind::In | PunchKind::BreakEnd => start = Some(event.timestamp),
                PunchKind::BreakStart | PunchKind::Out => {
                    if let Some(value) = start.take() {
                        total += (event.timestamp - value).max(0);
                    }
                }
            }
        }
        total
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub workspace: Workspace,
    pub actor_id: String,
    pub catalog: Vec<ModuleDefinition>,
    pub server_time: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Workspace, Actor) {
        let actor = Actor {
            tenant_id: "a".into(),
            person_id: "me".into(),
            role: Role::Admin,
        };
        let workspace = Workspace {
            tenant_id: "a".into(),
            organization: "Test".into(),
            theme: "sage".into(),
            modules: ModuleSet::preset("complete").unwrap(),
            people: vec![Person {
                id: "me".into(),
                name: "Test".into(),
                initials: "TE".into(),
                department: "Test".into(),
                job: "Test".into(),
            }],
            punches: vec![],
        };
        (workspace, actor)
    }
    #[test]
    fn identity_resolves_only_an_active_membership_in_the_selected_company() {
        let identity = UserIdentity {
            id: "user-1".into(),
        };
        let memberships = vec![
            Membership {
                user_id: "user-1".into(),
                tenant_id: "company-a".into(),
                person_id: "person-a".into(),
                role: Role::Admin,
                active: true,
            },
            Membership {
                user_id: "user-1".into(),
                tenant_id: "company-b".into(),
                person_id: "person-b".into(),
                role: Role::Employee,
                active: true,
            },
            Membership {
                user_id: "user-1".into(),
                tenant_id: "company-c".into(),
                person_id: "person-c".into(),
                role: Role::Admin,
                active: false,
            },
        ];

        let actor = identity.actor_for("company-b", &memberships).unwrap();
        assert_eq!(actor.tenant_id, "company-b");
        assert_eq!(actor.person_id, "person-b");
        assert_eq!(actor.role, Role::Employee);
        assert!(identity.actor_for("company-c", &memberships).is_err());

        let stranger = UserIdentity {
            id: "user-2".into(),
        };
        assert!(stranger.actor_for("company-a", &memberships).is_err());
    }

    #[test]
    fn dependency_changes_are_atomic() {
        let mut modules = ModuleSet::preset("complete").unwrap();
        assert!(modules.set(ModuleId::Attendance, false).is_err());
        assert!(modules.contains(ModuleId::Attendance));
        modules.set(ModuleId::Reports, false).unwrap();
        modules.set(ModuleId::Attendance, false).unwrap();
        assert!(modules.set(ModuleId::Reports, true).is_err());
        assert!(!modules.contains(ModuleId::Reports));
    }
    #[test]
    fn custom_module_lists_validate_dependencies_and_duplicates() {
        let modules = ModuleSet::from_enabled(vec![
            ModuleId::People,
            ModuleId::Attendance,
            ModuleId::Reports,
        ])
        .unwrap();
        assert!(modules.contains(ModuleId::Reports));
        assert!(ModuleSet::from_enabled(vec![ModuleId::Reports]).is_err());
        assert!(ModuleSet::from_enabled(vec![ModuleId::People, ModuleId::People]).is_err());
    }

    #[test]
    fn full_shift_excludes_pause_and_handles_midnight() {
        let (mut w, a) = fixture();
        for (kind, timestamp) in [
            (PunchKind::In, 82_800),
            (PunchKind::BreakStart, 86_400),
            (PunchKind::BreakEnd, 90_000),
            (PunchKind::Out, 93_600),
        ] {
            w.punch(&a, kind, timestamp, "UTC".into()).unwrap();
        }
        assert_eq!(w.recorded_seconds("me"), 7_200);
        assert_eq!(w.state("me"), WorkState::Off);
    }
    #[test]
    fn double_entry_and_reversed_clock_are_rejected() {
        let (mut w, a) = fixture();
        w.punch(&a, PunchKind::In, 100, "UTC".into()).unwrap();
        assert!(w.punch(&a, PunchKind::In, 101, "UTC".into()).is_err());
        assert!(w.punch(&a, PunchKind::Out, 99, "UTC".into()).is_err());
        assert_eq!(w.punches.len(), 1);
    }
    #[test]
    fn foreign_tenant_and_employee_configuration_are_denied() {
        let (mut w, mut a) = fixture();
        a.tenant_id = "other".into();
        assert!(w.punch(&a, PunchKind::In, 100, "UTC".into()).is_err());
        a.tenant_id = "a".into();
        a.role = Role::Employee;
        assert!(w.set_module(&a, ModuleId::Reports, false).is_err());
    }
    #[test]
    fn disabled_module_cannot_be_used_and_data_is_retained() {
        let (mut w, a) = fixture();
        w.punch(&a, PunchKind::In, 10, "UTC".into()).unwrap();
        assert!(w.set_module(&a, ModuleId::Attendance, false).is_err());
        w.punch(&a, PunchKind::Out, 20, "UTC".into()).unwrap();
        w.set_module(&a, ModuleId::Reports, false).unwrap();
        w.set_module(&a, ModuleId::Attendance, false).unwrap();
        assert!(w.punch(&a, PunchKind::In, 30, "UTC".into()).is_err());
        assert_eq!(w.punches.len(), 2);
    }
    #[test]
    fn leaving_during_pause_does_not_count_pause_as_work() {
        let (mut w, a) = fixture();
        for (kind, timestamp) in [
            (PunchKind::In, 0),
            (PunchKind::BreakStart, 60),
            (PunchKind::Out, 120),
        ] {
            w.punch(&a, kind, timestamp, "UTC".into()).unwrap();
        }
        assert_eq!(w.recorded_seconds("me"), 60);
    }
}
