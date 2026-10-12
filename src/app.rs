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
        <html lang="it">
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
        <Title text="HPH — Il lavoro, in armonia"/>
        <Router>
            <Routes fallback=|| view! { <main class="empty"><h1>"Questa pagina non esiste"</h1><a href="/">"Torna a HPH"</a></main> }>
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
            Self::Overview => "Panoramica",
            Self::People => "Persone",
            Self::Attendance => "Presenze",
            Self::Reports => "Report",
            Self::Modules => "Il tuo workspace",
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
                message.set(Some((
                    true,
                    "Modifica registrata nella sessione demo.".into(),
                )));
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
        <a class="skip" href="#main">"Vai al contenuto"</a>
        <Suspense fallback=|| view! { <main class="empty" role="status"><div class="brand-mark">"h"</div><h1>"Prepariamo il tuo workspace…"</h1></main> }>
            {move || snapshot.get().map(|result| match result {
                Ok(data) => view! { <WorkspaceView data screen busy message refresh/> }.into_any(),
                Err(error) => view! { <main class="empty"><h1>"Non riusciamo a caricare il workspace"</h1><p>{error.to_string()}</p><button class="primary" on:click=move |_| snapshot.refetch()>"Riprova"</button></main> }.into_any(),
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
            <aside class="sidebar" aria-label="Navigazione principale">
                <a href="/" class="brand" aria-label="HPH, pagina iniziale"><span class="brand-mark">"h"</span><span>"hph"<span class="brand-dot">"."</span></span></a>
                <div class="org"><span class="org-icon">"A"</span><div><strong>{organization}</strong><small>"Workspace dimostrativo"</small></div></div>
                <div class="nav-heading">"WORKSPACE"</div>
                <nav>
                    <NavItem target=Screen::Overview screen icon="grid"/>
                    {people_enabled.then(|| view! { <NavItem target=Screen::People screen icon="people"/> })}
                    {attendance_enabled.then(|| view! { <NavItem target=Screen::Attendance screen icon="clock"/> })}
                    {reports_enabled.then(|| view! { <NavItem target=Screen::Reports screen icon="chart"/> })}
                    <div class="nav-divider"></div>
                    <NavItem target=Screen::Modules screen icon="modules"/>
                </nav>
                <div class="sidebar-bottom"><div class="build-note"><Icon name="spark"/><strong>"Spazio alle tue idee."</strong><p>"Componi il gestionale intorno al tuo modo di lavorare."</p><button on:click=move |_| screen.set(Screen::Modules)>"Esplora i moduli →"</button></div>
                <div class="profile"><span class="avatar">"EC"</span><div><strong>"Elena Conti"</strong><small>"Amministratrice demo"</small></div><span class="online-dot"></span></div></div>
            </aside>
            <div class="main-wrap">
                <header class="topbar"><div class="breadcrumb">"Workspace"<span>"/"</span><strong>{move || screen.get().title()}</strong></div><div class="top-actions"><span class="demo-badge">"DEMO LOCALE"</span><button class="icon-button" aria-label="Aggiorna i dati" disabled=move || busy.get() on:click=move |_| refresh.run(())>"↻"</button></div></header>
                <main id="main">
                    <div class="demo-notice">"Dati di esempio · nessun account reale · le modifiche si azzerano al riavvio del server."</div>
                    <div role="status" aria-live="polite">{move || message.get().map(|(ok, text)| view! { <div class="notice" class:error=!ok>{text}</div> })}</div>
                    {move || {
                        let snapshot = data.get_value();
                        match screen.get() {
                            Screen::Overview => view! { <Overview data=snapshot screen busy message refresh/> }.into_any(),
                            Screen::People if people_enabled => view! { <People data=snapshot/> }.into_any(),
                            Screen::Attendance if attendance_enabled => view! { <Attendance data=snapshot busy message refresh/> }.into_any(),
                            Screen::Reports if reports_enabled => view! { <Reports data=snapshot/> }.into_any(),
                            Screen::Modules => view! { <Modules data=snapshot busy message refresh/> }.into_any(),
                            _ => view! { <section class="empty"><h1>"Questo modulo è disattivato"</h1><p>"Puoi riattivarlo dal tuo workspace. I dati della sessione vengono conservati."</p><button class="primary" on:click=move |_| screen.set(Screen::Modules)>"Configura i moduli"</button></section> }.into_any(),
                        }
                    }}
                    <footer>"HPH · Il lavoro, in armonia."<span>"Anteprima 0.1 · Rust / Leptos"</span></footer>
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
        <div class="page-heading"><div><p class="eyebrow">"IL TUO SPAZIO DI LAVORO"</p><h1>"Tutto al posto giusto."</h1><p>"Persone, tempo e strumenti. Finalmente insieme."</p></div><button class="secondary" on:click=move |_| screen.set(Screen::Modules)><Icon name="modules"/>"Personalizza workspace"</button></div>
        <section class="hero"><div><div class="hero-kicker"><span class="tiny-dot"></span>"UN BUON LAVORO INIZIA QUI"</div><h2>"Meno complessità."<br/>"Più spazio alle persone."</h2><p>"Un workspace che cresce con te. Scegli ciò che serve al tuo team e costruisci il resto, un modulo alla volta."</p><button class="hero-button" on:click=move |_| screen.set(Screen::Modules)>"Crea il tuo equilibrio"<Icon name="arrow"/></button></div><div class="hero-art" aria-hidden="true"><div class="arch arch-one"></div><div class="arch arch-two"></div><div class="arch arch-three"></div><div class="floating-card"><span class="icon-tile"><Icon name="check"/></span><div><strong>"Il tuo team."</strong><span>"Il tuo modo di lavorare."</span></div></div><span class="hero-star">"✳"</span></div></section>
        <div class="stats-grid">
            <Stat title="Persone nel team" value=if people { total.to_string() } else { "—".into() } note=if people { "Profili dimostrativi" } else { "Modulo non attivo" } icon="people"/>
            <Stat title="Al lavoro adesso" value=if attendance { working.to_string() } else { "—".into() } note="Da timbrature della sessione" icon="clock"/>
            <Stat title="Eventi registrati" value=if attendance { events.to_string() } else { "—".into() } note="Entrate, pause e uscite" icon="chart"/>
            <Stat title="Moduli attivi" value=format!("{active} / 3") note="Un workspace su misura" icon="modules"/>
        </div>
        <div class="content-grid"><section class="panel"><div class="panel-heading"><div><h2>"Il tuo team"</h2><p>"Le persone al centro, ogni giorno."</p></div>{people.then(|| view! { <button class="text-button" on:click=move |_| screen.set(Screen::People)>"Tutte le persone →"</button> })}</div>
        {if people { view! { <PeopleTable data=data.clone()/> }.into_any() } else { view! { <div class="empty small"><Icon name="people"/><p>"Attiva il modulo Persone per iniziare."</p></div> }.into_any() }}</section>
        <section class="panel time-panel"><div class="panel-heading"><div><p class="eyebrow">"IL TUO TEMPO"</p><h2>"Un gesto. Sei al lavoro."</h2></div><Icon name="clock"/></div>
        {if attendance { view! { <PunchCard data=data.clone() busy message refresh/> }.into_any() } else { view! { <p>"Attiva Timbrature e presenze per registrare la tua giornata."</p> }.into_any() }}
        </section></div>
        <section class="bottom-banner"><div class="icon-tile"><Icon name="modules"/></div><div><h3>"Il gestionale segue te. Non il contrario."</h3><p>"Attiva solo i moduli utili. Una base condivisa, possibilità da costruire."</p></div><button class="text-button" on:click=move |_| screen.set(Screen::Modules)>"Configura HPH →"</button></section>
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
        view! { <tr><td><div class="person"><span class="avatar">{person.initials.clone()}</span><div><strong>{person.name.clone()}</strong><small>{person.job.clone()}</small></div></div></td><td>{person.department.clone()}</td><td><span class="status" class:working=state==WorkState::Working class:paused=state==WorkState::Paused>{if attendance { state.label() } else { "Non rilevato" }}</span></td></tr> }
    }).collect_view();
    view! { <div class="table-scroll"><table><caption class="sr-only">"Persone e stato di presenza della sessione demo"</caption><thead><tr><th scope="col">"PERSONA"</th><th scope="col">"REPARTO"</th><th scope="col">"STATO"</th></tr></thead><tbody>{rows}</tbody></table></div> }.into_any()
}

#[component]
fn People(data: Snapshot) -> AnyView {
    view! { <div class="page-heading"><div><p class="eyebrow">"PERSONE"</p><h1>"Il valore è nel team."</h1><p>"Anagrafica dimostrativa. La gestione dei profili arriverà nel prossimo incremento."</p></div><span class="pill">{format!("{} persone", data.workspace.people.len())}</span></div><section class="panel"><PeopleTable data/></section> }.into_any()
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
        <div class="clock-face"><div class="clock-ring"><Icon name="clock"/></div><span class="status" class:working=state==WorkState::Working class:paused=state==WorkState::Paused>{state.label()}</span><h3>{match state { WorkState::Off => "Pronta quando vuoi.", WorkState::Working => "Buon lavoro, Elena.", WorkState::Paused => "Prenditi il tuo tempo." }}</h3><p>{last.map(|value| format!("Ultimo evento: {value}")).unwrap_or_else(|| "Nessuna timbratura nella sessione.".into())}</p></div>
        <div class="punch-actions">{buttons.into_iter().map(|kind| view! { <button class="primary" disabled=move || busy.get() on:click=move |_| dispatch(Command::Punch(kind), busy, message, refresh)>{kind.label()}<Icon name="arrow"/></button> }).collect_view()}</div>
        <p class="microcopy">"Ora registrata dal server · UTC · profilo demo Elena Conti"</p>
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
    view! { <div class="page-heading"><div><p class="eyebrow">"PRESENZE"</p><h1>"Ogni giornata ha il suo ritmo."</h1><p>"Registra entrate, pause e uscite. Tutti gli orari sono espressi in UTC."</p></div></div><div class="attendance-grid"><section class="panel time-panel"><h2>"La tua timbratura"</h2><PunchCard data=data.clone() busy message refresh/></section><section class="panel"><div class="panel-heading"><div><h2>"Attività della sessione"</h2><p>"Gli eventi vengono controllati e registrati dal server."</p></div></div>{empty.then(|| view! { <div class="empty small"><Icon name="clock"/><h3>"Una nuova giornata ti aspetta"</h3><p>"Registra la prima entrata: comparirà qui."</p></div> })}<ol class="event-list">{events}</ol></section></div> }.into_any()
}

#[component]
fn Reports(data: Snapshot) -> AnyView {
    let rows = data.workspace.people.iter().map(|p| {
        let seconds = data.workspace.recorded_seconds(&p.id);
        view! { <tr><td>{p.name.clone()}</td><td>{format!("{}h {:02}m {:02}s", seconds / 3600, seconds % 3600 / 60, seconds % 60)}</td><td>{data.workspace.state(&p.id).label()}</td></tr> }
    }).collect_view();
    view! { <div class="page-heading"><div><p class="eyebrow">"REPORT"</p><h1>"Il tempo, con chiarezza."</h1><p>"Riepilogo della sessione demo, calcolato dalle timbrature."</p></div></div><section class="panel"><div class="panel-heading"><div><h2>"Tempo registrato per persona"</h2><p>"Solo intervalli chiusi da una pausa o un'uscita. Pause escluse; intervallo in corso non conteggiato."</p></div></div><div class="table-scroll"><table><thead><tr><th scope="col">"PERSONA"</th><th scope="col">"TEMPO REGISTRATO"</th><th scope="col">"STATO"</th></tr></thead><tbody>{rows}</tbody></table></div></section><p class="microcopy">"Questo riepilogo non applica regole contrattuali, arrotondamenti, straordinari o calcoli paghe."</p> }.into_any()
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
        let dependency = match id { ModuleId::People => "Nessuna dipendenza", ModuleId::Attendance => "Richiede Persone", ModuleId::Reports => "Richiede Timbrature e presenze" };
        let icon = match id { ModuleId::People => "people", ModuleId::Attendance => "clock", ModuleId::Reports => "chart" };
        let label = format!("{} {}", if enabled { "Disattiva" } else { "Attiva" }, definition.name);
        view! { <article class="module-card" class:enabled><div class="module-top"><span class="icon-tile"><Icon name=icon/></span><span class="pill" class:active-pill=enabled>{if enabled { "Attivo" } else { "Disponibile" }}</span></div><h2>{definition.name}</h2><p>{definition.description}</p><small>{dependency}</small><button class="module-toggle" role="switch" aria-label=label aria-checked=enabled.to_string() disabled=move || busy.get() on:click=move |_| dispatch(Command::Module(id, !enabled), busy, message, refresh)><span>{if enabled { "Modulo attivo" } else { "Attiva modulo" }}</span><span class="switch" class:on=enabled><span></span></span></button></article> }
    }).collect_view();
    view! { <div class="page-heading"><div><p class="eyebrow">"IL TUO WORKSPACE"</p><h1>"Scegli ciò che ti serve."</h1><p>"Una base solida. I moduli giusti. Un gestionale che ti somiglia."</p></div></div><section class="core-banner"><span class="icon-tile"><Icon name="check"/></span><div><h2>"HPH Core"</h2><p>"Configurazione, design condiviso e regole dei moduli. Sempre presente."</p></div><span class="pill">"Incluso"</span></section><div class="module-grid">{cards}</div><section class="panel next-modules"><p class="eyebrow">"DA COSTRUIRE INSIEME"</p><h2>"Il prossimo capitolo."</h2><p>"Ferie e permessi, turni, documenti, note spese e commesse sono possibili estensioni. Non sono ancora implementate."</p><div class="roadmap-tags"><span>"Ferie e permessi"</span><span>"Turni"</span><span>"Documenti"</span><span>"Commesse"</span></div></section><p class="microcopy">"Le dipendenze vengono verificate sul server. Disattivare un modulo non elimina i dati; le presenze richiedono la chiusura delle sessioni aperte."</p> }.into_any()
}
