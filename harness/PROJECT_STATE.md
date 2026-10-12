# HPH — Stato operativo

## Obiettivo
Costruire con l'utente un gestionale aziendale in Rust, con timbrature, presenze e un'interfaccia particolarmente curata. Definire una prima versione completa prima di estendere i moduli.

## Attività corrente
Avvio e definizione del prodotto. Stato: in_progress.
Checkpoint: 2026-10-12, data della sessione; accesso tramite connettore GitHub, nessun checkout locale verificato.
Repository privata consteuni/HPH, branch main. Presente README.md nell'ispezione iniziale.

## Requisiti e proposte
Requisiti confermati e proposta di moduli/direzione visiva sono nella [mappa](PROJECT_MAP.md). Non considerare le proposte approvate né implementate.

## Piano breve
1. Chiarire Rust frontend/backend e singola azienda/multi-azienda.
2. Definire flusso iniziale, ruoli, regole di timbratura e criteri osservabili.
3. Selezionare stack e proporre una prima interfaccia con dati sintetici.
4. Implementare una sezione completa con persistenza e autorizzazioni; verificare i casi di errore.
5. Estendere a richieste assenze e gestione presenze sulla base del feedback.

## Criteri della prima sezione proposta
| Criterio | Risultato | Evidenza o motivo |
| --- | --- | --- |
| Persistenza di entrata e uscita e visualizzazione delle proprie presenze | NOT RUN | Applicazione non implementata |
| Accesso alle presenze limitato secondo ruolo e ambito aziendale | NOT RUN | Modello organizzativo da definire |
| Timbrature duplicate, sequenze non valide e rettifiche gestite esplicitamente | NOT RUN | Regole da definire e testare |
| Interfaccia utilizzabile da mobile e desktop con stati errore/vuoto | NOT RUN | Interfaccia non implementata |

## Verifiche effettive
Connettore GitHub: metadati confermano visibilità privata e permessi pull/push; elenco radice restituisce README.md. Queste osservazioni verificano accesso e contenuti iniziali, non il funzionamento dell'applicazione.
Nessun build, test applicativo, database o deployment eseguito.

## File del kickoff
- harness/PROJECT_MAP.md
- harness/PROJECT_STATE.md

## Scelte aperte
- Rust anche per il frontend oppure solo per il backend?
- Una sola azienda oppure servizio per più aziende?
- Piattaforma proposta: web responsive; confermare eventuali vincoli diversi.

## Prossima azione
Ottenere le prime due scelte dall'utente, quindi definire stack e prima sezione implementabile. Non inventare requisiti di paghe, contabilità o contratti di lavoro.
