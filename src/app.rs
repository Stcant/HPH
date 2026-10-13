use crate::api::{configure_module, get_snapshot, register_punch};
use hph_core::{ModuleId, PunchKind, Snapshot, WorkState};
use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    StaticSegment,
    components::{Route, Router, Routes},
};

pub fn shell(options: LeptosOptions) -> AnyView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <meta name="theme-color" content="#173d32"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body><App/></body>
        </html>
    }
    .into_any()
}

#[component]
pub fn App() -> AnyView {
    provide_meta_context();
    view! {
        <Stylesheet id="leptos" href="/pkg/hph.css"/>
        <Title text="HPH — Work, in harmony"/>
        <Router>
            <Routes fallback=|| view! { <main class="empty"><h1>"This page does not exist"</h1><a href="/">"Back to HPH"</a></main> }>
                <Route path=StaticSegment("") view=WorkspacePage/>
            </Routes>
        </Router>
    }.into_any()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Overview,
    People,
    Attendance,
    Reports,
    Modules,
}
impl Screen {
    fn title(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::People => "People",
            Self::Attendance => "Attendance",
            Self::Reports => "Reports",
            Self::Modules => "Your workspace",
        }
    }
}

#[derive(Clone, Copy)]
enum Command {
    Module(ModuleId, bool),
    Punch(PunchKind),
}

fn dispatch(
    command: Command,
    busy: RwSignal<bool>,
    message: RwSignal<Option<(bool, String)>>,
    refresh: Callback<()>,
) {
    if busy.get_untracked() {
        return;
    }
    busy.set(true);
    message.set(None);
    leptos::task::spawn_local(async move {
        let result = match command {
            Command::Module(id, enabled) => configure_module(id, enabled).await,
            Command::Punch(kind) => register_punch(kind).await,
        };
        match result {
            Ok(()) => {
                message.set(Some((true, "Change saved for this demo session.".into())));
                refresh.run(());
            }
            Err(error) => message.set(Some((false, error.to_string()))),
        }
        busy.set(false);
    });
}

#[component]
fn WorkspacePage() -> AnyView {
    let snapshot = Resource::new(|| (), |_| get_snapshot());
    let screen = RwSignal::new(Screen::Overview);
    let busy = RwSignal::new(false);
    let message = RwSignal::new(None::<(bool, String)>);
    let refresh = Callback::new(move |()| snapshot.refetch());
    view! {
        <a class="skip" href="#main">"Skip to content"</a>
        <Suspense fallback=|| view! { <main class="empty" role="status"><div class="brand-mark">"h"</div><h1>"Getting your workspace ready…"</h1></main> }>
            {move || snapshot.get().map(|result| match result {
                Ok(data) => view! { <WorkspaceView data screen busy message refresh/> }.into_any(),
                Err(error) => view! { <main class="empty"><h1>"We could not load your workspace"</h1><p>{error.to_string()}</p><button class="primary" on:click=move |_| snapshot.refetch()>"Try again"</button></main> }.into_any(),
            })}
        </Suspense>
    }.into_any()
}

#[component]
fn Icon(#[prop(default = "grid")] name: &'static str) -> AnyView {
    let path = match name {
        "people" => {
            "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2M16 3a4 4 0 0 1 0 8M22 21v-2a4 4 0 0 0-3-3.87M13 7a4 4 0 1 1-8 0 4 4 0 0 1 8 0"
        }
        "clock" => "M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0M12 6v6l4 2",
        "chart" => "M4 3v18h17M9 16v-4M14 16V8M19 16V5",
        "modules" => "M3 3h7v7H3zM14 3h7v7h-7zM3 14h7v7H3zM17.5 14v7M14 17.5h7",
        "arrow" => "M5 12h14M13 6l6 6-6 6",
        "check" => "M5 12l4 4L19 6",
        "spark" => "M12 3l2.7 6.3L21 12l-6.3 2.7L12 21l-2.7-6.3L3 12l6.3-2.7L12 3",
        _ => "M3 3h7v7H3zM14 3h7v7h-7zM3 14h7v7H3zM14 14h7v7h-7z",
    };
    view! { <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.65" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d=path/></svg> }.into_any()
}

#[component]
fn NavItem(target: Screen, screen: RwSignal<Screen>, icon: &'static str) -> AnyView {
    view! {
        <button class="nav-item" class:active=move || screen.get() == target aria-current=move || if screen.get() == target { Some("page") } else { None } on:click=move |_| screen.set(target)>
            <Icon name=icon/><span>{target.title()}</span>
        </button>
    }.into_any()
}

#[component]
fn WorkspaceView(
    data: Snapshot,
    screen: RwSignal<Screen>,
    busy: RwSignal<bool>,
    message: RwSignal<Option<(bool, String)>>,
    refresh: Callback<()>,
) -> AnyView {
    let organization = data.workspace.organization.clone();
    let theme = data.workspace.theme.clone();
    let modules = data.workspace.modules.clone();
    let people_enabled = modules.contains(ModuleId::People);
    let attendance_enabled = modules.contains(ModuleId::Attendance);
    let reports_enabled = modules.contains(ModuleId::Reports);
    let data = StoredValue::new(data);
    view! {
        <div class="workspace" data-theme=theme>
            <aside class="sidebar" aria-label="Main navigation">
                <a href="/" class="brand" aria-label="HPH home"><span class="brand-mark">"h"</span><span>"hph"<span class="brand-dot">"."</span></span></a>
                <div class="org"><span class="org-icon">"A"</span><div><strong>{organization}</strong><small>"Demo workspace"</small></div></div>
                <div class="nav-heading">"WORKSPACE"</div>
                <nav>
                    <NavItem target=Screen::Overview screen icon="grid"/>
                    {people_enabled.then(|| view! { <NavItem target=Screen::People screen icon="people"/> })}
                    {attendance_enabled.then(|| view! { <NavItem target=Screen::Attendance screen icon="clock"/> })}
                    {reports_enabled.then(|| view! { <NavItem target=Screen::Reports screen icon="chart"/> })}
                    <div class="nav-divider"></div>
                    <NavItem target=Screen::Modules screen icon="modules"/>
                </nav>
                <div class="sidebar-bottom"><div class="build-note"><Icon name="spark"/><strong>"Room for your ideas."</strong><p>"Build your workspace around the way you work."</p><button on:click=move |_| screen.set(Screen::Modules)>"Explore modules →"</button></div>
                <div class="profile"><span class="avatar">"EC"</span><div><strong>"Elena Conti"</strong><small>"Demo administrator"</small></div><span class="online-dot"></span></div></div>
            </aside>
            <div class="main-wrap">
                <header class="topbar"><div class="breadcrumb">"Workspace"<span>"/"</span><strong>{move || screen.get().title()}</strong></div><div class="top-actions"><span class="demo-badge">"LOCAL DEMO"</span><button class="icon-button" aria-label="Refresh data" disabled=move || busy.get() on:click=move |_| refresh.run(())>"↻"</button></div></header>
                <main id="main">
                    <div class="demo-notice">"Sample data · no real accounts · changes reset when the server restarts."</div>
                    <div role="status" aria-live="polite">{move || message.get().map(|(ok, text)| view! { <div class="notice" class:error=!ok>{text}</div> })}</div>
                    {move || {
                        let snapshot = data.get_value();
                        match screen.get() {
                            Screen::Overview => view! { <Overview data=snapshot screen busy message refresh/> }.into_any(),
                            Screen::People if people_enabled => view! { <People data=snapshot/> }.into_any(),
                            Screen::Attendance if attendance_enabled => view! { <Attendance data=snapshot busy message refresh/> }.into_any(),
                            Screen::Reports if reports_enabled => view! { <Reports data=snapshot/> }.into_any(),
                            Screen::Modules => view! { <Modules data=snapshot busy message refresh/> }.into_any(),
                            _ => view! { <section class="empty"><h1>"This module is disabled"</h1><p>"You can enable it again in your workspace. Session data is preserved."</p><button class="primary" on:click=move |_| screen.set(Screen::Modules)>"Configure modules"</button></section> }.into_any(),
                        }
                    }}
                    <footer>"HPH · Work, in harmony."<span>"Preview 0.1 · Rust / Leptos"</span></footer>
                </main>
            </div>
        </div>
    }.into_any()
}

#[component]
fn Overview(
    data: Snapshot,
    screen: RwSignal<Screen>,
    busy: RwSignal<bool>,
    message: RwSignal<Option<(bool, String)>>,
    refresh: Callback<()>,
) -> AnyView {
    let attendance = data.workspace.modules.contains(ModuleId::Attendance);
    let people = data.workspace.modules.contains(ModuleId::People);
    let working = data
        .workspace
        .people
        .iter()
        .filter(|p| data.workspace.state(&p.id) == WorkState::Working)
        .count();
    let total = data.workspace.people.len();
    let active = data.workspace.modules.enabled.len();
    let events = data.workspace.punches.len();
    view! {
        <div class="page-heading"><div><p class="eyebrow">"YOUR WORKSPACE"</p><h1>"Everything in its place."</h1><p>"People, time and tools. Finally together."</p></div><button class="secondary" on:click=move |_| screen.set(Screen::Modules)><Icon name="modules"/>"Customize workspace"</button></div>
        <section class="hero"><div><div class="hero-kicker"><span class="tiny-dot"></span>"GOOD WORK STARTS HERE"</div><h2>"Less complexity."<br/>"More room for people."</h2><p>"A workspace that grows with you. Choose what your team needs and build the rest, one module at a time."</p><button class="hero-button" on:click=move |_| screen.set(Screen::Modules)>"Find your balance"<Icon name="arrow"/></button></div><div class="hero-art" aria-hidden="true"><div class="arch arch-one"></div><div class="arch arch-two"></div><div class="arch arch-three"></div><div class="floating-card"><span class="icon-tile"><Icon name="check"/></span><div><strong>"Your team."</strong><span>"Your way of working."</span></div></div><span class="hero-star">"✳"</span></div></section>
        <div class="stats-grid">
            <Stat title="People on the team" value=if people { total.to_string() } else { "—".into() } note=if people { "Demo profiles" } else { "Module disabled" } icon="people"/>
            <Stat title="Working now" value=if attendance { working.to_string() } else { "—".into() } note="From this session’s time entries" icon="clock"/>
            <Stat title="Recorded events" value=if attendance { events.to_string() } else { "—".into() } note="Clock-ins, breaks and clock-outs" icon="chart"/>
            <Stat title="Active modules" value=format!("{active} / 3") note="A workspace that fits" icon="modules"/>
        </div>
        <div class="content-grid"><section class="panel"><div class="panel-heading"><div><h2>"Your team"</h2><p>"People at the heart of every day."</p></div>{people.then(|| view! { <button class="text-button" on:click=move |_| screen.set(Screen::People)>"View everyone →"</button> })}</div>
        {if people { view! { <PeopleTable data=data.clone()/> }.into_any() } else { view! { <div class="empty small"><Icon name="people"/><p>"Enable the People module to get started."</p></div> }.into_any() }}</section>
        <section class="panel time-panel"><div class="panel-heading"><div><p class="eyebrow">"YOUR TIME"</p><h2>"One click. You’re clocked in."</h2></div><Icon name="clock"/></div>
        {if attendance { view! { <PunchCard data=data.clone() busy message refresh/> }.into_any() } else { view! { <p>"Enable Time and attendance to record your day."</p> }.into_any() }}
        </section></div>
        <section class="bottom-banner"><div class="icon-tile"><Icon name="modules"/></div><div><h3>"Business software built around you."</h3><p>"Enable the modules you need. A shared foundation with room to grow."</p></div><button class="text-button" on:click=move |_| screen.set(Screen::Modules)>"Configure HPH →"</button></section>
    }.into_any()
}

#[component]
fn Stat(title: &'static str, value: String, note: &'static str, icon: &'static str) -> AnyView {
    view! { <section class="stat"><div><span>{title}</span><Icon name=icon/></div><strong>{value}</strong><small>{note}</small></section> }.into_any()
}

#[component]
fn PeopleTable(data: Snapshot) -> AnyView {
    let attendance = data.workspace.modules.contains(ModuleId::Attendance);
    let rows = data.workspace.people.iter().map(|person| {
        let state = data.workspace.state(&person.id);
        view! { <tr><td><div class="person"><span class="avatar">{person.initials.clone()}</span><div><strong>{person.name.clone()}</strong><small>{person.job.clone()}</small></div></div></td><td>{person.department.clone()}</td><td><span class="status" class:working=state==WorkState::Working class:paused=state==WorkState::Paused>{if attendance { state.label() } else { "Not tracked" }}</span></td></tr> }
    }).collect_view();
    view! { <div class="table-scroll"><table><caption class="sr-only">"People and attendance status in the demo session"</caption><thead><tr><th scope="col">"PERSON"</th><th scope="col">"DEPARTMENT"</th><th scope="col">"STATUS"</th></tr></thead><tbody>{rows}</tbody></table></div> }.into_any()
}

#[component]
fn People(data: Snapshot) -> AnyView {
    view! { <div class="page-heading"><div><p class="eyebrow">"PEOPLE"</p><h1>"Your team makes the difference."</h1><p>"Demo directory. Profile management is planned for a future update."</p></div><span class="pill">{format!("{} people", data.workspace.people.len())}</span></div><section class="panel"><PeopleTable data/></section> }.into_any()
}

#[component]
fn PunchCard(
    data: Snapshot,
    busy: RwSignal<bool>,
    message: RwSignal<Option<(bool, String)>>,
    refresh: Callback<()>,
) -> AnyView {
    let state = data.workspace.state(&data.actor_id);
    let buttons: Vec<PunchKind> = match state {
        WorkState::Off => vec![PunchKind::In],
        WorkState::Working => vec![PunchKind::BreakStart, PunchKind::Out],
        WorkState::Paused => vec![PunchKind::BreakEnd, PunchKind::Out],
    };
    let last = data
        .workspace
        .punches
        .iter()
        .rev()
        .find(|p| p.person_id == data.actor_id)
        .map(|p| p.label.clone());
    view! {
        <div class="clock-face"><div class="clock-ring"><Icon name="clock"/></div><span class="status" class:working=state==WorkState::Working class:paused=state==WorkState::Paused>{state.label()}</span><h3>{match state { WorkState::Off => "Ready when you are.", WorkState::Working => "Have a good day, Elena.", WorkState::Paused => "Enjoy your break." }}</h3><p>{last.map(|value| format!("Last event: {value}")).unwrap_or_else(|| "No time entries in this session.".into())}</p></div>
        <div class="punch-actions">{buttons.into_iter().map(|kind| view! { <button class="primary" disabled=move || busy.get() on:click=move |_| dispatch(Command::Punch(kind), busy, message, refresh)>{kind.label()}<Icon name="arrow"/></button> }).collect_view()}</div>
        <p class="microcopy">"Server-recorded time · UTC · demo profile Elena Conti"</p>
    }.into_any()
}

#[component]
fn Attendance(
    data: Snapshot,
    busy: RwSignal<bool>,
    message: RwSignal<Option<(bool, String)>>,
    refresh: Callback<()>,
) -> AnyView {
    let events = data.workspace.punches.iter().rev().map(|p| view! { <li><span class="event-dot"></span><div><strong>{p.kind.label()}</strong><small>{p.label.clone()}</small></div><span class="pill">"Elena Conti"</span></li> }).collect_view();
    let empty = data.workspace.punches.is_empty();
    view! { <div class="page-heading"><div><p class="eyebrow">"ATTENDANCE"</p><h1>"Every day has its own rhythm."</h1><p>"Record clock-ins, breaks and clock-outs. All times are shown in UTC."</p></div></div><div class="attendance-grid"><section class="panel time-panel"><h2>"Your time entry"</h2><PunchCard data=data.clone() busy message refresh/></section><section class="panel"><div class="panel-heading"><div><h2>"Session activity"</h2><p>"Events are validated and recorded by the server."</p></div></div>{empty.then(|| view! { <div class="empty small"><Icon name="clock"/><h3>"A new day awaits"</h3><p>"Clock in for the first time and your entry will appear here."</p></div> })}<ol class="event-list">{events}</ol></section></div> }.into_any()
}

#[component]
fn Reports(data: Snapshot) -> AnyView {
    let rows = data.workspace.people.iter().map(|p| {
        let seconds = data.workspace.recorded_seconds(&p.id);
        view! { <tr><td>{p.name.clone()}</td><td>{format!("{}h {:02}m {:02}s", seconds / 3600, seconds % 3600 / 60, seconds % 60)}</td><td>{data.workspace.state(&p.id).label()}</td></tr> }
    }).collect_view();
    view! { <div class="page-heading"><div><p class="eyebrow">"REPORTS"</p><h1>"A clearer view of time."</h1><p>"A summary of the demo session, calculated from time entries."</p></div></div><section class="panel"><div class="panel-heading"><div><h2>"Recorded time per person"</h2><p>"Only intervals closed by a break or clock-out. Breaks and the current open interval are excluded."</p></div></div><div class="table-scroll"><table><thead><tr><th scope="col">"PERSON"</th><th scope="col">"RECORDED TIME"</th><th scope="col">"STATUS"</th></tr></thead><tbody>{rows}</tbody></table></div></section><p class="microcopy">"This summary does not apply contract rules, rounding, overtime or payroll calculations."</p> }.into_any()
}

#[component]
fn Modules(
    data: Snapshot,
    busy: RwSignal<bool>,
    message: RwSignal<Option<(bool, String)>>,
    refresh: Callback<()>,
) -> AnyView {
    let cards = data.catalog.into_iter().map(|definition| {
        let id = definition.id;
        let enabled = data.workspace.modules.contains(id);
        let dependency = match id { ModuleId::People => "No dependencies", ModuleId::Attendance => "Requires People", ModuleId::Reports => "Requires Time and attendance" };
        let icon = match id { ModuleId::People => "people", ModuleId::Attendance => "clock", ModuleId::Reports => "chart" };
        let label = format!("{} {}", if enabled { "Disable" } else { "Enable" }, definition.name);
        view! { <article class="module-card" class:enabled><div class="module-top"><span class="icon-tile"><Icon name=icon/></span><span class="pill" class:active-pill=enabled>{if enabled { "Active" } else { "Available" }}</span></div><h2>{definition.name}</h2><p>{definition.description}</p><small>{dependency}</small><button class="module-toggle" role="switch" aria-label=label aria-checked=enabled.to_string() disabled=move || busy.get() on:click=move |_| dispatch(Command::Module(id, !enabled), busy, message, refresh)><span>{if enabled { "Module enabled" } else { "Enable module" }}</span><span class="switch" class:on=enabled><span></span></span></button></article> }
    }).collect_view();
    view! { <div class="page-heading"><div><p class="eyebrow">"YOUR WORKSPACE"</p><h1>"Choose what you need."</h1><p>"A solid foundation. The right modules. A workspace that feels like yours."</p></div></div><section class="core-banner"><span class="icon-tile"><Icon name="check"/></span><div><h2>"HPH Core"</h2><p>"Configuration, shared design and module rules. Always included."</p></div><span class="pill">"Included"</span></section><div class="module-grid">{cards}</div><section class="panel next-modules"><p class="eyebrow">"BUILDING WHAT COMES NEXT"</p><h2>"The next chapter."</h2><p>"Leave, shifts, documents, expenses and projects are possible extensions. They are not implemented yet."</p><div class="roadmap-tags"><span>"Leave and time off"</span><span>"Shifts"</span><span>"Documents"</span><span>"Projects"</span></div></section><p class="microcopy">"Dependencies are checked on the server. Disabling a module preserves its data; attendance requires all open sessions to be closed."</p> }.into_any()
}
