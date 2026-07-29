# Guida all’uso di Hydra

Questa guida descrive come usare Hydra e come personalizzarne il comportamento.
È mantenuta insieme al codice: comandi e opzioni presentati come disponibili
devono esistere nel binario corrente.

Hydra è ancora in sviluppo e non è stato distribuito pubblicamente. Le sezioni
marcate **Pianificato — non ancora disponibile** descrivono il flusso previsto,
ma non costituiscono istruzioni eseguibili.

---

## 1. Concetti essenziali

Hydra crea più directory di lavoro isolate, chiamate **Head**, a partire dallo
stesso repository Git.

Ogni Head:

- è una vera worktree Git;
- possiede un branch privato, per esempio `hydra/payment`;
- ha working tree e index indipendenti;
- contiene i file tracciati del commit di partenza;
- può ricevere file ignorati selezionati tramite gli overlay;
- può essere aperta direttamente in un IDE, terminale o agente AI.

Git rimane la fonte autorevole per commit, branch e worktree. Hydra aggiunge
materializzazione efficiente, configurazione condivisa e metadati locali per
coordinare il ciclo di vita delle Head.

---

## 2. Funzionalità disponibili oggi

Il binario corrente espone:

```text
hydra init [PATH]
hydra head create <NAME> [--from <REF>] [--target <BRANCH>]
```

Puoi verificare la sintassi installata con:

```bash
hydra --help
hydra init --help
hydra head --help
hydra head create --help
```

I comandi `status`, `path`, `open`, `close`, `remove`, `repair`, `doctor` e il
completamento della shell appartengono al contratto MVP, ma non sono ancora
implementati.

---

## 3. Installazione durante lo sviluppo

Finché Hydra non dispone di un pacchetto distribuito, puoi installare il
binario dalla root del repository sorgente:

```bash
cargo install --path crates/hydra-cli --force
```

Verifica quale binario viene eseguito:

```bash
hydra --version
command -v hydra
```

La guida non definisce ancora un processo di aggiornamento o distribuzione
stabile.

---

## 4. Flusso base

### 4.1 Prepara un repository

Hydra deve essere inizializzato dentro un repository Git:

```bash
cd /percorso/del/progetto
git status
```

È consigliabile partire da un repository con almeno un commit, perché la
creazione di una Head deve risolvere un commit di base.

### 4.2 Inizializza Hydra

Dalla root del progetto:

```bash
hydra init
```

Oppure indicando il repository:

```bash
hydra init /percorso/del/progetto
```

Con la configurazione predefinita, dato:

```text
/workspace/Shop/
```

Hydra crea:

```text
/workspace/Shop/.hydra.json
/workspace/Shop.heads/
```

Il comando mostra anche il backend verificato sul volume:

```text
Initialized Hydra in /workspace/Shop
Storage backend: copy-on-write
```

oppure:

```text
Storage backend: full copy
```

`.hydra.json` contiene la politica condivisa del progetto e deve essere
versionato dopo averlo revisionato:

```bash
git add .hydra.json
git commit -m "chore: configure Hydra"
```

I locator, marker di ownership e inventari fisici sono locali alla macchina e
non devono essere aggiunti al repository.

### 4.3 Crea una Head

Per creare una Head dal `HEAD` corrente:

```bash
hydra head create payment
```

Un risultato tipico è:

```text
New Head successfully created at /workspace/Shop.heads/payment
Storage backend: copy-on-write
```

Nei terminali compatibili il percorso è un collegamento `file://` cliccabile.
Se sono presenti overlay, Hydra mostra prima il loro numero e peso logico.

Puoi entrare nella Head come in qualunque progetto:

```bash
cd /workspace/Shop.heads/payment
git status
git branch --show-current
```

Il branch sarà normalmente:

```text
hydra/payment
```

Da questo momento puoi modificare file, eseguire test e creare commit senza
condividere working tree o index con il repository originale o con altre Head.

### 4.4 Crea un’altra Head da una Head esistente

Se `.hydra.json` è stato committato ed è quindi presente nella Head, Hydra può
essere eseguito anche da lì:

```bash
cd /workspace/Shop.heads/payment
hydra head create auth
```

Il locator locale condiviso tramite il Git common directory fa sì che `auth`
venga creata nella stessa directory:

```text
/workspace/Shop.heads/auth
```

La posizione non viene ricalcolata rispetto a `payment`.

### 4.5 Stato attuale del ciclo di vita

Oggi Hydra crea e registra le Head, ma non espone ancora i comandi per
chiuderle o rimuoverle. Evita di cancellare manualmente una directory Head o di
usare `git worktree remove`: l’inventario Hydra rimarrebbe incoerente.

Fino all’implementazione della chiusura, il flusso supportato termina con il
lavoro e i commit sul branch privato della Head.

---

## 5. Scegliere origine e destinazione

### Origine predefinita

Senza `--from`, Hydra usa `HEAD`:

```bash
hydra head create payment
```

### Origine esplicita

Puoi partire da un branch, ref o commit:

```bash
hydra head create payment --from beta
hydra head create payment --from refs/heads/beta
hydra head create payment --from 0123456789abcdef
```

Hydra registra sia la ref richiesta sia il commit esatto risolto.

### Branch di integrazione

Quando `--from` risolve un branch locale, quel branch diventa anche il target
predefinito.

Puoi indicare esplicitamente un altro branch locale:

```bash
hydra head create payment --from beta --target main
```

Se `--from` è un commit detached, un tag o un’altra origine che non identifica
un branch locale, `--target` è obbligatorio:

```bash
hydra head create experiment \
  --from 0123456789abcdef \
  --target main
```

Hydra non esegue il merge durante la creazione: `targetRef` registra la
destinazione prevista per la futura chiusura.

---

## 6. Nomi e branch delle Head

Un nome valido:

- inizia con un carattere ASCII alfanumerico;
- continua con caratteri ASCII alfanumerici, `.`, `-` oppure `_`;
- non contiene `..`;
- non termina con l’estensione `.lock`, senza distinzione tra maiuscole e
  minuscole.

Esempi validi:

```text
payment
auth-v2
issue_123
release.1
```

Esempi non validi:

```text
../payment
-payment
auth/refresh
payment.lock
```

Il branch privato è formato da:

```text
<branchPrefix><nome-head>
```

Con il default:

```json
{
  "branchPrefix": "hydra/"
}
```

la Head `payment` usa `hydra/payment`. Il risultato completo deve essere un
nome di branch valido per Git e non deve esistere già.

---

## 7. Overlay: file ignorati da copiare

I file tracciati provengono sempre dal commit di base. Gli overlay permettono
di aggiungere alla nuova Head file non tracciati o ignorati presenti nella
worktree da cui esegui il comando.

La configurazione predefinita contiene:

```json
{
  "overlay": {
    "copy": [
      "... .gitignore"
    ]
  }
}
```

La direttiva:

```text
... .gitignore
```

espande, in quella posizione, le regole del `.gitignore` corrente. Per esempio:

```gitignore
.env
cache/
!cache/logs/
```

Hydra seleziona `.env` e i file ignorati sotto `cache/`, rispettando ordine,
negazioni e precedenze della sintassi Gitignore.

Puoi combinare direttive espanse e pattern espliciti:

```json
{
  "overlay": {
    "copy": [
      "... .gitignore",
      ".tool-cache/",
      "!.tool-cache/private/"
    ]
  }
}
```

### Protezioni

Hydra rifiuta:

- `.git` e qualunque percorso al suo interno;
- percorsi assoluti o con traversal;
- symlink e file speciali selezionati;
- un overlay che sovrascriverebbe un file tracciato;
- una sorgente che esce dal repository;
- una sorgente modificata durante la materializzazione.

Un overlay può contenere credenziali o configurazioni locali. Hydra lo copia
nella Head, ma non lo rende sicuro automaticamente: mantieni correttamente
ignorati i segreti e controlla sempre `git status`.

### Conferma della copia completa

Hydra tenta il copy-on-write per ogni overlay. Se alcuni file richiedono una
copia completa, mostra soltanto il sottoinsieme interessato:

```text
Full copy required: 2 file(s), 1048576 byte(s)
Continue? [y/N]
```

Rispondono positivamente soltanto `y` e `yes`, senza distinzione tra maiuscole
e minuscole. Invio, EOF o qualunque altra risposta annullano la creazione prima
delle mutazioni Git.

---

## 8. Storage

La sola modalità attualmente supportata è:

```json
{
  "storage": {
    "mode": "auto"
  }
}
```

Hydra verifica il volume che ospita le Head:

- su APFS tenta il clone nativo;
- su filesystem Linux compatibili tenta il reflink;
- quando il copy-on-write non è disponibile usa una copia completa isolata.

Il backend è una decisione locale. Non viene inserito nella configurazione
versionata; Hydra registra invece il backend effettivamente usato da ogni Head.

Modalità come `cow-only`, `copy` o preferenze globali non sono ancora
disponibili.

---

## 9. Posizione della directory delle Head

### Default disponibile tramite `hydra init`

La configurazione generata è:

```json
{
  "headsDirectory": {
    "strategy": "sibling",
    "suffix": ".heads"
  }
}
```

Per un progetto `Shop`, Hydra risolve `Shop.heads` accanto al repository.

Il suffisso può essere qualunque stringa non vuota, inclusi spazi, Unicode e
punteggiatura. Non può contenere caratteri di controllo, `/` o `\`, perché deve
rimanere un frammento di nome e non un percorso.

Esempi validi:

```json
{"strategy": "sibling", "suffix": "heads"}
```

```json
{"strategy": "sibling", "suffix": " workspace 🚀"}
```

### Strategy comprese dal motore

Il lettore della configurazione comprende anche:

```json
{
  "strategy": "relative",
  "base": "repositoryParent",
  "path": "workspaces/shop-heads"
}
```

e:

```json
{
  "strategy": "local"
}
```

`relative` risolve un percorso portabile rispetto alla directory che contiene
il progetto. `local` usa esclusivamente il percorso assoluto memorizzato nel
locator non versionato.

Queste due strategy non dispongono ancora di opzioni CLI per inizializzazione o
relocation. Non modificare manualmente la policy, il locator o la directory di
un progetto già inizializzato: Hydra rileva la divergenza e rifiuta di mutare
lo stato. Un comando sicuro di configurazione o repair verrà documentato
quando sarà disponibile.

---

## 10. Configurazione condivisa

Un `.hydra.json` completo generato oggi ha questa forma:

```json
{
  "version": 2,
  "projectId": "shop-0123456789abcdef0123456789abcdef",
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

Regole:

- `version` deve essere `2`; il formato sperimentale v1 non è supportato;
- `projectId` identifica il progetto tra dispositivi e non deve essere
  modificato;
- `headsDirectory` è una politica portabile, non un percorso assoluto
  condiviso;
- `branchPrefix` viene anteposto al nome della Head;
- `storage.mode` accetta oggi soltanto `auto`;
- `overlay.copy` contiene regole Gitignore e direttive `...`.

Hydra rifiuta campi sconosciuti e campi appartenenti a una strategy diversa.
Versiona `.hydra.json`, ma non inserire percorsi assoluti o informazioni
specifiche della macchina.

---

## 11. Stato locale: non modificarlo manualmente

Hydra separa:

```text
<git-common-dir>/hydra/project.json
<heads-directory>/.hydra/directory.json
<heads-directory>/.hydra/heads.json
```

- `project.json` individua l’installazione locale da qualunque worktree;
- `directory.json` prova l’ownership tramite `projectId` e `installationId`;
- `heads.json` contiene l’inventario fisico delle Head locali.

Questi file:

- non sono versionati;
- contengono percorsi e stato specifici della macchina;
- vengono verificati prima di ogni mutazione;
- non devono essere copiati tra collaboratori o modificati a mano.

Hydra rifiuta ownership incoerente, locator e policy divergenti, directory
metadata symlinkate e destinazioni annidate dentro altre worktree.

---

## 12. Risoluzione dei problemi

### “Hydra is already initialized”

Esiste già `.hydra.json`. Non eseguire nuovamente `hydra init` e non cancellare
metadati finché non è disponibile un comando di repair.

### “configuration version 1 is not supported”

Il formato v1 era sperimentale e non viene migrato. Poiché Hydra non è ancora
distribuito, ricrea il progetto o fixture di sviluppo e inizializzalo con il
binario corrente.

### “--target is required”

La ref passata a `--from` non identifica un branch locale. Specifica il branch
locale previsto per l’integrazione:

```bash
hydra head create experiment --from <COMMIT> --target main
```

### “directory ownership does not match”

Locator e marker non descrivono la stessa installazione. Non correggere gli ID
a mano e non riutilizzare implicitamente la directory. Il flusso guidato di
repair è pianificato ma non ancora disponibile.

### “does not match the versioned directory policy”

La configurazione condivisa e il locator locale risolvono directory diverse.
Questo può accadere dopo modifiche o spostamenti manuali. Ripristina la
configurazione conosciuta senza cancellare le Head; la relocation assistita non
è ancora disponibile.

### Esiste `heads.json.lock`

Un’operazione Hydra potrebbe essere attiva oppure essersi interrotta. Non
eliminare automaticamente il lock: prima verifica processi, worktree, branch e
inventario. La riconciliazione automatica è pianificata.

---

## 13. Pianificato — non ancora disponibile

Il contratto MVP comprende:

- elenco e stato delle Head;
- risoluzione e apertura del percorso di una Head;
- apertura tramite comando configurabile;
- chiusura con merge isolato o adapter configurabile;
- rimozione protetta;
- completamento della shell per i nomi delle Head;
- diagnostica del backend storage;
- repair e riconciliazione;
- output JSON per automazioni;
- una skill installabile che insegni agli agenti AI a usare questi flussi senza
  aggirare le protezioni di Hydra.

La sintassi definitiva verrà aggiunta a questa guida soltanto insieme
all’implementazione e all’help del binario.

---

## 14. Regola di manutenzione

Questa guida deve cambiare nello stesso intervento che modifica:

- comandi, argomenti, opzioni, default o output;
- schema e validazione della configurazione;
- flusso base o avanzato;
- comportamento Git, filesystem, overlay o storage visibile all’utente;
- messaggi di errore che richiedono un’azione diversa;
- disponibilità di una funzionalità precedentemente pianificata.

Una funzionalità passa da **Pianificato** a **Disponibile** soltanto quando il
codice, l’help e i test ne dimostrano il comportamento.

Per intenti di prodotto e dettagli tecnici:

- [contesto MVP](../product/hydra-mvp-context.md);
- [inizializzazione](../architecture/project-initialization.md);
- [creazione delle Head](../architecture/head-creation.md).
