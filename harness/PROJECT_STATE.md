# HPH — Stato operativo

## Obiettivo e fase
Costruire un gestionale aziendale modulare e riutilizzabile, con frontend/backend Rust e un'interfaccia curata. La preferenza Rust anche per il frontend sostituisce la scelta frontend aperta nel kickoff.

Incremento corrente: verifica del repository moduli con ruolo PostgreSQL non proprietario. Stato: **in progress** sul branch `test/non-owner-module-repository`; attendere CI. PR #5 è stata integrata in main con merge `0d9033ffdc951c24d2ce9ea16c66debcbbf93fb8`; i controlli su quel commit erano verdi. Il progetto complessivo resta in_progress; QA browser ancora mancante.
Checkpoint originario: 2026-10-12. Aggiornamento puntamenti, direzione prodotto e configurazione moduli: 2026-10-13. Repository pubblica Stcant/HPH (portfolio); branch feat/sqlx-database-foundation. Base remota ispezionata: d32316c0b91532bb65f398bb2caa9c754ce0da1e. Implementazione e verifiche in una copia locale dei nuovi sorgenti, salvataggio tramite API GitHub; nessuna verifica di checkout Git locale.

## Decisioni
Leptos + Axum; core di dominio indipendente; catalogo e dipendenze dei moduli; CSS responsive e temi; PostgreSQL con SQLx per pool, migrazioni e contesto tenant transazionale; repository applicativi restano da integrare. La lista iniziale dei moduli è configurazione persistente per installazione, validata all'avvio. Le modifiche runtime demo restano temporanee; PostgreSQL dovrà persistere la configurazione per organizzazione. Il modello di deploy (istanza dedicata o multi-azienda) resta aperto. Dettagli e confini in [PROJECT_MAP.md](PROJECT_MAP.md).

## Risultato implementato
- Dashboard, persone demo in sola lettura, timbrature, riepiloghi e configurazione moduli.
- Preset essential/team/complete e temi sage/indigo in hph.toml; lista esplicita di moduli aziendali supportata come override persistente del preset, con dipendenze e duplicati validati all'avvio.
- Server function reali e stato in memoria: entrata, pausa, rientro, uscita; sequenze e orario verificati dal server.
- Attivazione con dipendenze e permesso amministratore nel dominio; disattivazione senza cancellazione degli eventi e senza interrompere sessioni aperte.
- Avvio solo con HPH_DEMO=true e binding loopback. Un attore e un'azienda sintetici: nessuna autenticazione o persistenza reale.
- README di avvio, contratto di estensione moduli e workflow CI.

## Evidenza del 2026-10-12
Scope: sorgenti del primo template salvati insieme a questo checkpoint; test applicati prima del commit remoto. La cronologia del commit identifica i file. Nessun test browser o database è implicato.

| Criterio | Risultato | Azione ed evidenza |
| --- | --- | --- |
| Regole moduli, tenant e timbrature | PASS | cargo test --workspace --features ssr --locked: 6 test hph-core superati |
| RPC reali e applicazione regole lato HTTP | PASS | Stesso comando: 2 test del router Axum superati; doppia entrata, modulo spento e dipendenze respinti |
| Rendering server della pagina completa | PASS | Stesso comando: 1 test SSR superato; nome azienda, avviso demo, contenuto e CSS presenti |
| Build frontend e backend | PASS | cargo-leptos 0.3.11 build: binario SSR, libreria WASM, CSS e loader generati; exit 0 |
| Formattazione | PASS | cargo fmt --all --check: exit 0 |
| Avvio demo esplicito e binding locale | PASS | Esecuzione del binario senza HPH_DEMO=true e con 0.0.0.0: rifiuto verificato in entrambi i casi |
| Aspetto, mobile, navigazione e hydration nel browser | NOT RUN | Browser QA non disponibile nell'ambiente; build e SSR non provano interazione o resa visiva |
| Autenticazione, PostgreSQL e isolamento multi-azienda | NOT RUN | Non implementati; appartengono al prossimo incremento |

## Incremento: selezione personalizzata dei moduli — 2026-10-13

Scope: override `modules` di `hph.toml`, mantenimento dei preset, validazione delle dipendenze e dei duplicati. Commit sorgenti: `8f350db4aec6c1dc3974c6d67086dd9079ace1da`.

| Criterio | Risultato | Evidenza |
| --- | --- | --- |
| TOML consente una lista aziendale valida che sostituisce il preset | PASS | Test `explicit_company_modules_override_preset_and_invalid_lists_fail`; TOML deserializzato, preset essential sovrascritto da people/attendance/reports |
| Dipendenze mancanti e ID duplicati sono rifiutati senza mutare la selezione | PASS | Test `custom_module_lists_validate_dependencies_and_duplicates` e test config invalida |
| Formattazione, test SSR e check WASM | PASS | [GitHub Actions run 38320041661](https://github.com/Stcant/HPH/actions/runs/38320041661), concluso success. Commit `8f350db4aec6c1dc3974c6d67086dd9079ace1da`; 2026-10-12T23:02:44Z–23:03:55Z. |

La selezione statica in `hph.toml` persiste tra riavvii; le modifiche dalla schermata demo restano in memoria.

## Incremento: identità e membership — 2026-10-13

Scope: contratti di dominio per identità utente e membership attiva per azienda; il server demo crea l'attore selezionando una membership esplicita. Non introduce login, sessioni o persistenza.

| Criterio | Risultato | Evidenza |
| --- | --- | --- |
| Un utente può avere ruoli differenti in più aziende | PASS | Test `identity_resolves_only_an_active_membership_in_the_selected_company`: seleziona la membership Employee corretta, rifiuta la membership inattiva e un utente senza membership |
| L'attore demo è derivato da una membership esplicita | PASS | `DemoState::new` chiama `UserIdentity::actor_for` |
| Formattazione, test SSR e check WASM | PASS | [GitHub Actions run 38454681554](https://github.com/Stcant/HPH/actions/runs/38454681554), tutti gli step conclusi success; commit testato `8097d605f6ed95547dec2688cd03ce61c09ee6e3` |

Le membership restano in memoria nella demo. L'identità verificata deve arrivare da un middleware di autenticazione e la membership dovrà essere letta dal database prima delle operazioni aziendali.

## Incremento: schema PostgreSQL multi-azienda — 2026-10-13

Scope: organizzazioni, identità esterne, persone, membership, moduli configurati e timbrature idempotenti; Row-Level Security scoped per azienda. Il repository SQL non è ancora integrato nell'app Rust.

| Criterio | Risultato | Evidenza |
| --- | --- | --- |
| Dati aziendali e coerenza tra organizzazioni | PASS | FK composte legano membership/persona e timbrature/persona alla stessa organizzazione; vincoli univoci e chiave idempotente per timbratura |
| Letture scoped, membership proprie e blocco insert/update/delete cross-company | PASS | Script `migrations/tests/tenant_isolation.sql` eseguito contro PostgreSQL 18 in CI, run 38456551456 |
| Test Rust, formattazione e check WASM | PASS | Job `checks` del run CI 38456551456 |

La runtime app dovrà impostare `hph.tenant_id` e `hph.user_id` transazionalmente, verificare membership e ruoli nel servizio e usare un ruolo non proprietario senza `BYPASSRLS`; questo schema da solo non prova la sicurezza delle operazioni HTTP.

## Incremento: schema PostgreSQL e isolamento tenant — 2026-10-13

Scope: schema iniziale per organizzazioni, identità esterne, persone, membership, moduli aziendali e timbrature; RLS forzata sulle tabelle aziendali; test SQL con ruolo runtime senza privilegi di bypass. Non introduce repository SQLx nell'app.

| Criterio | Risultato | Evidenza |
| --- | --- | --- |
| La migrazione PostgreSQL si applica | PASS | Job `database` del run CI 38456551456 |
| Le query tenant-scoped non espongono righe di un'altra azienda | PASS | Test automatici di visibilità, update/delete cross-company, insert cross-company e assenza del contesto tenant |
| Verifiche Rust e WASM restano verdi | PASS | Job `checks` dello stesso run CI |

## Incremento: fondazione SQLx applicativa — 2026-10-13

Scope verificato: pool PostgreSQL SQLx per il server SSR; migrator separabile con ruolo dedicato; helper che assegna user e tenant solo per la durata di una transazione; health check opzionale. Le server function continuano a usare lo stato demo in memoria.

| Criterio | Risultato | Evidenza |
| --- | --- | --- |
| Pool SQLx, migrazione e tenant context compilano | PASS | Run CI 38460315357: cargo fmt, test workspace e check WASM superati |
| Migrazione applicata due volte e pool raggiungibile su PostgreSQL | PASS | Job `database` del run 38460315357; `sqlx_pool_migrates_and_scopes_tenant_context` |
| Tenant e user context sono transaction-local | PASS | Il test legge entrambi i valori nella transazione e verifica il tenant nullo dopo il commit |
| CI Rust, SSR, WASM e RLS preesistente | PASS | Job `checks` e `database` del run [38460315357](https://github.com/Stcant/HPH/actions/runs/38460315357) sul commit `6b3964a4acef3b1241ead813f2980c9b9a140cba` |

## Incremento: repository SQLx per moduli aziendali — 2026-10-13

Aggiunto `src/module_repository.rs`: lettura tenant-scoped per membership attiva; scrittura atomica di tutti i flag, validazione dipendenze e controllo admin nella stessa transazione. Una selezione non ancora salvata è restituita come `None`; l'integrazione UI è rinviata finché l'identità demo sintetica non viene sostituita da UUID verificati di una sessione autenticata.

| Criterio | Risultato | Evidenza |
| --- | --- | --- |
| Repository limita query e scritture al tenant tramite contesto RLS | PASS | Test PostgreSQL `company_module_selection_is_tenant_scoped_and_admin_only` superato nel job database del run 38461693770 |
| Solo membership admin attiva può salvare la configurazione | PASS | Test PostgreSQL verifica round-trip admin e rifiuto per ruolo employee |
| Formattazione, test SSR e check WASM | PASS | Job `checks` del run CI 38461693770 superato |

## Verifica aggiuntiva: ruolo PostgreSQL runtime non proprietario — 2026-10-13

Il primo test del repository usava una connessione superuser e quindi non esercitava davvero le policy RLS. La correzione separa il pool admin per fixture/migrazioni dal pool runtime `hph_runtime`, creato con `NOSUPERUSER NOBYPASSRLS`; il test di integrazione prova le operazioni repository sotto il ruolo runtime.

| Criterio | Risultato | Evidenza |
| --- | --- | --- |
| Repository testato sotto RLS effettiva | PENDING | Nuova configurazione CI e test aggiunti; attendere Actions |
| Formattazione, test SSR e check WASM | PENDING | Branch `test/non-owner-module-repository` |

## Problemi rilevanti risolti
- Un'interruzione dell'esecutore ha invalidato una prima esecuzione dei test; i risultati validi provengono dalle esecuzioni successive.
- Il rendering SSR dei componenti profondamente annidati produceva uno stack overflow. Confini dei componenti resi AnyView; successivamente tutti i 9 test sono passati con la configurazione standard, senza aumentare lo stack.

## Limiti e lavoro aperto
La demo perde dati e configurazioni interattive al riavvio. I profili sono sintetici e il ruolo amministratore è fisso. Il modello tenant del dominio non costituisce isolamento di un servizio reale. Nessuna pubblicazione o esposizione esterna.
Ferie, permessi, turni, CRUD dipendenti, rettifiche/audit, idempotenza, esportazioni e regole contrattuali rimangono da implementare. Il flag GitHub Template repository non è stato modificato.

## Prossima azione
Dopo aver confermato il test RLS con ruolo runtime non proprietario, introdurre sessione autenticata e membership persistita, poi collegare il repository moduli all'API; non usare l'attore demo per query SQL tenant-scoped. Poi proseguire con repository per persone e presenze. Eseguire QA browser desktop/mobile quando il checkout locale sarà disponibile. Il clone locale non è disponibile in questo ambiente; il controllo Rust avviene tramite CI GitHub.

## File e lettura
Codice: Cargo.toml/Cargo.lock, rust-toolchain.toml, crates/hph-core, src e style. Configurazione: hph.toml. Avvio e limiti: [README](../README.md). Estensioni: [MODULES](../docs/MODULES.md). Conservare questi due file harness come stato e mappa canonici.
