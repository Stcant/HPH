# HPH

**Il lavoro, in armonia.** Una base riutilizzabile per un gestionale aziendale modulare, con frontend e backend scritti in Rust.

HPH è attualmente una **demo locale eseguibile**, non un gestionale pronto per dati reali. Il primo incremento dimostra il design, la composizione dei moduli e un flusso di timbratura controllato dal server.

## Stack

| Parte | Scelta |
| --- | --- |
| Interfaccia | Leptos 0.8, rendering server e hydration WebAssembly |
| Server | Axum 0.8 + Tokio |
| Regole condivise | Crate indipendente `hph-core` |
| Design | CSS responsive con token; temi sage e indigo, nessun font remoto |
| Build | cargo-leptos 0.3.11, Cargo.lock versionato |
| Persistenza attuale | Memoria del server, esclusivamente per la demo |
| Persistenza prevista | PostgreSQL + SQLx, da implementare insieme all'identità aziendale |

Il codice applicativo è Rust. Il browser usa anche CSS e il piccolo loader JavaScript generato da wasm-bindgen/Leptos: non serve mantenere una seconda applicazione React o TypeScript. Non sono richiesti Node o npm.

## Avvio locale

Prerequisiti: Git e Rust tramite [rustup](https://rustup.rs). Il file `rust-toolchain.toml` installa target WebAssembly e componenti richiesti.

```bash
git clone https://github.com/Stcant/HPH.git
cd HPH
cargo install cargo-leptos --version 0.3.11 --locked
HPH_DEMO=true cargo leptos watch
```

In PowerShell, impostare prima `$env:HPH_DEMO = "true"`, poi eseguire `cargo leptos watch`.

Aprire **http://127.0.0.1:3000**. La demo rifiuta l'avvio senza consenso esplicito `HPH_DEMO=true` o con un indirizzo di ascolto diverso da loopback. Non esporla tramite proxy, tunnel o rete pubblica: non è presente un sistema di autenticazione. L'integrazione PostgreSQL è facoltativa e limitata alla connessione, alle migrazioni e al health check; le schermate usano ancora dati in memoria. Vedi [docs/DATABASE.md](docs/DATABASE.md).

## Provare il flusso

1. Dalla dashboard, registrare **Entrata** per il profilo demo Elena Conti.
2. Aprire **Presenze**: registrare **Inizio pausa**, **Fine pausa**, **Uscita**. Gli eventi arrivano al server e sono visibili dopo un aggiornamento pagina.
3. Aprire **Il tuo workspace**, attivare **Report** e consultare il tempo degli intervalli chiusi. Gli intervalli ancora aperti non sono inclusi.
4. Provare a disattivare **Presenze** mentre Report è attivo: il server rifiuta la modifica perché Reports dipende da Attendance. Con una sessione aperta, le presenze non possono essere disattivate.
5. Disattivare Report, chiudere eventuali sessioni e disattivare Presenze. Il menu si aggiorna; riattivandolo, gli eventi della sessione rimangono.

Tutti i profili sono inventati. L'unico attore della demo è un'amministratrice fissa scelta dal server, non dal browser. Le modifiche ai moduli e gli eventi sono condivisi tra le schede della stessa istanza e **si perdono al riavvio del processo**. L'interfaccia non aggiorna in tempo reale altre schede: usare il pulsante di aggiornamento.

## Personalizzare il template

Modificare `hph.toml` e riavviare il server:

```toml
organization = "La tua azienda"
theme = "sage"       # sage | indigo
preset = "team"      # essential | team | complete
# modules = ["people", "attendance"] # opzionale: sovrascrive preset
```

`modules` è facoltativo e accetta gli ID `people`, `attendance` e `reports`. I moduli devono includere le proprie dipendenze; HPH rifiuta una configurazione non valida all'avvio. Se `modules` è omesso, usa il preset. Le scelte iniziali in `hph.toml` restano salvate; le attivazioni fatte dall'interfaccia demo valgono solo fino al riavvio.

| Preset | Moduli |
| --- | --- |
| essential | Persone |
| team | Persone + Timbrature e presenze |
| complete | Persone + Timbrature e presenze + Report |

I moduli sono composizioni del codice Rust compilato. L'amministratore può abilitarli o disabilitarli nell'interfaccia; aggiungere una nuova funzione richiede codice e una nuova build. Non è un marketplace di plugin caricati da terze parti.

La configurazione dei moduli appartiene all'azienda: ogni workspace compone HPH scegliendo ciò che serve ai propri processi. Nella demo le attivazioni restano in memoria e si azzerano al riavvio; la configurazione persistente per azienda è parte del prossimo incremento PostgreSQL.

La base può essere clonata/forkata e personalizzata. La creazione del codice riutilizzabile non imposta automaticamente il flag **Template repository** nelle impostazioni GitHub.

## Cosa funziona oggi

| Funzione | Stato |
| --- | --- |
| Dashboard responsive, schermate e temi | Implementati; QA browser ancora da eseguire |
| Elenco persone | Profili demo in sola lettura |
| Moduli e dipendenze | Configurazione eseguita e verificata sul server |
| Entrata / pausa / rientro / uscita | Controllo sequenza e orario sul server |
| Riepilogo del tempo | Intervalli chiusi, pause escluse, precisione al secondo |
| Ruoli e confine aziendale | Regole di dominio testate; nessuna autenticazione reale |
| Aziende multiple, database, utenti reali | Non implementati |
| Ferie, turni, rettifiche, audit, esportazioni | Non implementati |
| Paghe, contabilità, magazzino | Fuori dalla prima versione |

Gli eventi usano timestamp UTC; i riepiloghi coprono la sessione della demo, non una giornata contrattuale. Nessun calcolo automatico di straordinari, ferie maturate o busta paga.

## Verifiche

```bash
cargo fmt --all --check
cargo test --workspace --features ssr --locked
cargo check --target wasm32-unknown-unknown --features hydrate --lib --locked
cargo leptos build
```

I test del dominio coprono dipendenze, permessi, azienda errata, doppia entrata, orario retrogrado, pause, disattivazione e conservazione degli eventi. I test HTTP esercitano le vere server function attraverso il router Axum e controllano il rendering SSR. Il job PostgreSQL della CI applica le policy RLS e prova pool SQLx, migrazioni e contesto tenant contro PostgreSQL reale. Non sono test browser né provano operazioni aziendali persistite.

## Struttura

- `crates/hph-core/`: moduli, ruoli, regole di timbratura, contratti serializzati e test.
- `src/app.rs`: componenti e schermate Leptos.
- `src/api.rs`: server function, senza accettare l'identità aziendale dal client.
- `src/server.rs`: configurazione e stato della demo, router e test HTTP.
- `src/database.rs`: pool PostgreSQL SQLx, migrazioni, ping e transazioni scoped per tenant.
- `style/main.css`: layout, token, temi e adattamento mobile.
- `docs/MODULES.md`: contratto per estendere il template.
- `harness/PROJECT_MAP.md` e `harness/PROJECT_STATE.md`: architettura e checkpoint.

Il prossimo incremento è **persistenza applicativa**: repository SQLx per membership, persone, timbrature e configurazione moduli, poi autenticazione e autorizzazioni reali. Fino ad allora HPH resta una demo locale e non deve ospitare dati aziendali reali.
