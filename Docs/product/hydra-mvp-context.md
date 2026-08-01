# Hydra

> **One repository. Many heads.**

Hydra è un workspace manager Git-native e local-first che materializza più copie operative isolate dello stesso repository come directory complete e indipendenti.

Permette a sviluppatori e agenti AI di lavorare contemporaneamente su più attività senza condividere working tree, index o diff, continuando a usare normalmente Git, l’IDE e gli strumenti già presenti sul computer.

---

## 1. Contesto

Con l’avvento della programmazione agentica, una parte crescente della codifica può essere delegata a strumenti come Codex, Claude Code, Gemini CLI, Aider o agli agenti integrati negli IDE.

Il ruolo dello sviluppatore si sposta verso:

- definizione dei requisiti;
- decisioni architetturali;
- coordinamento di più attività;
- revisione delle modifiche;
- controllo del versionamento;
- integrazione del lavoro prodotto.

Questo rende concretamente possibile lavorare su più feature nello stesso momento. Il limite non è più soltanto la velocità con cui viene scritto il codice, ma la capacità di mantenere separati e comprensibili i diversi flussi di lavoro.

In un unico working tree, attività parallele producono facilmente:

- una sola diff composta da modifiche non correlate;
- cambi di branch continui;
- file temporanei condivisi;
- agenti che intervengono sugli stessi file;
- difficoltà nel capire quale attività abbia introdotto una modifica;
- commit meno atomici e più difficili da revisionare;
- perdita di controllo sullo stato complessivo del progetto.

## 2. Obiettivo

Hydra deve permettere di lavorare a più feature dello stesso progetto in parallelo, mantenendo ogni attività fisicamente e logicamente isolata.

Dal punto di vista dell’utente, ogni **Head** deve apparire come un progetto completo sul filesystem. Può quindi essere aperta con qualsiasi IDE, terminale, agente o altro software senza richiedere integrazioni specifiche con Hydra.

Dal punto di vista dell’implementazione, Hydra deve condividere tutto ciò che Git può condividere in sicurezza e conservare separato ciò che determina lo stato operativo di ogni attività.

Hydra virtualizza quindi gli aspetti utili di un team che lavora su più computer:

- directory di lavoro;
- `HEAD`;
- index;
- modifiche non committate;
- diff;
- branch operativo;
- contesto aperto nell’IDE o nell’agente.

Non virtualizza necessariamente, almeno nell’MVP:

- sistema operativo;
- database;
- cache;
- container;
- servizi esterni;
- dipendenze installate.

## 3. Definizione del prodotto

> **Hydra è un workspace manager Git-native che materializza più Head isolate dello stesso repository come directory complete e indipendenti, consentendo a persone e agenti di sviluppare in parallelo senza condividere working tree o diff.**

Hydra non è:

- un IDE;
- un agente AI;
- un’alternativa a Git;
- un ambiente cloud;
- un sistema di virtualizzazione completo;
- un process manager, almeno nel suo nucleo.

Hydra coordina e rende accessibili workspace isolati. Gli strumenti esterni continuano a lavorare direttamente sui normali file del progetto.

---

## 4. Principi fondamentali

### Local-first

Il codice e i metadati operativi rimangono sul computer dello sviluppatore. Il funzionamento fondamentale non dipende da un servizio cloud.

### Git-native

Hydra utilizza Git come fonte primaria della verità. Branch, commit, merge, rebase e diff rimangono standard e accessibili anche senza Hydra.

### Tool-agnostic

Ogni Head è una directory reale. Può essere aperta con VS Code, Cursor, Windsurf, Zed, JetBrains, Vim o qualsiasi altro strumento capace di lavorare su una directory.

Lo stesso principio vale per Codex, Claude Code, Gemini CLI, Aider e agenti custom.

### Isolamento esplicito

Ogni Head possiede un working tree, un index e una ref Git indipendenti. Una modifica in una Head non deve alterare il filesystem operativo delle altre.

### Efficienza

Hydra non riclona l’intero repository per ogni Head. Condivide l’object database di Git e, quando il volume lo consente, usa primitive copy-on-write per condividere anche i blocchi fisici dei file materializzati.

L’obiettivo è che ogni Head occupi inizialmente soprattutto lo spazio delle proprie differenze, pur continuando ad apparire come una directory completa a qualunque programma.

Le directory operative con migliaia di file devono essere elaborate senza
avviare un processo Git per ogni file. Hydra raggruppa l'hashing degli overlay
in batch limitati, esegue batch indipendenti con un numero limitato di worker e
riusa un unico flusso Git batch quando deve leggere più blob tracciati,
preservando ordine dei risultati, controlli di contenuto, isolamento e
sicurezza.

### Nessuna sincronizzazione implicita

Una Head nasce da un commit preciso e non cambia automaticamente quando il branch sorgente avanza. Aggiornamento, rebase e integrazione devono essere azioni esplicite.

### Recuperabilità

La perdita o corruzione dei metadati di Hydra non deve rendere irrecuperabili branch o working tree. Lo stato deve poter essere riconciliato a partire da Git.

---

## 5. Modello tecnico

La primitiva fondamentale è `git worktree`.

Ogni Head è un worktree Git completo sul filesystem, mentre repository e Head condividono l’object database. Un livello di materializzazione separato determina come i file visibili vengono creati:

```text
Repository Git
├── object database condiviso
├── workspace originale
└── Hydra
    ├── Git worktree metadata
    ├── Materializer
    │   ├── native CoW backend
    │   └── full-copy fallback
    ├── Head payment
    ├── Head auth
    └── Head refactor
```

Ogni Head conserva comunque:

- un branch privato;
- un `HEAD` indipendente;
- un index indipendente;
- un working tree indipendente;
- file normali e direttamente accessibili sul filesystem.

### Collocazione delle Head

Per default, le Head di un progetto vengono create in una directory sorella del repository:

```text
<directory-genitore>/
├── Heimdall/
│   ├── .git/
│   ├── .hydra.json
│   └── ...
└── Heimdall.heads/
    ├── chatbot/
    ├── report-kpi/
    └── refactor/
```

La relazione rimane immediatamente visibile:

```text
Heimdall/        → progetto principale
Heimdall.heads/  → Head appartenenti a Heimdall
```

Le Head devono rimanere esterne al working tree principale perché una collocazione interna:

- potrebbe essere inclusa accidentalmente nei file tracciati o negli overlay;
- introdurrebbe il rischio di materializzazioni ricorsive;
- farebbe indicizzare tutte le Head ai watcher, all’IDE e agli strumenti del progetto principale;
- renderebbe più facile creare una Head dentro un’altra Head;
- confonderebbe la dimensione e lo stato operativo del repository principale.

Hydra non usa invece un contenitore globale del tipo `.hydra-heads/<progetto>/` come default. Quel layout renderebbe possibile raccogliere più progetti nello stesso namespace, ma aggiungerebbe un livello che non serve al ciclo di vita di una singola repository e renderebbe meno evidente la relazione tra progetto e Head.

Più progetti collocati nella stessa directory avranno quindi directory sorelle indipendenti:

```text
WorkingArea/
├── Heimdall/
├── Heimdall.heads/
│   ├── chatbot/
│   └── report-kpi/
├── AltroProgetto/
└── AltroProgetto.heads/
    └── autenticazione/
```

Il percorso rimane configurabile. Hydra salva in `.hydra.json` il percorso concreto, preferibilmente relativo alla root del progetto, e non deriva nuovamente la destinazione a ogni comando.

Se la directory predefinita esiste già ma non appartiene al progetto corrente, `hydra init` deve interrompersi senza riutilizzarla e richiedere una destinazione differente. Una directory delle Head già inizializzata può essere riconosciuta tramite i metadati Git e Hydra.

### Separazione tra isolamento Git e materializzazione

`git worktree` risolve l’isolamento Git, ma da solo scrive una copia completa dei file visibili per ogni working tree.

Il **Materializer** risolve invece l’efficienza fisica:

- cerca un file già materializzato con lo stesso contenuto;
- crea un clone copy-on-write quando il filesystem lo supporta;
- usa una copia normale quando il clone non è disponibile;
- tratta con lo stesso modello sia i file tracciati sia gli overlay;
- non cambia il modo in cui IDE, agenti, compilatori e Git vedono i file.

L’agnosticità è quindi garantita verso gli strumenti che usano le Head. I backend di storage possono essere specifici per piattaforma, ma rimangono un dettaglio interno.

### Perché non semplici copie, clone o hard link

| Soluzione | File normali | Scritture isolate | Git standard | Spazio efficiente | Scelta |
|---|---:|---:|---:|---:|---|
| Copia integrale | Sì | Sì | Sì | No | Fallback |
| Clone Git per Head | Sì | Sì | Sì | Parzialmente | No |
| Hard link | Sì | **No** | Fragile | Sì | **Mai per file modificabili** |
| Filesystem virtuale Hydra | Dipende | Sì | Dipende | Sì | Fuori dall’MVP |
| Worktree + clone CoW | Sì | Sì | Sì | Sì | **Preferito** |

Un hard link non implementa il copy-on-write. Due path collegati puntano allo stesso inode e una scrittura in-place effettuata in una Head può alterare immediatamente le altre. Un watcher riceverebbe normalmente l’evento troppo tardi per impedire la propagazione.

Gli hard link possono essere valutati in futuro soltanto per contenuti dichiarati immutabili e protetti in sola lettura. Non sono un backend generale di Hydra.

### Copy-on-write nativo

Con un clone copy-on-write, i path sono file logicamente indipendenti ma inizialmente condividono gli stessi blocchi fisici:

```text
Base/app.php ──────┬──── blocchi condivisi
Head A/app.php ────┤
Head B/app.php ────┘

Dopo una modifica in Head A:

Base/app.php ─────────── blocchi originali
Head B/app.php ───────── blocchi originali
Head A/app.php ───────── blocchi modificati propri
```

La separazione avviene prima della scrittura ed è garantita dal filesystem. Hydra non deve intercettare gli editor né mantenere una copia logica delle patch per ricostruire i file.

Backend iniziali:

| Piattaforma/filesystem | Primitiva preferita |
|---|---|
| macOS su APFS | clone file nativo |
| Linux su Btrfs/XFS e volumi compatibili | reflink (`FICLONE`) |
| Altri volumi | copia completa |

Il supporto va rilevato sul volume effettivo che conterrà le Head, non soltanto in base al sistema operativo. Una primitiva disponibile sulla piattaforma può fallire tra volumi diversi o su un filesystem che non la implementa.

### Sorgenti basate sul contenuto

Hydra non deve collegare una Head a un particolare path sorgente. Per ogni file cerca una sorgente esistente con contenuto identico:

- per i file tracciati, l’identità primaria è il blob Git atteso dal `baseCommit`;
- per gli overlay, l’identità è un hash del contenuto letto dalla sorgente;
- se esiste già una copia compatibile in un’altra Head o nel workspace, può essere usata come origine del clone CoW;
- se non esiste, il contenuto viene materializzato da Git o copiato dalla sorgente.

Una sorgente può essere modificata dopo la clonazione senza compromettere le Head già create: il filesystem separa automaticamente i blocchi.

Hydra non deve fidarsi soltanto di nome, dimensione o timestamp. Prima del riuso deve verificare che il contenuto corrisponda all’identità attesa.

Quando lo stato tracciato del workspace corrente coincide interamente con il
`baseCommit`, i suoi file regolari possono essere riusati direttamente come
sorgenti CoW. Se il confronto rileva una modifica tracciata, Hydra deve
materializzare dai blob del commit e non deve usare selettivamente contenuti
non verificati del workspace.

### Garanzia e ottimizzazione

La proprietà fondamentale è:

> Modificare un file in una Head non altera mai le altre Head.

Il risparmio fisico è invece una capacità negoziata:

- `cow`: blocchi condivisi fino alla prima modifica;
- `copy`: file completamente duplicati ma ugualmente isolati.

Se CoW non è disponibile, Hydra deve continuare a funzionare in modo corretto e dichiarare chiaramente il fallback. Non deve mai sostituirlo silenziosamente con hard link mutabili.

### Un branch privato per ogni Head

Due Head non devono lavorare contemporaneamente sulla stessa ref Git.

Hydra permette invece di:

- creare più Head dallo stesso branch;
- creare più Head dallo stesso commit;
- integrare successivamente più Head nello stesso branch di destinazione.

Esempio:

```text
Base: beta

payment  → hydra/payment
auth     → hydra/auth
refactor → hydra/refactor
```

Tutte le Head possono nascere dallo stesso commit di `beta`, ma ognuna avanza sul proprio branch.

### Metadati di una Head

| Campo | Significato |
|---|---|
| `name` | Identificatore Hydra leggibile |
| `worktreePath` | Directory completa della Head |
| `headRef` | Branch privato della Head |
| `baseRef` | Ref indicata come origine |
| `baseCommit` | Commit esatto usato alla creazione |
| `targetRef` | Branch previsto per l’integrazione |
| `materializationBackend` | Backend effettivamente usato (`cow` o `copy`) |
| `createdAt` | Data di creazione |

`baseRef` descrive l’intenzione; `baseCommit` rende deterministico lo stato effettivo di partenza.

---

## 6. Configurazione del progetto

Hydra separa configurazione condivisa, locator locale e stato fisico.

| Posizione | Funzione | Versionabile |
|---|---|---:|
| `<project-root>/.hydra.json` | Politica condivisa per creare le Head | Sì |
| `<git-common-dir>/hydra/project.json` | Locator canonico e identità dell'installazione locale | No |
| `<heads-directory>/.hydra/directory.json` | Marker di ownership della directory | No |
| `<heads-directory>/.hydra/heads.json` | Inventario delle Head fisiche locali | No |

La configurazione nel progetto descrive come quel progetto deve essere
materializzato. Il locator nel Git common directory permette al workspace
principale e a tutte le worktree di trovare la stessa directory fisica senza
reinterpretare un path relativo. Il marker e l'inventario risiedono invece
nella directory comune delle Head, accanto alle istanze che descrivono.

`projectId` identifica lo stesso progetto tra dispositivi;
`installationId` identifica una singola inizializzazione locale. Due
collaboratori condividono quindi il primo ma possiedono locator, directory Head
e `installationId` differenti.

### Configurazione iniziale

Il contratto condiviso target usa una politica portabile anziché un percorso
fisico dipendente dalla macchina:

```json
{
  "version": 2,
  "projectId": "heimdall-a84f2c",
  "headsDirectory": {
    "strategy": "sibling",
    "suffix": ".heads"
  },
  "branchPrefix": "hydra/",
  "storage": {
    "mode": "auto"
  },
  "overlay": {
    "copy": [
      "... .gitignore"
    ]
  }
}
```

`projectId` è un identificatore stabile del progetto e non dipende soltanto dal nome della directory. Permette a Hydra di distinguere repository omonimi nei metadati e nelle future operazioni globali.

`headsDirectory` descrive come individuare la directory locale, non memorizza il
percorso assoluto di una singola installazione. La strategia `sibling` risolve
su ogni dispositivo una directory sorella `<nome-progetto><suffix>`. Il
percorso canonico risultante e l'identità dell'installazione rimangono stato
locale non versionato.

Lo schema v2 tratta `headsDirectory` come un'unione discriminata dal campo
`strategy`. Le strategie previste sono:

| Strategy | Configurazione versionata | Risoluzione locale |
|---|---|---|
| `sibling` | `suffix` obbligatorio | Directory sorella `<nome-progetto><suffix>` |
| `relative` | `base: "repositoryParent"` e `path` obbligatori | `path` relativo alla directory che contiene il repository |
| `local` | Nessun percorso | Percorso assoluto scelto localmente e registrato soltanto nel locator non versionato |

Esempi:

```json
{
  "strategy": "sibling",
  "suffix": "-hydra-heads"
}
```

```json
{
  "strategy": "relative",
  "base": "repositoryParent",
  "path": "workspaces/heimdall-heads"
}
```

```json
{
  "strategy": "local"
}
```

`suffix` è un frammento di nome, non un percorso. Non impone convenzioni
stilistiche: può contenere testo ASCII o Unicode, spazi e punteggiatura e non
deve iniziare con un separatore convenzionale. Sono quindi validi, per esempio,
`.heads`, `heads`, `-hydra-heads` e ` workspace 🚀`.

Hydra rifiuta soltanto valori vuoti, caratteri di controllo e `/` o `\`, perché
questi ultimi trasformerebbero il suffisso in un percorso anziché in un
frammento del nome della directory. Eventuali ulteriori limiti specifici di un
filesystem vengono restituiti come errori operativi della piattaforma, non
normalizzati o sostituiti silenziosamente.

Il `path` della strategia `relative` usa `/` come separatore portabile, non può
essere vuoto o assoluto e non può contenere componenti `.` o `..`. La strategia
`local` richiede una scelta locale esplicita durante inizializzazione,
collegamento o riparazione; il percorso non viene mai scritto nella
configurazione versionata.

Ogni strategy accetta soltanto i propri campi. Strategy sconosciute, campi
mancanti o campi appartenenti a un'altra variante producono un errore di
configurazione; Hydra non applica fallback impliciti.

Qualunque destinazione risolta viene canonicalizzata e deve rispettare gli
stessi vincoli di ownership: non può trovarsi dentro il working tree del
progetto, dentro un'altra Head o nella directory appartenente a un altro
progetto.

Hydra implementa soltanto la versione 2 della configurazione strutturata. La
versione 1 sperimentale non è compatibile e viene rifiutata esplicitamente:
poiché Hydra non è ancora stata distribuita, mantenere un parser o una
migrazione per quel formato aggiungerebbe complessità senza proteggere utenti
reali.

La configurazione rimane JSON standard, non accetta commenti e rifiuta tutti i
campi sconosciuti. Hydra non pubblica ancora uno schema per editor e non
genera né accetta l'annotazione `$schema`. La pubblicazione dello schema
ufficiale tramite SchemaStore è pianificata per una fase successiva; soltanto
dopo che l'URL pubblico sarà disponibile Hydra potrà reintrodurre
l'annotazione e gli aiuti dell'editor.

Con `storage.mode: "auto"`, Hydra prova il clone CoW sul volume di destinazione e usa la copia completa se non è supportato. Modalità più rigide potranno essere esposte per test e automazioni, ma il default deve privilegiare compatibilità e sicurezza.

La direttiva:

```text
... .gitignore
```

espande le regole contenute in `.gitignore` esattamente in quella posizione dell’array.

Il `.gitignore` rimane quindi la sorgente viva predefinita degli overlay: Hydra non ne duplica il contenuto nella configurazione.

### Sintassi degli overlay

Le regole usano la stessa sintassi di `.gitignore`:

- `*`, `?` e `**`;
- slash iniziale per ancorare alla root;
- slash finale per indicare una directory;
- negazione con `!`;
- commenti con `#` nei file espansi;
- escaping di `!`, `#` e spazi;
- ordine delle regole;
- comportamento **last matching rule wins**.

L’unica estensione di Hydra è:

```text
... <path-del-file>
```

che include le regole del file indicato nel punto esatto in cui compare.

Esempio:

```json
{
  "version": 2,
  "projectId": "heimdall-a84f2c",
  "headsDirectory": {
    "strategy": "sibling",
    "suffix": ".heads"
  },
  "branchPrefix": "hydra/",
  "overlay": {
    "copy": [
      "... .gitignore",
      "!node_modules/",
      "!vendor/",
      "!storage/logs/",
      ".env.hydra"
    ]
  }
}
```

La semantica è quella di una lista di selezione per la copia:

| Regola | In `.gitignore` | Nell’overlay Hydra |
|---|---|---|
| `.env` | Ignora il file | Copia il file |
| `storage/` | Ignora la directory | Copia la directory |
| `!storage/logs/` | Non ignorare il percorso | Non copiarlo |

In questo esempio:

- le voci di `.gitignore` formano la selezione predefinita;
- `node_modules`, `vendor` e i log vengono esclusi dalla copia;
- `.env.hydra` viene aggiunto esplicitamente.

### Origine degli overlay

Il codice versionato deriva dal commit indicato da `--from`.

I file locali selezionati dagli overlay derivano invece dalla Head o dal workspace da cui viene eseguito il comando:

```bash
hydra head create payment --from beta
```

significa:

- codice versionato dal commit risolto da `beta`;
- overlay dal workspace corrente;
- nuovo branch privato `hydra/payment`.

### Algoritmo di risoluzione

Durante la creazione di una Head, Hydra:

1. legge `overlay.copy`;
2. espande le direttive `... <file>`;
3. valuta tutte le regole in ordine;
4. considera soltanto i percorsi esistenti nella sorgente;
5. calcola quantità e dimensione logica, conserva il target dei link simbolici
   e ordina deterministicamente i percorsi selezionati;
6. calcola le identità dei file regolari tramite batch Git limitati e worker
   concorrenti limitati, preservando l'associazione ordinata tra percorso e
   hash;
7. verifica per ogni file regolare la capacità CoW verso il volume effettivo
   delle Head, senza generalizzare il successo di un file agli altri;
8. mostra un riepilogo e gli eventuali avvisi;
9. affida i file selezionati al Materializer;
10. usa CoW o copia completa secondo il piano verificato.

Hydra deve rispettare la semantica dei pattern Git e non interpretarli come semplici glob del filesystem.

### Protezioni obbligatorie

Indipendentemente dalla configurazione, Hydra non deve:

- copiare `.git`;
- copiare la directory che contiene le Head;
- collocare la directory delle Head dentro il working tree principale o dentro un’altra Head;
- seguire symlink che escono dalla root del progetto;
- ricreare symlink assoluti, rotti o il cui target finale non rimane nella
  root della Head;
- copiare socket, pipe o altri file speciali;
- sovrascrivere con un overlay un file già tracciato da Git;
- creare una Head dentro un’altra Head;
- entrare in ricorsione durante l’espansione dei file di regole.

Poiché `.gitignore` può includere directory molto grandi come `node_modules` o `vendor`, Hydra deve mostrare quantità e dimensione della copia e richiedere conferma oltre soglie di sicurezza. Non deve tuttavia modificare implicitamente le regole scelte dall’utente.

I link simbolici relativi selezionati dagli overlay devono essere preservati
come link, non dereferenziati. Hydra può ricrearli soltanto quando il target
risolve all’interno del progetto sorgente e, dopo la materializzazione, dentro
la nuova Head. Questo permette di conservare strutture locali come
`node_modules/.bin` e `vendor/bin` senza collegare la Head al workspace
sorgente o a percorsi esterni.

Se la pianificazione seleziona uno o più symlink assoluti, rotti o in uscita
dal progetto, Hydra deve raccogliere tutti i relativi percorsi portabili prima
di proporre una correzione. La CLI può chiedere esplicitamente se aggiungere in
fondo a `overlay.copy` regole di negazione letterali e ancorate per quei soli
percorsi. Una risposta negativa, EOF o input non riconosciuto non modifica
configurazione, branch, worktree o stato. Una risposta positiva autorizza una
scrittura atomica di `.hydra.json` prima di qualunque mutazione Git; la
configurazione versionata rimane una modifica visibile che l'utente deve
revisionare e committare. Hydra non deve estendere questa autorizzazione a file
speciali, collisioni con file tracciati o altri errori di sicurezza.

---

## 7. MVP v0.1

La prima versione deve validare il ciclo di vita delle Head. È una CLI, senza dashboard web e senza gestione dei runtime.

### 7.1 Inizializzazione

```bash
hydra init .
```

Hydra:

1. verifica che il percorso appartenga a un repository Git;
2. risolve repository root e Git common directory;
3. genera un `projectId` stabile;
4. determina come default la directory sorella `<nome-progetto>.heads`;
5. verifica che la destinazione sia esterna al working tree e non appartenga a un altro progetto;
6. genera `.hydra.json` con la politica portabile della directory delle Head e
   registra il percorso concreto soltanto nel locator locale;
7. inizializza lo stato locale;
8. verifica le capacità di materializzazione del volume;
9. non modifica i file applicativi del progetto.

Al termine, `hydra init` dichiara il backend verificato sul volume delle Head:
`copy-on-write` quando la prova nativa riesce, oppure `full copy` quando è stato
verificato il fallback sicuro.

### 7.2 Creazione di una Head

```bash
hydra head create payment
hydra head create payment --from beta
hydra head create payment --from beta --target beta
```

Se `--from` non è specificato, Hydra usa l’`HEAD` corrente come origine.

Hydra:

1. valida il nome;
2. risolve la ref di origine nel relativo commit;
3. genera un branch dedicato;
4. crea la struttura amministrativa del worktree;
5. inizializza l’index sul commit di base;
6. materializza i file tracciati con il backend selezionato;
7. risolve e materializza gli overlay con lo stesso backend;
8. verifica che il worktree risultante corrisponda allo stato atteso;
9. registra i metadati;
10. restituisce percorso e backend effettivamente usato.

L’implementazione preferita evita che il checkout standard di Git scriva inutilmente tutti i file prima della materializzazione. Può creare il worktree senza checkout, inizializzare separatamente l’index e delegare la scrittura dei file al Materializer. Se una piattaforma richiede un flusso differente, il risultato Git osservabile deve rimanere equivalente.

Per l’MVP, la sorgente degli overlay deve avere uno stato sufficientemente
stabile finché il relativo payload non è stato isolato. Hydra confronta ogni
file materializzato con l'identità pianificata e fallisce in modo sicuro se la
copia ha osservato contenuto differente, anziché produrre una Head parziale non
dichiarata. Una modifica della sorgente successiva a un clone CoW già verificato
non invalida la Head: i blocchi della destinazione sono ormai indipendenti.

### 7.3 Elenco e stato

```bash
hydra status
hydra head list
hydra head status payment
hydra head path payment
```

Lo stato mostra almeno:

- nome;
- percorso;
- branch;
- commit corrente;
- base ref e base commit;
- target ref;
- file modificati, aggiunti, eliminati e untracked;
- commit ahead/behind rispetto alla base;
- presenza o assenza del worktree;
- eventuali incoerenze tra Git e i metadati Hydra.

`hydra head list` restituisce i nomi delle Head locali in ordine stabile.
`hydra head path` restituisce soltanto il percorso assoluto registrato, così da
poter essere composto con shell, IDE e automazioni.

`hydra status` mostra il progetto, la directory fisica delle Head e una sintesi
`clean`, `modified` o `inconsistent` per ogni Head. `hydra head status` espone
il dettaglio. Branch, commit e modifiche descrivono la `HEAD` realmente aperta
nella worktree. Se differisce dalla `headRef` registrata, Hydra mostra entrambe
e segnala l'incoerenza.

I conteggi ahead/behind confrontano il commit osservato nella worktree con il
commit corrente della `baseRef` simbolica. Per un'origine non simbolica, come
uno SHA abbreviato, usano sempre il `baseCommit` completo registrato: una
successiva ref con lo stesso nome non può cambiare retroattivamente la base.
Il `baseCommit` resta visibile come commit esatto dal quale la Head è stata
creata.

Questi comandi sono strettamente read-only: non acquisiscono il lock destinato
alle mutazioni, non riscrivono l'inventario e non tentano repair impliciti.
Un'incoerenza ispezionabile, come una directory mancante o una target ref
scomparsa, viene mostrata senza correggerla. Metadati che indirizzano fuori
dalla directory delle Head posseduta vengono invece rifiutati come non sicuri.
Gli output destinati alle persone neutralizzano i caratteri di controllo nei
percorsi e nei valori persistiti. `hydra head path`, quando stdout non è un
terminale, conserva invece il percorso esatto per la composizione in pipeline.

### 7.4 Apertura

```bash
hydra head open payment
```

Hydra può eseguire un adapter a comando configurabile. La configurazione deve
separare programma e argomenti, così i placeholder non vengono interpolati in
una stringa di shell:

```json
{
  "commands": {
    "open": {
      "program": "code",
      "args": ["{path}"]
    }
  }
}
```

Si tratta di un adapter generico a comando, non di un’integrazione specifica con VS Code.

I placeholder iniziali disponibili per gli adapter sono:

- `{name}`;
- `{path}`;
- `{headRef}`;
- `{baseRef}`;
- `{targetRef}`.

Hydra passa ogni argomento separatamente al processo configurato e non
costruisce un comando shell non escapato.

### 7.5 Chiusura

```bash
hydra head close payment
```

La chiusura rappresenta il workflow esplicito che conclude il lavoro di una
Head. In assenza di configurazione custom, Hydra:

1. verifica che la Head e il relativo target siano coerenti;
2. integra il branch privato della Head nel suo `targetRef`;
3. usa il normale comportamento Git: fast-forward quando possibile, altrimenti
   merge commit;
4. non esegue rebase, squash o risoluzione automatica dei conflitti;
5. solo dopo un merge riuscito esegue la rimozione protetta della Head.

Per una Head creata da un branch locale senza `--target`, `targetRef` coincide
con il branch di partenza. Un `--target` esplicito rimane invece autorevole per
la chiusura.

Il merge predefinito sceglie dinamicamente dove integrare:

- se `targetRef` è attivo in una worktree registrata, Hydra può avanzare quella
  worktree soltanto quando branch e commit corrispondono allo snapshot
  validato, non è in corso un'operazione Git e working tree e index sono
  puliti;
- se `targetRef` non è attivo in alcuna worktree, Hydra integra senza checkout
  e pubblica la ref con compare-and-swap.

Una worktree target sporca o impegnata in un'operazione Git blocca la chiusura
senza mutare target o Head. Il meccanismo garantisce:

- target ref invariata se il merge fallisce o produce conflitti;
- aggiornamento coerente di ref, index e file quando il target pulito è
  checkoutato;
- nessuna modifica a working tree diverse dal target selezionato;
- nessuna eliminazione della Head in caso di conflitto;
- diagnostica sufficiente per risolvere o ripetere esplicitamente la chiusura.

La chiusura può essere sostituita da un adapter configurabile:

```json
{
  "commands": {
    "close": {
      "strategy": "command",
      "program": "./tools/close-head",
      "args": ["{path}", "{headRef}", "{targetRef}"],
      "removeOnSuccess": true
    }
  }
}
```

Se `commands.close` è assente, i default equivalgono concettualmente a:

```json
{
  "strategy": "merge",
  "removeOnSuccess": true
}
```

`removeOnSuccess` non viene espanso dentro il comando custom: è un passo
successivo posseduto da Hydra. Il passo viene eseguito soltanto se l’adapter
termina con successo e usa le stesse protezioni di `hydra head remove`, senza
forzature implicite. Se il comando riesce ma la rimozione protetta fallisce,
Hydra segnala separatamente che l’azione di chiusura è completata ma la Head è
rimasta presente.

Con `removeOnSuccess: false`, Hydra esegue l’azione di chiusura ma conserva
worktree, branch e metadati. Un comando custom che non integra i commit nel
`targetRef` non può aggirare la protezione contro la perdita di lavoro:
l’eventuale rimozione finale deve fallire in modo sicuro.

Hydra non può assumere di poter annullare in sicurezza effetti Git o filesystem
prodotti da un adapter arbitrario. Se il comando custom fallisce dopo avere
modificato la target ref, Hydra conserva la Head, non esegue la rimozione e
segnala la differenza rispetto allo snapshot iniziale.

L'integrazione nativa implementata usa `merge-tree` e `commit-tree` per
preparare in modo isolato le storie divergenti. Se il target è checkoutato in
una worktree registrata e pulita, Hydra la avanza al commit validato mantenendo
coerenti ref, index e file. Se il target non è checkoutato, pubblica invece il
fast-forward o il merge commit con un aggiornamento compare-and-swap della
ref. Un conflitto conserva entrambe le ref e la Head; una worktree target
sporca o impegnata in un'operazione Git blocca la chiusura senza mutazioni.

### 7.6 Rimozione sicura

```bash
hydra head remove payment
hydra head remove payment --force
```

Senza `--force`, Hydra rifiuta la rimozione se:

- esistono modifiche non committate;
- esistono file untracked;
- il branch contiene commit non integrati nel target;
- il percorso o lo stato Git non corrispondono ai metadati;
- Git richiederebbe una rimozione forzata.

Worktree e branch sono entità separate. La rimozione ordinaria elimina il worktree secondo una politica esplicita e non deve cancellare automaticamente un branch che contiene lavoro recuperabile.

L'implementazione corrente interpreta `--force` come autorizzazione a
scartare modifiche tracked, staged e untracked del worktree, non come
autorizzazione a ignorare ownership o incoerenze dei metadati. Se il branch
privato contiene commit non integrati, una rimozione forzata elimina worktree e
inventario ma conserva la ref e ne mostra il nome. Un branch viene eliminato
soltanto quando il relativo commit è ancora raggiungibile dal target e la ref
punta ancora al commit validato.

### 7.7 Completamento della shell

Hydra deve offrire completamento tramite Tab per la gerarchia dei comandi,
le opzioni e soprattutto per le entità locali già esistenti.

```bash
hydra completions <shell>
```

Il completamento dinamico dei nomi di Head si applica almeno a:

```bash
hydra head status <name>
hydra head path <name>
hydra head open <name>
hydra head close <name>
hydra head remove <name>
```

La regola generale è: una posizione che richiede un’entità esistente propone
le entità di quel tipo note al progetto corrente; una posizione che crea una
nuova entità, come `head create <name>`, non propone nomi già occupati.

La risoluzione dei candidati deve:

- essere read-only e non eseguire repair, open, close o remove;
- non mostrare prompt;
- essere sufficientemente veloce per l’uso interattivo;
- restituire nomi ordinati e senza duplicati;
- fallire silenziosamente con zero candidati fuori da un progetto Hydra o con
  stato non leggibile;
- delegare alla shell l’escaping finale dei candidati;
- poter essere riutilizzata dai diversi script di completamento senza
  duplicare la logica di lettura dello stato.

Il primo supporto deve coprire almeno le shell principali dell’ambiente di
sviluppo del progetto. L’elenco esatto delle shell supportate e il contratto del
comando interno per i candidati vanno fissati prima dell’implementazione.

In una fase successiva, i pacchetti e gli installer ufficiali potranno
registrare automaticamente gli script nei percorsi standard delle shell
supportate. Questa integrazione non appartiene al comando di completamento:
deve rispettare le convenzioni del package manager, non modificare
silenziosamente i file di configurazione personali e mantenere disponibile la
registrazione manuale.

### 7.8 Repair e riconciliazione

```bash
hydra repair
```

Hydra confronta i propri metadati con:

```bash
git worktree list --porcelain
```

e deve poter:

- rilevare Head mancanti;
- individuare worktree Hydra non registrati;
- aggiornare percorsi modificati;
- segnalare branch o directory incoerenti;
- ricostruire lo stato minimo senza modificare il codice.

Le correzioni distruttive o ambigue richiedono sempre una conferma esplicita.

### 7.9 Diagnostica dello storage

```bash
hydra doctor storage
```

Il comando verifica il volume sul quale saranno create le Head e mostra almeno:

```text
Storage backend: copy-on-write
Native primitive: APFS clone
Fallback: full copy
Mutable hard links: disabled
Isolation: supported
```

La diagnostica deve eseguire una prova reale e sicura in una directory temporanea sul volume di destinazione: la sola rilevazione del sistema operativo non è sufficiente.

---

## 8. CLI dell’MVP

```bash
hydra init [path]

hydra status

hydra head create <name> [--from <ref>] [--target <ref>]
hydra head list
hydra head status <name>
hydra head path <name>
hydra head open <name>
hydra head close <name>
hydra head remove <name> [--force]

hydra completions <shell>
hydra repair
hydra doctor storage
```

Eventuali alias più brevi potranno essere introdotti senza cambiare il modello:

```bash
hydra create <name>
hydra list
hydra open <name>
hydra destroy <name>
```

La gerarchia `hydra head ...` resta però più chiara e lascia spazio a future entità.

### Convenzioni UX della CLI

Hydra si rivolge a utenti Git e adotta, dove il modello delle Head lo consente,
la stessa grammatica concettuale e lo stesso tono operativo di Git:

- comandi e opzioni usano termini Git esistenti come `HEAD`, ref, commit e
  branch locale;
- l’help di ogni comando documenta scopo, sintassi, argomenti, default
  significativi ed esempi copiabili;
- l’help principale e quello dei gruppi mostrano la sintassi completa dei
  comandi annidati già implementati, senza costringere l’utente a scoprirli un
  livello alla volta;
- i messaggi restano concisi, dichiarativi e orientati all’esito;
- una creazione riuscita mostra il percorso concreto della nuova Head e, su un
  terminale interattivo compatibile, lo rende apribile come collegamento locale;
- durante una creazione lunga, gli eventi di avanzamento per fase vengono
  mostrati su `stderr` soltanto se `stderr` è un terminale interattivo; pipe,
  file e automazioni non ricevono questi messaggi;
- un riepilogo informativo non richiede conferma;
- quando la pianificazione trova symlink overlay non sicuri, la CLI elenca
  tutti i percorsi relativi e può chiedere se escluderli in modo persistente
  aggiornando `.hydra.json`;
- la conferma è riservata a un fallback o a un’azione con un costo o rischio
  materiale che Hydra ha rilevato concretamente.

Per gli overlay, Hydra mostra sempre numero di file e peso logico. Se la prova
copy-on-write dalla sorgente reale al volume delle Head riesce, procede senza
prompt. Se uno o più file richiedono la duplicazione completa dei byte, mostra
numero e peso del sottoinsieme interessato e chiede conferma prima di creare
branch, worktree o stato.

---

## 9. Stato locale e fonte della verità

Il Git common directory è la directory amministrativa condivisa da tutte le
worktree dello stesso repository. In un repository normale coincide
tipicamente con `.git`; in una linked worktree il suo `.git` operativo è
separato, ma `git rev-parse --git-common-dir` continua a restituire la stessa
directory comune.

Hydra vi conserva soltanto il locator necessario al bootstrap:

```json
{
  "version": 1,
  "projectId": "ecommerce-a84f2c",
  "installationId": "local-24b64f",
  "projectRoot": "/projects/ecommerce",
  "headsDirectory": "/projects/ecommerce.heads"
}
```

La directory risolta contiene un marker di ownership
`<heads-directory>/.hydra/directory.json`:

```json
{
  "version": 1,
  "projectId": "ecommerce-a84f2c",
  "installationId": "local-24b64f"
}
```

Locator e marker devono concordare prima di ogni mutazione. Un `projectId`
uguale con `installationId` differente rappresenta un'altra installazione
locale dello stesso progetto e non autorizza il riuso implicito della
directory.

L'inventario fisico vive in `<heads-directory>/.hydra/heads.json`:

```json
{
  "version": 1,
  "heads": {
    "payment": {
      "worktreePath": "/projects/ecommerce.heads/payment",
      "headRef": "refs/heads/hydra/payment",
      "baseRef": "refs/heads/beta",
      "baseCommit": "abc123",
      "targetRef": "refs/heads/beta",
      "materializationBackend": "cow",
      "createdAt": "2026-07-26T20:00:00Z"
    }
  }
}
```

Questa separazione evita due dipendenze circolari:

- qualsiasi Head trova il locator attraverso il Git common directory;
- se l'inventario viene perso, Git e le directory fisiche rimangono
  ispezionabili;
- se il locator viene perso, il marker permette a un futuro `repair` di
  riconnettere una directory indicata esplicitamente dall'utente;
- spostare repository o directory Head richiede una relocation esplicita e
  verificata, non la risoluzione silenziosa di un nuovo percorso.

I file locali:

- vengono scritti atomicamente;
- contengono una versione dello schema;
- non sono l’unica fonte della verità;
- non memorizzano informazioni ricavabili in modo affidabile da Git se non utili alla riconciliazione;
- non contengono PID, porte, agenti o runtime nell’MVP.

Git rimane autorevole per:

- worktree esistenti;
- branch e ref;
- commit;
- stato del working tree.

Hydra rimane autorevole per:

- nome logico della Head;
- base ref originaria;
- base commit registrato;
- target previsto;
- backend di materializzazione usato e diagnostica associata;
- configurazioni e intenzioni che Git non conosce.

Lo stato non deve contenere una patch proprietaria necessaria per aprire o ricostruire i file. Ogni Head resta autosufficiente come normale working tree.

---

## 10. Operazioni Git

Hydra deve rimanere pienamente compatibile con l’uso diretto di Git.

Dentro una Head, l’utente può normalmente:

```bash
git status
git diff
git add .
git commit
git fetch
git rebase
git merge
git push
```

Un commit creato in una Head è immediatamente disponibile nel repository condiviso.

Nell’MVP Hydra non implementa:

- merge automatici o in background al di fuori dell’azione esplicita
  `hydra head close`;
- rebase automatico;
- risoluzione dei conflitti;
- push o pull impliciti;
- sincronizzazione automatica con la base;
- cancellazione di branch al di fuori delle azioni esplicite e protette
  `head remove` e `head close`.

La chiusura è quindi un’orchestrazione richiesta esplicitamente dall’utente,
non una sincronizzazione automatica. Le altre operazioni hanno conseguenze
abbastanza importanti da dover restare inizialmente sotto il controllo
esplicito dell’utente e di Git.

---

## 11. Casi limite minimi

L’MVP deve gestire in modo prevedibile:

- repository standard e repository già aperti da un worktree;
- repository omonimi collocati in directory differenti;
- branch sorgente locale;
- ref o commit esplicito;
- nomi di Head duplicati;
- branch Hydra già esistente;
- destinazione già esistente;
- directory sorella predefinita già appartenente a un altro progetto;
- `headsDirectory` configurata dentro il repository o dentro un’altra Head;
- worktree bloccato o rimosso manualmente;
- repository in stato detached;
- submodule;
- file ignorati grandi;
- filesystem senza supporto CoW;
- Head collocate su un volume differente dalla sorgente;
- fallimento di un clone CoW a metà materializzazione;
- sorgente candidata modificata durante la verifica;
- symlink;
- interruzione durante creazione o copia;
- perdita del file di stato;
- rimozione manuale di una directory;
- modifica diretta delle ref tramite Git.
- `targetRef` avanzata dopo la creazione della Head;
- target già aperto in un altro worktree;
- conflitto durante la chiusura;
- adapter di chiusura terminato con exit code non-zero;
- merge riuscito seguito da rimozione protetta fallita.

Per i submodule, l’MVP può dichiarare un supporto limitato e lasciare esplicito il comando necessario per inizializzarli. Non deve fingere che siano già isolati o pronti.

Le operazioni che attraversano più passaggi devono essere progettate come transazioni recuperabili:

1. validazione;
2. creazione branch;
3. creazione worktree;
4. copia overlay;
5. registrazione;
6. rollback sicuro o stato riconciliabile in caso di errore.

La chiusura richiede una transazione distinta:

1. validazione della Head e snapshot della target ref;
2. integrazione isolata o adapter custom;
3. verifica dell’esito e della target ref risultante;
4. rimozione protetta soltanto quando configurata;
5. se l’integrazione nativa fallisce, conservazione della Head e target ref
   invariata;
6. se un adapter custom fallisce, conservazione della Head e segnalazione di
   ogni modifica osservata sulla target ref.

---

## 12. Requisiti non funzionali

### Portabilità

Target iniziali:

- macOS;
- Linux.

Il design deve evitare assunzioni che impediscano un successivo supporto a Windows.

### Prestazioni

- Nessuna duplicazione dell’object database Git.
- Clone CoW dei file tracciati e degli overlay quando il volume lo supporta.
- Copia completa per singolo file come fallback sicuro.
- Stato calcolato tramite comandi Git mirati.
- Materializzazione preceduta da scansione e stima.
- Distinzione tra dimensione logica della Head e spazio fisico esclusivo quando il backend consente di misurarlo in modo attendibile.
- Nessun daemon obbligatorio nell’MVP.

### Sicurezza

- Nessuna rimozione forzata implicita.
- Nessun hard link per file modificabili.
- Nessuna interpolazione insicura dei comandi.
- Validazione e normalizzazione di tutti i percorsi.
- Nessuna uscita dalla root tramite traversal o symlink.
- Scritture di stato atomiche.
- Messaggi di errore che indicano cosa è stato creato e come recuperarlo.

### Osservabilità

Ogni comando deve poter produrre:

- output leggibile;
- exit code coerenti;
- opzionalmente output JSON per script e integrazioni future.

---

## 13. Stack consigliato

```text
hydra/
├── Cargo.toml
├── rust-toolchain.toml
└── crates/
    ├── hydra-cli/
    ├── hydra-core/
    ├── hydra-git/
    ├── hydra-materializer/
    ├── hydra-overlays/
    └── hydra-config/
```

Tecnologie:

- Rust con toolchain ed edition dichiarate nel repository;
- Cargo workspace;
- `clap` o equivalente per il parsing dichiarativo della CLI;
- `std::process::Command` o un wrapper Rust giustificato per Git e processi;
- `serde` e `serde_json` o equivalenti per configurazione e stato;
- errori tipizzati e contestualizzati, senza panic per condizioni operative recuperabili;
- adapter nativi minimi per APFS clone e reflink Linux, con fallback portabile;
- una libreria compatibile con la semantica `gitignore`, verificata con test di conformità;
- test Rust unitari, di integrazione e CLI eseguiti da Cargo;
- repository Git e filesystem temporanei reali per verificare i contratti di integrazione.

Lo sviluppo segue obbligatoriamente il ciclo Red-Green-Refactor. Ogni comportamento viene introdotto partendo da un test fallito e ogni modifica protegge sia il nuovo contratto sia il comportamento esistente più esposto a regressioni.

Per l’MVP, file JSON e scritture atomiche sono sufficienti. SQLite e dipendenze native aggiuntive non sono necessari, salvo gli adapter minimi richiesti dalle primitive CoW di piattaforma.

---

## 14. Fuori dall’MVP

| Funzionalità | Fase prevista |
|---|---|
| Ciclo di vita Head/worktree | MVP |
| Branch privato per Head | MVP |
| Materializer CoW con fallback sicuro | MVP |
| Diagnostica del backend storage | MVP |
| Overlay basato su `.gitignore` | MVP |
| Status e rimozione protetta | MVP |
| Apertura tramite comando configurabile | MVP |
| Chiusura con merge o comando configurabile | MVP |
| Completamento shell statico e dinamico delle Head | MVP |
| Repair e riconciliazione | MVP |
| Guida utente italiana mantenuta | MVP |
| Skill operativa installabile per agenti AI | MVP |
| Schema della configurazione pubblicato tramite SchemaStore | Successivo |
| Head Recipe condivisibili e materializzabili | Successivo |
| Hook o comando di setup | v0.2 |
| Adapter per agenti | v0.2 |
| Processi runtime e porte | v0.2 |
| Dashboard web locale | v0.3 |
| Diff visuale navigabile | v0.3 |
| Merge/rebase assistiti e risoluzione interattiva | v0.3 |
| Terminale incorporato | Successivo |
| Docker e servizi isolati | Successivo |
| Isolamento di database e cache | Successivo |
| Collaborazione cloud | Non prioritaria |
| Applicazione desktop Tauri | Solo se giustificata dall’uso |
| Filesystem virtuale Hydra | Solo se diventa necessario garantire CoW su volumi non compatibili |

Dashboard, runtime e agenti rimangono parte della visione, ma vengono costruiti sopra un motore delle Head già affidabile.

### Skill operativa per agenti AI

L’MVP deve includere almeno una skill installabile che permetta a un agente AI
di usare Hydra in modo autonomo, efficace e sicuro. La skill è un asset di
istruzioni operative, distinto da un adapter o da un’integrazione runtime:
insegna all’agente a usare la CLI esistente senza introdurre un protocollo
proprietario, un daemon o accesso diretto ai metadati interni.

La skill deve guidare almeno questi comportamenti:

- verificare repository, configurazione e comandi realmente disponibili;
- scegliere consapevolmente nome, origine e target di una Head;
- creare la Head e spostare il proprio contesto operativo nella directory
  restituita;
- lavorare e committare soltanto sul branch privato della Head;
- usare i comandi disponibili `status`, `path`, `open`, `close` e `remove`
  verificandone prima la sintassi nell'help del binario installato;
- non modificare manualmente locator, marker, inventario o lock;
- non sostituire i comandi protetti di Hydra con cancellazioni filesystem o
  rimozioni worktree distruttive;
- fermarsi e riportare lo stato quando ownership, policy, working tree o
  integrazione non consentono un’azione sicura.

La guida utente italiana rimane la sorgente operativa leggibile dalle persone.
La skill deve derivarne comandi, vincoli e procedure senza creare una seconda
specifica divergente. Ogni modifica a un workflow rilevante aggiorna nello
stesso intervento codice, help, test, guida e skill.

La prima skill è distribuita in `skills/hydra/` nel formato installabile da
Codex. Le istruzioni operative restano indipendenti dal vendor, così da poter
produrre varianti per altri agenti senza ridefinire il comportamento di Hydra.
La skill non integra o rimuove automaticamente una Head: lascia per default il
workspace disponibile alla revisione e richiede un'autorizzazione esplicita
prima delle operazioni che aggiornano il target o possono scartare file.

### Head Recipe condivisibili

Una Head fisica rimane un'istanza locale e non viene versionata. In futuro
Hydra può introdurre una **Head Recipe** portabile che descrive l'intenzione
riproducibile necessaria a materializzare una nuova istanza su un altro
dispositivo:

```json
{
  "version": 1,
  "name": "payment",
  "source": "feature/payment",
  "target": "main",
  "overlayProfile": "default",
  "lifecycle": {
    "removeRecipeOnClose": true
  }
}
```

Una recipe può essere creata direttamente oppure promuovendo una Head locale
tramite un comando dedicato. La promozione deve verificare che il contenuto da
condividere sia raggiungibile tramite Git e non può incorporare modifiche non
committate, percorsi locali, backend, lock, timestamp operativi o segreti degli
overlay.

Un collaboratore materializza la recipe come una nuova Head locale: path,
worktree, branch privato e backend restano specifici del suo dispositivo. Git
continua a trasportare commit e ref Git; la recipe trasporta soltanto intenzione
e parametri riproducibili.

Una recipe può dichiararsi effimera e richiedere la rimozione dopo la chiusura
riuscita della relativa Head. Poiché una recipe versionata è un file Git, tale
rimozione deve far parte esplicitamente della transazione di chiusura: non deve
sporcare silenziosamente un'altra worktree, essere eseguita dopo una chiusura
fallita o generare un commit implicito non autorizzato.

---

## 15. Definition of done

Hydra v0.1 è conclusa quando:

1. può inizializzare un normale repository Git;
2. crea per default una directory sorella `<nome-progetto>.heads`;
3. rifiuta destinazioni interne al working tree o appartenenti a un altro progetto;
4. genera una configurazione che espande dinamicamente `.gitignore`;
5. crea almeno tre Head dallo stesso commit;
6. assegna a ogni Head un branch privato;
7. presenta ogni Head come un progetto completo a IDE e agenti;
8. materializza i file tracciati tramite CoW quando supportato;
9. materializza gli overlay applicando correttamente pattern, negazioni e precedenze;
10. applica lo stesso modello CoW/copia ai file tracciati e non tracciati;
11. usa una copia completa sicura quando CoW non è disponibile;
12. non usa hard link per alcun file modificabile;
13. dichiara il backend effettivo tramite `hydra doctor storage`;
14. impedisce la copia di percorsi vietati o pericolosi;
15. modificare un file tracciato o un overlay in una Head non altera le altre;
16. ogni Head produce un diff indipendente;
17. ogni Head può creare commit sul proprio branch;
18. i commit sono visibili dal repository originale;
19. mostra uno stato attendibile delle Head;
20. impedisce rimozioni rischiose senza `--force`;
21. rimuove una Head senza danneggiare le altre;
22. ricostruisce lo stato dopo la perdita dei metadati Hydra;
23. rimane compatibile con l’uso diretto dei normali comandi Git;
24. completa i flussi principali su macOS e Linux;
25. completa tramite Tab i nomi delle Head nei comandi che richiedono una Head
    esistente;
26. chiude una Head integrandola nel `targetRef` e la rimuove soltanto dopo
    un’integrazione riuscita;
27. permette un adapter di chiusura custom con rimozione finale configurabile e
    protetta;
28. supera test di integrazione eseguiti su repository temporanei reali,
    includendo sia il backend CoW sia il fallback di copia;
29. mantiene una guida utente italiana che distingue comportamento disponibile
    e pianificato e documenta flusso base, configurazione, customizzazioni,
    sicurezza e troubleshooting;
30. distribuisce almeno una skill installabile per agenti AI, allineata alla
    guida utente e verificata su un repository temporaneo, che usa Hydra senza
    aggirarne le protezioni.

## 16. Ipotesi da validare

L’MVP deve rispondere a una domanda precisa:

> Materializzare ogni attività come una Head Git isolata, completa ma fisicamente efficiente, riduce davvero il caos dello sviluppo parallelo e rende più controllabile il lavoro prodotto dagli agenti AI?

Se la risposta è positiva, Hydra potrà estendere la Head fino a rappresentare un’intera sessione operativa:

```text
Head
├── branch
├── worktree
├── materializer
├── overlay
├── IDE
├── agent
├── runtime
├── diff
└── commits
```

Il nucleo, però, deve restare invariato:

> **una directory reale, una diff indipendente, contenuti isolati, Git come fondamento.**
