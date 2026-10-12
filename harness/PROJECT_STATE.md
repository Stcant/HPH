# HPH — Stato operativo

## Obiettivo e fase
Costruire un gestionale aziendale modulare e riutilizzabile, con frontend/backend Rust e un'interfaccia curata. La preferenza Rust anche per il frontend sostituisce la scelta frontend aperta nel kickoff.

Attività corrente: primo template full-stack locale. Stato: **implemented**; controlli automatici superati, QA browser ancora mancante. Non rappresenta il completamento dell'intero gestionale.
Checkpoint: 2026-10-12. Repository privata consteuni/HPH; branch main. Base remota ispezionata: d32316c0b91532bb65f398bb2caa9c754ce0da1e. Implementazione e verifiche in una copia locale dei nuovi sorgenti, salvataggio tramite API GitHub; nessuna verifica di checkout Git locale.

## Decisioni
Leptos + Axum; core di dominio indipendente; catalogo e dipendenze dei moduli; CSS responsive e temi; PostgreSQL + SQLx previsti per la fase successiva. Dettagli e confini in [PROJECT_MAP.md](PROJECT_MAP.md).

## Risultato implementato
- Dashboard, persone demo in sola lettura, timbrature, riepiloghi e configurazione moduli.
- Preset essential/team/complete e temi sage/indigo in hph.toml.
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

## Problemi rilevanti risolti
- Un'interruzione dell'esecutore ha invalidato una prima esecuzione dei test; i risultati validi provengono dalle esecuzioni successive.
- Il rendering SSR dei componenti profondamente annidati produceva uno stack overflow. Confini dei componenti resi AnyView; successivamente tutti i 9 test sono passati con la configurazione standard, senza aumentare lo stack.

## Limiti e lavoro aperto
La demo perde dati e configurazioni interattive al riavvio. I profili sono sintetici e il ruolo amministratore è fisso. Il modello tenant del dominio non costituisce isolamento di un servizio reale. Nessuna pubblicazione o esposizione esterna.
Ferie, permessi, turni, CRUD dipendenti, rettifiche/audit, idempotenza, esportazioni e regole contrattuali rimangono da implementare. Il flag GitHub Template repository non è stato modificato.

## Prossima azione
Eseguire la demo in un browser e validare interazioni/resa desktop e mobile; poi implementare autenticazione, membership e persistenza PostgreSQL come prima sezione operativa multi-azienda, con test reali di isolamento e concorrenza.

## File e lettura
Codice: Cargo.toml/Cargo.lock, rust-toolchain.toml, crates/hph-core, src e style. Configurazione: hph.toml. Avvio e limiti: [README](../README.md). Estensioni: [MODULES](../docs/MODULES.md). Conservare questi due file harness come stato e mappa canonici.
