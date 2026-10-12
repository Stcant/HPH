# HPH — Mappa del progetto

## Requisiti confermati
- Progetto personale dell'utente: gestionale aziendale con timbrature e presenze, estendibile ad altre funzioni aziendali.
- Linguaggio richiesto: Rust. Estensione al frontend ancora da chiarire.
- Qualità estetica e cura dell'esperienza utente sono requisiti centrali.
- Repository: https://github.com/consteuni/HPH, privata; branch predefinito main.

## Stato delle conoscenze
Ispezione remota del 2026-10-12: repository iniziale con README.md; nessuna applicazione o architettura implementata osservata. Questa mappa descrive una proposta, non funzionalità disponibili.

## Prima versione proposta — da validare
| Area | Flusso |
| --- | --- |
| Azienda e persone | Dipendenti, reparti, sedi e ruoli dipendente/responsabile/amministratore |
| Timbrature | Entrata, uscita, pause e riepilogo personale |
| Presenze | Vista giornaliera e mensile, ore registrate e anomalie |
| Assenze | Richiesta ferie/permessi, approvazione e calendario |
| Amministrazione | Riepiloghi ed esportazione dei dati presenze |

Prima sezione funzionante proposta: accesso con ruoli, registrazione entrata/uscita, visualizzazione delle proprie presenze e controllo autorizzato del responsabile.

## Direzione visiva proposta
Web app responsive, desktop per amministrazione e mobile per dipendenti. Tipografia leggibile, spaziatura generosa, palette coerente, tabelle curate, calendario chiaro e dashboard con dati operativi. Disegnare anche stati vuoti, caricamento, errori e conferme; navigazione da tastiera, focus visibile e informazioni non affidate al solo colore. Tema, riferimenti grafici e piattaforma sono ancora da scegliere.

## Confini tecnici da definire
- Rust per backend oppure anche per interfaccia; framework e database non selezionati.
- Singola azienda oppure prodotto multi-azienda con isolamento dei dati.
- Web/mobile browser come proposta, app native o terminali fisici non confermati.
- Orari, fusi, turni notturni, pause e correzioni devono avere regole esplicite prima di implementare il relativo calcolo.
- Proposta: controlli dei permessi sul server, registrazione delle rettifiche senza cancellare la storia, prevenzione di timbrature duplicate.
- Usare dati sintetici durante sviluppo e dimostrazioni.

## Evoluzione candidata, non approvata
Turni, documenti, note spese, progetti/commesse, clienti, fatturazione, magazzino e integrazioni. Calcolo paghe e contabilità non fanno parte della prima versione proposta e richiedono requisiti specifici.

## Lettura
- [Stato operativo](PROJECT_STATE.md): attività corrente, verifiche e prossimo passo.
- [README](../README.md): documento iniziale della repository.
