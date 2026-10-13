# HPH — Mappa del progetto

## Requisiti confermati
- Progetto personale dell'utente: gestionale aziendale con timbrature e presenze, estendibile a ulteriori funzioni.
- Template riutilizzabile: ciascuna installazione/azienda deve poter scegliere i moduli.
- Ogni azienda deve poter comporre il proprio gestionale attivando i moduli adatti ai suoi processi; la selezione appartiene all'azienda e deve diventare persistente.
- Dopo iniziale libertà tecnologica, l'utente ha espresso preferenza per tutta l'applicazione in Rust. Scelta adottata: Rust frontend e backend.
- Qualità estetica e cura dell'esperienza utente sono requisiti centrali.
- Repository pubblica di portfolio: https://github.com/Stcant/HPH, branch main. Nessuna distribuzione dell'app richiesta.

## Decisioni architetturali
| Area | Scelta e motivo |
| --- | --- |
| Frontend | Leptos 0.8, SSR + hydration WASM: componenti e contratti in Rust, una sola applicazione |
| HTTP | Axum 0.8 + Tokio, integrazione ufficiale Leptos |
| Organizzazione | Monolite modulare: un deploy, regole indipendenti in hph-core, niente microservizi iniziali |
| Moduli | Catalogo tipizzato, dipendenze dichiarate e abilitazione controllata sul server; configurazione persistente per azienda nel prossimo incremento |
| Configurazione | hph.toml per azienda, tema, preset o lista esplicita di moduli; le modifiche runtime demo sono temporanee |
| Design | CSS con token, verde salvia e alternativa indigo; sidebar, card, tabelle e layout responsive |
| Persistenza | Schema PostgreSQL e policy RLS testate in CI; pool SQLx, migrazioni e repository moduli. Le schermate usano ancora stato in memoria; l'API attende un'identità autenticata |
| Identità | Contratti Rust per identità utente e membership attiva per azienda con ruolo; resolver seleziona l'attore per tenant. Login, sessioni e storage membership restano da implementare |

Compromesso: Rust end-to-end condivide tipi e competenze, ma comporta build WASM e un ecosistema UI meno ampio rispetto a React. Leptos genera un loader JavaScript; il codice applicativo resta Rust. Non promettere assenza assoluta di JavaScript nel browser.

## Implementazione iniziale
- `crates/hph-core/src/lib.rs`: catalogo e dipendenze, identità e membership aziendali, macchina a stati delle timbrature, calcolo di intervalli chiusi e test.
- `migrations/0001_company_data.sql`: schema PostgreSQL multi-azienda e policy RLS; `migrations/tests/tenant_isolation.sql` verifica visibilità e scritture cross-company.
- `src/database.rs`: pool SQLx, migrator runtime, ping e transazioni con contesto RLS transaction-local.
- `src/module_repository.rs`: lettura/scrittura atomica di moduli aziendali sotto RLS, membership attiva e ruolo admin. Non è ancora collegato alle server function finché manca una sessione autenticata con UUID verificati.

- `src/app.rs`: dashboard, anagrafica dimostrativa, timbrature, report, configurazione moduli e componenti condivisi.
- `src/api.rs`: server function snapshot, configurazione e timbratura. L'identità dell'attore viene dal contesto server.
- `src/server.rs`: configurazione, stato demo in memoria, router Axum e test HTTP/SSR; health check PostgreSQL opzionale.
- `src/main.rs`: avvio esclusivamente con HPH_DEMO=true e ascolto loopback; pool runtime e migrator separato sono opzionali via environment.
- `style/main.css`: estetica, token e adattamento mobile; nessun font o asset remoto richiesto.
- `Cargo.lock`: versioni risolte; `rust-toolchain.toml`: toolchain e target.
- `.github/workflows/ci.yml`: formattazione, test SSR, controllo WASM e integrazione PostgreSQL.

## Moduli
La configurazione iniziale dell'azienda può usare un preset o una lista esplicita validata in hph.toml; la UI può cambiare la selezione durante la demo. In produzione la configurazione runtime deve essere persistita e autorizzata per organizzazione.
Core sempre attivo. Persone contiene profili sintetici in sola lettura. Presenze dipende da Persone; Report dipende da Presenze. I moduli si disattivano senza cancellare eventi; sessioni aperte impediscono la disattivazione di Presenze. Report mostra il tempo degli intervalli chiusi della sessione, pause escluse.

## Confini attuali
Questa è una demo locale con una membership amministrativa sintetica. I contratti identità/membership risolvono l'attore per tenant e lo schema PostgreSQL con RLS e il relativo test sono stati aggiunti; la verifica CI è in corso. Login, sessioni, repository SQL e isolamento HTTP non sono ancora integrati. Il tenant è modellato e testato nel dominio, ma questo non prova un servizio multi-azienda sicuro. Dati e modifiche ai moduli si azzerano al riavvio. Aggiornamento tra schede manuale.

Il percorso verso più aziende richiede membership verificata, scope tenant obbligatorio su query/mutazioni, migrazioni, vincoli e test di isolamento nel database. Nessun dato reale prima di questa fase.

Timestamp UTC, orologio del server e sequenze validate. Calendario aziendale, fuso di visualizzazione, regole contrattuali, arrotondamenti, rettifiche/audit e idempotenza di richieste ritentate richiedono implementazione dedicata.

## Modello di personalizzazione
HPH Core resta condiviso; i moduli sono funzionalità Rust versionate nel template, selezionabili dall'amministratore dell'azienda e protette sul server. Disattivarli nasconde e blocca le operazioni senza cancellare i dati. La demo applica la scelta a un solo workspace in memoria. La configurazione persistente deve essere associata all'organizzazione; il modello di distribuzione (istanza dedicata o servizio multi-azienda) resta una scelta futura, quindi il confine tenant va mantenuto esplicito fin da subito.

## Evoluzione del prodotto
Primo flusso operativo target: autenticazione, anagrafica reale, timbrature persistenti e controllo del responsabile. Poi ferie/permessi e calendario presenze. Turni, documenti, note spese, commesse, clienti, fatturazione, magazzino e integrazioni rimangono moduli candidati. Paghe e contabilità richiedono specifiche autonome.

## Lettura
- [Stato operativo](PROJECT_STATE.md): evidenza e prossimo passo.
- [Avvio e limiti](../README.md).
- [Contratto dei moduli](../docs/MODULES.md).

Riferimenti primari consultati: https://book.leptos.dev/getting_started/index.html ; https://book.leptos.dev/ssr/21_cargo_leptos.html ; https://github.com/leptos-rs/start-axum ; https://docs.rs/axum/latest/axum/ ; https://www.postgresql.org/docs/current/ddl-rowsecurity.html .
