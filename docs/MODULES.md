# Estendere HPH

## Contratto attuale

La composizione dei moduli è dichiarata in `crates/hph-core/src/lib.rs`: `ModuleId`, `catalog()` e `ModuleSet`. È una fonte condivisa compilata sia per il server sia per il browser. Non mantenere un secondo elenco JavaScript o un secondo motore di dipendenze nella UI.

La scelta dei moduli è una configurazione dell'azienda: HPH Core è sempre presente e ogni azienda abilita le funzioni adatte ai propri processi. I moduli restano codice Rust del template, compilato e versionato; non sono plugin di terze parti caricati a runtime. La demo mantiene la scelta solo in memoria; il prossimo livello persistente deve associarla all'organizzazione autenticata.

Il core è sempre presente. I moduli attuali formano una catena: Persone → Presenze → Report. Per attivare un modulo, le sue dipendenze devono essere attive; per disattivarne una, occorre disattivare prima i moduli che la usano. Una modifica non valida non altera lo stato precedente.

Disabilitare un modulo deve interromperne le operazioni protette e nascondere le schermate applicabili, **senza eliminare i dati**. Le sessioni di timbratura aperte impediscono di disattivare Presenze.

## Aggiungere un modulo

1. Aggiungere un identificatore tipizzato a `ModuleId` e una definizione in `catalog()` con dipendenze esplicite. Non creare cicli.
2. Definire le regole del dominio e i tipi serializzati nel core o, quando cresce, in un crate dedicato. Tenere Leptos, HTTP e database fuori dalle regole pure.
3. Implementare l'operazione server in `src/api.rs`. Ottenere l'attore dal contesto server, verificare appartenenza, permesso e attivazione prima della mutazione. Un pulsante nascosto non è un controllo di accesso.
4. Aggiungere la schermata e la navigazione in `src/app.rs`. Riutilizzare colori, componenti, stati vuoti ed errori esistenti.
5. Definire cosa succede alle operazioni aperte quando si disattiva il modulo. Non cancellare implicitamente documenti o eventi.
6. Scrivere test delle regole e del confine HTTP: operazione valida, permesso negato, azienda estranea, modulo spento, input errato e conservazione dei dati.
7. Aggiornare preset e documentazione solo se il nuovo modulo fa realmente parte di quel preset. Non mostrare come disponibile un modulo non implementato.

## Esempio futuro: ferie

Dipendenza: Persone. Requisiti da chiarire prima del codice: intervalli di date, fuso aziendale, calendario lavorativo, approvatore, sovrapposizioni e rettifiche. Una richiesta è associata all'azienda e al dipendente; l'approvazione richiede un permesso distinto dall'invio. Il saldo ferie richiede regole esplicite e non va ricavato da ipotesi sui contratti.

## Confine della demo

`DemoState` usa memoria condivisa protetta da mutex e un attore fisso. Il controllo del tenant nel dominio non equivale a un sistema multi-azienda sicuro: mancano autenticazione, verifica della membership a ogni richiesta e isolamento nel database.

La futura implementazione PostgreSQL deve usare transazioni per modifiche correlate, vincoli per impedire sessioni incompatibili, chiavi collegate al tenant e test di isolamento con almeno due aziende. L'abilitazione del modulo è una scelta aziendale, mentre il permesso è una proprietà dell'utente: entrambe vanno verificate.

Per preservare configurazioni e dati durante l'evoluzione del template, usare migrazioni versionate. Non ricopiare i dati di esempio sopra quelli di un'installazione esistente.
