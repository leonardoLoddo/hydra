# Guida all’uso di Hydra

Questa guida descrive come usare Hydra e come personalizzarne il comportamento.
È mantenuta insieme al codice: comandi e opzioni presentati come disponibili
devono esistere nel binario corrente.

Hydra è ancora una preview iniziale. L'ultima release pubblica è disponibile
tramite Homebrew e
[GitHub Releases](https://github.com/leonardoLoddo/hydra/releases/latest).

La [documentazione utente inglese](hydra-user-guide.md) offre lo stesso
perimetro in pagine tematiche navigabili.

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
hydra status
hydra repair
hydra doctor storage
hydra completions <SHELL>
hydra skill install <PROVIDER> [--yes | --no]
hydra skill status <PROVIDER>
hydra skill update <PROVIDER> [--yes | --no]
hydra skill remove <PROVIDER> [--yes | --no]
hydra head create <NAME> [--from <REF>] [--target <BRANCH>]
hydra head list
hydra head status <NAME>
hydra head path <NAME>
hydra head open <NAME>
hydra head close <NAME>
hydra head remove <NAME> [--force]
```

Puoi verificare la sintassi installata con:

```bash
hydra --help
hydra init --help
hydra repair --help
hydra doctor storage --help
hydra completions --help
hydra skill --help
hydra head --help
hydra head create --help
hydra head open --help
hydra head close --help
hydra head remove --help
```

### Completamento della shell

Hydra supporta Bash, Zsh e Fish. La Formula Homebrew installa automaticamente
i completamenti dinamici nei percorsi standard del package manager senza
modificare i file personali della shell. Dopo installazione o aggiornamento,
riavvia la shell; se questa carica i completamenti Homebrew, non serve altra
configurazione Hydra.

Per installazioni da sorgente o archivio, oppure se Tab non è ancora
disponibile, carica manualmente la registrazione ad ogni avvio della shell,
così rimane allineata con il binario installato.

Per Bash, aggiungi a `~/.bashrc`:

```bash
source <(hydra completions bash)
```

Per Zsh, aggiungi a `~/.zshrc`:

```zsh
source <(hydra completions zsh)
```

Per Fish, aggiungi a `~/.config/fish/config.fish`:

```fish
hydra completions fish | source
```

Il completamento propone comandi e opzioni. Nei comandi `head status`, `head
path`, `head open`, `head close` e `head remove` propone anche i nomi delle
Head del progetto corrente. Non propone Head esistenti per `head create`,
perché quel comando richiede un nome nuovo.

Fuori da un progetto Hydra, oppure quando lo stato locale non è leggibile, la
ricerca dinamica non mostra errori: restituisce semplicemente zero nomi. Dopo
un aggiornamento di Hydra riavvia la shell o ricarica il relativo file di
configurazione. Se anche Homebrew richiede il fallback manuale, verifica prima
che il normale ambiente Homebrew sia inizializzato nella shell.

---

## 3. Installazione e aggiornamento

Su macOS, Linux nativo o WSL 2 installa la preview dal tap Homebrew dedicato:

```bash
brew install leonardoLoddo/tap/hydra-heads
```

La Formula si chiama `hydra-heads` per non entrare in conflitto con un altro
pacchetto Homebrew chiamato `hydra`, ma il comando installato rimane `hydra`.
Al termine Homebrew mostra l'artwork Hydra, `hydra --help` come punto di
ingresso e il comando opzionale `hydra skill install codex`. La skill non viene
mai installata automaticamente.

Su WSL verifica da PowerShell che la distribuzione usi WSL 2 con `wsl -l -v` e
configura Homebrew nel prefisso Linux predefinito prima dell'installazione.
WSL 1 non è supportato. La Formula include gli URL nativi Linux e WSL 2 la usa
con supporto di qualità preview.

Su Windows 11 x86-64 nativo scarica
[`hydra-windows-x86_64.zip`](https://github.com/leonardoLoddo/hydra/releases/latest/download/hydra-windows-x86_64.zip)
e il relativo
[`checksum SHA-256`](https://github.com/leonardoLoddo/hydra/releases/latest/download/hydra-windows-x86_64.zip.sha256).
La stessa release conserva anche l'archivio immutabile
`hydra-<version>-x86_64-pc-windows-msvc.zip`. Verifica il checksum, estrai
`hydra.exe` in una directory stabile del `PATH` utente di Windows, riavvia Git
Bash e verifica:

```bash
hydra.exe --version
command -v hydra
hydra --help
```

Git Bash risolve automaticamente `hydra` in `hydra.exe`, quindi gli esempi
restano invariati. Git for Windows deve essere installato e raggiungibile nel
`PATH`. PowerShell può essere usato per installare il binario, ma il flusso
operativo e i completamenti documentati usano Git Bash.

Il packaging Windows nativo è disponibile a partire da `v0.2.0`. Il link
stabile risolve l'ultima GitHub Release, mentre l'archivio versionato rimane
disponibile per download riproducibili.

Per aggiornare il binario e ogni copia provider gestita separatamente:

```bash
brew update
brew upgrade leonardoLoddo/tap/hydra-heads
hydra skill status codex
hydra skill update codex
hydra skill status gemini
hydra skill update gemini
```

Per rimuovere il binario e, soltanto se lo desideri, la skill:

```bash
hydra skill remove codex
hydra skill remove gemini
brew uninstall leonardoLoddo/tap/hydra-heads
```

La disinstallazione Homebrew non rimuove una skill posseduta o modificata
dall'utente.

In alternativa, durante lo sviluppo puoi installare il binario dalla root del
repository sorgente con il toolchain Rust fissato dal progetto:

```bash
cargo install --path crates/hydra-cli --locked --force
```

Verifica quale binario viene eseguito:

```bash
hydra --version
command -v hydra
```

Homebrew e Cargo possono installare due binari distinti senza cancellarsi a
vicenda. Se sono presenti entrambi, l'ordine del `PATH` determina quale viene
eseguito; `which -a hydra` mostra tutte le copie raggiungibili.

### 3.1 Installa la skill per agenti AI

Il repository distribuisce un'unica skill operativa canonica in
`skills/hydra/`. Il binario la installa attraverso adapter distinti nelle
directory personali documentate dai provider:

| Provider | Valore | Destinazione |
|---|---|---|
| Codex | `codex` | `$HOME/.agents/skills/hydra` |
| Gemini CLI | `gemini` | `$HOME/.gemini/skills/hydra` |
| Antigravity CLI | `agy` | `$HOME/.gemini/antigravity-cli/skills/hydra` |
| App Antigravity | `antigravity` | `$HOME/.gemini/config/skills/hydra` |

Hydra gestisce le destinazioni in modo indipendente. Gemini CLI riconosce anche
`$HOME/.agents/skills`, quindi può usare la copia installata con `codex` senza
una seconda installazione. Usa `gemini` se vuoi una copia separata nella sua
directory personale nativa. AGY e l'app Antigravity richiedono invece le loro
destinazioni globali dedicate. Il provider usato per `status`, `update` e
`remove` deve sempre corrispondere a quello che ha installato la copia.

Installa le copie indipendenti che vuoi usare:

```bash
hydra skill install codex
hydra skill install gemini
hydra skill install agy
hydra skill install antigravity
```

Su un terminale interattivo Hydra mostra la destinazione e chiede conferma con
default negativo. Un input vuoto o non disponibile non installa nulla. Per
automazioni la scelta deve essere esplicita:

```bash
hydra skill install codex --yes
hydra skill install codex --no
```

Una directory già esistente viene preservata. Hydra aggiunge un manifest di
provenienza con versione e checksum SHA-256; `update` e `remove` operano solo
quando manifest, struttura e contenuti dimostrano che la copia è stata gestita
da Hydra e non è stata modificata localmente:

```bash
hydra skill status codex
hydra skill update codex
hydra skill remove codex

hydra skill status gemini
hydra skill update gemini
hydra skill remove gemini

hydra skill status agy
hydra skill update agy
hydra skill remove agy

hydra skill status antigravity
hydra skill update antigravity
hydra skill remove antigravity
```

Anche aggiornamento e rimozione usano una conferma predefinita negativa quando
devono modificare lo stato; `--yes` e `--no` sono disponibili per automazioni.
Una skill sconosciuta, un symlink, un file aggiuntivo o un checksum diverso
causano un rifiuto sicuro senza sovrascrittura o cancellazione.

Codex rileva normalmente le modifiche automaticamente. Gemini CLI può
riscansionare le skill con `/skills reload`. In Antigravity CLI usa `/skills`
per verificare le skill caricate e riavvia l'host se la nuova copia non è
visibile. Nell'app Antigravity riavvia se necessario e controlla
**Settings > Customizations > Skills**. Puoi invocare la skill esplicitamente,
per esempio:

```text
Usa la skill Hydra per sviluppare questa attività in una Head isolata basata su main.
```

La skill verifica i comandi realmente installati, crea o seleziona una Head e
sposta il lavoro nella directory restituita da `hydra head path`. Non cattura
le modifiche non committate del workspace di partenza: una nuova Head nasce
dal commit risolto tramite `--from`.

Per sicurezza, al termine lascia normalmente la Head disponibile alla
revisione. Non esegue automaticamente `hydra head close`,
`hydra head remove --force`, modifiche ai metadati locali o cancellazioni
manuali di worktree. Integrazione e scarto di file richiedono
un'autorizzazione esplicita.

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

Se `.hydra.json` manca ma la directory sorella e il locator locale esistono
già, `hydra init` li riutilizza soltanto quando marker, percorsi e identità
coincidono, l'inventario è vuoto e non esiste alcun altro contenuto operativo.
Conserva byte per byte i metadati locali, verifica di nuovo lo storage e ricrea
la configurazione predefinita con lo stesso `projectId`. Revisiona e versiona
anche questa configurazione. Una directory estranea, incompleta, non vuota o
con ownership discordante viene preservata e rifiutata.

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

Se `stderr` è un terminale interattivo, durante le creazioni più lunghe Hydra
mostra brevi messaggi di avanzamento per fase, per esempio:

```text
Planning overlays...
Materializing 1840 tracked entries...
Materializing 10000 overlay entries...
```

Questi messaggi non vengono emessi quando `stderr` è rediretto o catturato da
un'automazione; l'output finale del comando rimane invariato.

Se gli overlay selezionano link simbolici non sicuri, Hydra li elenca tutti
prima di creare branch o worktree e può proporti di escluderli dalla
configurazione condivisa. Il flusso è descritto nella sezione
[Symlink non sicuri](#symlink-non-sicuri).

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

### 4.4 Usa Hydra da una Head esistente

Tutti i comandi Hydra possono essere eseguiti dal progetto principale o da una
qualsiasi Head gestita. Il locator nel Git common directory riporta sempre il
comando al `projectRoot` canonico: Hydra usa la configurazione corrente, la
`HEAD`, gli overlay e l'inventario del progetto padre anche se `.hydra.json`
manca nella Head chiamante o contiene una versione precedente. Per esempio:

```bash
cd /workspace/Shop.heads/payment
hydra head create auth
```

`auth` viene creata nella stessa directory delle Head:

```text
/workspace/Shop.heads/auth
```

La posizione non viene ricalcolata rispetto a `payment`: le Head sono sorelle
del progetto principale e non formano una gerarchia annidata. Una Head non può
quindi possedere proprie sotto-Head indipendenti; può soltanto operare sulle
Head dello stesso progetto Hydra. Senza `--from` e `--target`, `auth` usa la
`HEAD` e il branch locale del progetto padre, non `hydra/payment`; file tracked,
overlay e modifiche locali di `payment` non vengono ereditati implicitamente.

Anche `hydra init` lanciato da una Head riconosce l'inizializzazione del padre:
segnala la configurazione canonica già esistente e non prova a trasformare la
Head in un progetto Hydra separato.

Se chiudi la Head dalla sua stessa directory, Hydra può rimuoverla in sicurezza
ma la shell resta posizionata su un percorso ormai eliminato. Dopo il comando
spostati nel progetto padre o in un'altra Head prima di eseguire altri comandi.

### 4.5 Elenca e ispeziona le Head

Per ottenere i nomi delle Head locali in ordine alfabetico:

```bash
hydra head list
```

L'output contiene un nome per riga ed è quindi facile da usare in shell:

```text
auth
payment
```

Per una sintesi dell'intero progetto:

```bash
hydra status
```

Esempio:

```text
Project: /workspace/Shop
Heads directory: /workspace/Shop.heads
Heads: 2
  auth  clean
  payment  modified
```

Lo stato sintetico può essere:

- `clean`, quando Git non rileva modifiche;
- `modified`, quando esistono modifiche tracked, staged o untracked;
- `inconsistent`, quando inventario, Git e filesystem non concordano.

Per ispezionare una singola Head:

```bash
hydra head status payment
```

Hydra mostra nome, percorso, branch e commit correnti, base e target registrati,
conteggi delle modifiche, ahead/behind, presenza della worktree e coerenza:

```text
Head: payment
Path: /workspace/Shop.heads/payment
Branch: refs/heads/hydra/payment
Commit: 8f22a84...
Base: refs/heads/main (619be35...)
Target: refs/heads/main
Changes: 1 modified, 0 added, 0 deleted, 1 untracked
Ahead/behind: 2/1
Worktree: present
Consistency: ok
```

Branch, commit e modifiche descrivono ciò che è realmente aperto nella
worktree. Se usi Git per passare a un altro branch, Hydra conserva anche
l'intenzione registrata e la rende esplicita:

```text
Branch: refs/heads/alternate (expected refs/heads/hydra/payment)
Consistency: worktree branch does not match metadata
```

Ahead/behind confronta il commit osservato nella worktree con lo stato corrente
della `baseRef`. Il commit tra parentesi nella riga `Base` è invece il commit
esatto usato durante la creazione. Se la base ref non esiste più, Hydra segnala
l'incoerenza e usa quel commit esatto come riferimento di fallback. Se
l'origine era uno SHA o un'altra espressione non simbolica, il confronto usa
sempre il `baseCommit` completo e non reinterpreta il testo originale.

Per ottenere soltanto il percorso assoluto registrato:

```bash
hydra head path payment
```

Questo formato è intenzionalmente componibile:

```bash
cd "$(hydra head path payment)"
```

Quando stdout è collegato a una pipeline, Hydra conserva esattamente i byte del
percorso. Quando il comando scrive direttamente su un terminale, eventuali
caratteri di controllo vengono mostrati come escape testuali per non produrre
sequenze terminale ambigue o pericolose. Anche i percorsi negli output umani di
`status` sono sempre neutralizzati.

I quattro comandi di ispezione sono read-only: non creano il lock
`heads.json.lock`, non aggiornano i metadati e non eseguono repair. Se una
directory o una ref manca, `status` segnala l'incoerenza senza correggerla. Un
percorso registrato che esce dalla directory delle Head posseduta viene
rifiutato.

### 4.6 Apri una Head

Hydra non sceglie implicitamente un editor. Configura prima un adapter
condiviso in `.hydra.json`, per esempio:

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

Poi esegui:

```bash
hydra head open payment
```

Prima di avviare il programma, Hydra verifica che path, registrazione Git e
branch della worktree corrispondano ai metadati. Il processo parte dalla
directory della Head. Hydra attende il suo esito: un exit code non-zero rende
fallito il comando ma non modifica Head, branch o inventario.

### 4.7 Rimuovi una Head

La rimozione ordinaria è protetta:

```bash
hydra head remove payment
```

Hydra procede soltanto se la worktree è pulita e tutti i commit del branch
privato sono già integrati nel target registrato. In questo caso elimina
worktree, voce di inventario e branch privato:

```text
Removed Head payment
```

Modifiche tracked, staged o untracked bloccano il comando. Anche commit non
integrati bloccano la rimozione ordinaria.

Per scartare esplicitamente le modifiche non committate:

```bash
hydra head remove payment --force
```

`--force` può eliminare definitivamente quei file, ma non cancella commit
recuperabili. Se il branch contiene commit non integrati, Hydra conserva la
ref:

```text
Removed Head payment
Preserved branch refs/heads/hydra/payment with unintegrated commits
```

Puoi quindi ispezionarla con i normali comandi Git. `--force` non aggira path
non sicuri, ownership errata, worktree mancanti o non registrate, branch
divergenti dai metadati o target scomparsi.

### 4.8 Chiudi e integra una Head

Una Head pulita può essere integrata nel target registrato e rimossa:

```bash
hydra head close payment
```

Esegui il comando dalla worktree padre canonica. Per la chiusura nativa, il
target registrato, per esempio `main`, deve essere checkoutato lì. Se avvii il
comando da una Head, Hydra rifiuta la chiusura prima di qualsiasi modifica e
mostra il path del progetto padre. Anche gli adapter configurati rispettano il
vincolo della worktree padre, ma vengono eseguiti dentro la Head.

Hydra esegue un normale `git merge` del commit validato della Head nella
worktree padre. Vedi quindi direttamente l'output ordinario di Git per
fast-forward, merge commit e conflitti.

Una worktree target con modifiche staged, modificate, eliminate o non tracciate,
oppure con un merge, rebase o altra operazione Git in corso, blocca la chiusura:
Hydra descrive il motivo e non modifica né il target né la Head. Non serve
creare un branch temporaneo quando il target è già pulito.

In caso di conflitto Hydra resta in esecuzione e lascia il normale stato di
merge nella worktree padre. Risolvi i file e crea il commit di merge con l'IDE
o un altro terminale: Hydra verifica il commit e riprende automaticamente la
rimozione protetta. Se esegui `git merge --abort`, Hydra abortisce anche la
chiusura, conserva la Head e termina con errore.

Al successo Hydra indica sia dove ha integrato sia il risultato:

```text
Closed Head payment into refs/heads/main at <commit>
Integration strategy: target worktree /workspace/Shop
Integration result: fast-forward
```

Il risultato può essere `already integrated`, `fast-forward` o `merge commit`.
Se l'integrazione riesce ma la rimozione protetta fallisce, l'errore indica il
commit già pubblicato: non tentare di annullarlo manualmente e ripeti la
diagnostica sulla Head rimasta.

Puoi sostituire l'integrazione nativa con un comando configurato:

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

Avvia `head close` dalla worktree padre. Hydra esegue poi il programma dalla
directory validata della Head, passa ogni argomento separatamente e attende il
risultato. Il programma è codice fidato del progetto: non viene eseguito in una
sandbox e può modificare Git, file o servizi con i permessi dell'utente.

Con `removeOnSuccess: false`, un comando riuscito conserva worktree, branch e
inventario:

```text
Close command completed for Head payment; Head preserved
```

Con `removeOnSuccess: true`, Hydra tenta successivamente la normale rimozione
protetta, senza `--force`. L'adapter deve quindi avere integrato i commit nel
target e lasciato la Head pulita e coerente. Se la rimozione fallisce, Hydra
distingue il comando già completato dalla rimozione non eseguita e conserva la
Head.

Se l'adapter termina con un codice non-zero, Hydra non rimuove la Head. Se nel
frattempo il comando ha modificato o eliminato `targetRef`, l'errore confronta
il commit osservato prima e dopo. Hydra non tenta un rollback degli effetti
prodotti da un programma arbitrario.

#### Esempio: esegui una verifica e conserva la Head

In un progetto Rust puoi usare la chiusura come gate esplicito senza integrare
o rimuovere nulla:

```json
{
  "commands": {
    "close": {
      "strategy": "command",
      "program": "cargo",
      "args": ["test", "--workspace"],
      "removeOnSuccess": false
    }
  }
}
```

Con una Head pulita:

```bash
hydra head close payment
```

Hydra esegue `cargo test --workspace` dentro la Head. Se i test passano, stampa:

```text
Close command completed for Head payment; Head preserved
```

Se i test falliscono, `head close` termina con errore e non tenta la rimozione.
Sostituisci `cargo` e `args` con il comando di verifica previsto dal tuo
progetto.

#### Esempio: apri una pull request e conserva la Head

Un progetto ospitato su GitHub può affidare la pubblicazione a uno script
versionato, mantenendo la Head locale per revisioni successive:

```json
{
  "commands": {
    "close": {
      "strategy": "command",
      "program": "./tools/open-head-pr",
      "args": ["{headRef}", "{targetRef}"],
      "removeOnSuccess": false
    }
  }
}
```

Un possibile `tools/open-head-pr` è:

```bash
#!/usr/bin/env bash
set -euo pipefail

head_branch=${1#refs/heads/}
target_branch=${2#refs/heads/}

git push --set-upstream origin "$head_branch"
gh pr create --head "$head_branch" --base "$target_branch"
```

Rendi eseguibile e versiona lo script:

```bash
chmod +x tools/open-head-pr
git add tools/open-head-pr .hydra.json
git commit -m "chore: configure Hydra pull request workflow"
```

Questo esempio richiede GitHub CLI installata e autenticata. Il push e la
creazione della pull request sono effetti esterni dello script: Hydra non può
annullarli se `gh` fallisce dopo il push.

#### Esempio: integra tramite uno strumento del progetto e rimuovi

Se il repository possiede già un comando affidabile che integra il branch
privato nel target locale, puoi chiedere la rimozione successiva:

```json
{
  "commands": {
    "close": {
      "strategy": "command",
      "program": "./tools/integrate-head",
      "args": ["{path}", "{headRef}", "{targetRef}"],
      "removeOnSuccess": true
    }
  }
}
```

Prima di restituire exit code zero, `tools/integrate-head` deve lasciare la
Head pulita e fare in modo che il commit di `{headRef}` sia raggiungibile da
`{targetRef}`. Deve inoltre evitare di spostare una ref aperta in un'altra
worktree senza aggiornare in modo coerente file e index. Se una di queste
condizioni non è soddisfatta, la rimozione protetta fallisce e Hydra conserva
la Head. Per una normale integrazione Git locale continua a preferire la
strategia nativa, che implementa già queste protezioni.

### 4.9 Ripara inventario e worktree

Per confrontare lo stato locale di Hydra con le worktree e i branch Git:

```bash
hydra repair
```

Se tutto è coerente, il comando termina senza modifiche:

```text
Hydra state is consistent.
```

Hydra può proporti sei correzioni deterministiche:

- rimuovere un `heads.json.lock` abbandonato, ma solo se appartiene al formato
  corrente e il guard del sistema operativo dimostra che nessun processo Hydra
  lo possiede;
- ricostruire un `heads.json` assente dai record di recupero delle Head, ma
  solo se tutte le worktree con prefisso Hydra hanno evidenza coerente con
  nome, percorso gestito e branch Git;
- aggiungere a un inventario esistente una Head omessa dopo un crash, ma solo
  quando la worktree registrata e il suo manifest coincidono esattamente per
  nome, percorso gestito e branch;
- ripulire una creazione interrotta prima della registrazione della worktree,
  ma soltanto quando il journal durevole coincide con nome, percorso, ref e
  commit di base, non esistono directory o worktree associate e il branch non
  è avanzato;
- rimuovere dall’inventario una Head la cui directory e registrazione Git non
  esistono più, conservando sempre il branch privato;
- riportare nel percorso gestito una worktree spostata, quando Git associa in
  modo univoco quel percorso al branch privato registrato.

Tutte richiedono una conferma esplicita. Una risposta vuota o negativa non
applica modifiche. Dopo la conferma, Hydra riacquisisce la protezione necessaria,
ricontrolla lo stato corrente e salta una correzione che nel frattempo non è
più valida.

Per un lock corrente riconosciuto ma non più posseduto da un processo, Hydra
mostra il percorso e chiede:

```text
Remove the abandoned Hydra state lock? [y/N]
```

Se confermi, Hydra riacquisisce il guard, ricontrolla il file e rimuove soltanto
il marker effimero. Inventario, worktree, branch e marker di ownership restano
invariati. Il comando termina dopo questa correzione: esegui di nuovo
`hydra repair` per pianificare eventuali altre riparazioni.

Per l'inventario mancante, Hydra elenca le Head recuperabili e chiede, per
esempio:

```text
Rebuild the missing inventory with 1 recovered Head? [y/N]
```

Se confermi, Hydra ricontrolla l'intero insieme sotto lock e crea
atomicamente l'inventario senza modificare worktree o branch. È sufficiente il
record centrale oppure quello privato; se esistono entrambi devono coincidere.
Se una Head non ha evidenza coerente con Git, non viene creato un inventario
parziale.

Un record di recupero malformato, di versione non supportata o non
rappresentato da un file regolare interrompe la validazione senza modificare
record, worktree, branch o inventario.

Se l'inventario esiste ma omette una Head completa dotata di evidenza di recupero, Hydra la
mostra come recuperabile e chiede, per esempio:

```text
Add 1 recovered Head to the inventory? [y/N]
```

Se confermi, Hydra ricontrolla sotto lock l'intero insieme approvato e aggiunge
atomicamente i metadati esatti senza cambiare le voci già registrate. Se il
manifest, Git o l'inventario cambiano durante la conferma, non adotta nessuna
Head. L'assenza di entrambi i record o la loro incoerenza lascia la worktree in
sola segnalazione e non autorizza metadati dedotti.

Per una creazione interrotta prima della worktree, Hydra mostra il nome e
richiede conferma. Ricontrolla quindi tutto sotto lock e rimuove l'eventuale
branch soltanto se punta ancora esattamente al commit di base registrato nel
journal. Se il branch è avanzato o sono comparsi path o worktree, non elimina
nulla e richiede diagnosi.

Altre incoerenze vengono soltanto segnalate: worktree Hydra senza evidenza di
recupero verificabile, directory registrate ma mancanti, directory non registrate,
branch assenti o differenti e associazioni ambigue. Hydra conserva file e ref
perché Git da solo non contiene informazioni sufficienti per ricostruire con
certezza base, target, backend e intenzione originaria.

`repair` non elimina lock attivi; un lock vuoto o malformato e una versione
diversa da quella corrente falliscono invece la validazione senza essere
modificati. Hydra non migra i precedenti formati lock sperimentali perché non
sono mai stati distribuiti. Il comando non corregge ownership o locator e non
reloca l'intera directory delle Head. Un inventario malformato viene rifiutato
e conservato,
non sostituito con i manifest.

### 4.10 Stato attuale del ciclo di vita

Oggi Hydra crea, registra, ispeziona, integra, rimuove e riconcilia le Head.
Evita comunque di cancellare manualmente una directory Head o di usare
`git worktree remove`: `hydra repair` può recuperare solo gli stati
deterministici descritti sopra.

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
destinazione prevista per la successiva chiusura.

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
di aggiungere alla nuova Head file non tracciati o ignorati presenti nel
progetto padre canonico, anche quando esegui il comando da una Head.

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
- symlink assoluti, rotti o che risolvono fuori dal progetto;
- file speciali selezionati;
- un overlay che sovrascriverebbe un file tracciato;
- una sorgente che esce dal repository;
- un file materializzato che non corrisponde più all'identità calcolata durante
  la pianificazione.

Su macOS e Linux, Hydra conserva i symlink relativi che rimangono all’interno
del progetto. Il link viene ricreato nella Head con lo stesso target relativo e
deve risolvere dentro la Head anche dopo la materializzazione. Directory di
dipendenze come `node_modules` e `vendor` possono quindi mantenere i launcher
presenti in `node_modules/.bin` o `vendor/bin` senza riferimenti al workspace
sorgente. Su Windows i symlink tracciati e quelli selezionati dagli overlay
restano non supportati; i repository senza tali entry usano il normale flusso
Windows.

Directory di dipendenze con migliaia di file vengono pianificate in batch:
Hydra non avvia un processo Git separato per ogni file e distribuisce i batch
indipendenti su un numero limitato di worker, mantenendo l'ordine degli hash.
Ogni file regolare viene comunque provato singolarmente verso il volume delle
Head, così un successo copy-on-write non nasconde il fallback necessario per
un altro file. Dopo la copia Hydra confronta il file nella Head con l'identità
pianificata; una modifica successiva della sorgente non invalida un clone CoW
già isolato e verificato. La Head contiene file e symlink completi, quindi
comandi come `composer run dev` o gli script npm possono usare direttamente le
dipendenze materializzate.

Un overlay può contenere credenziali o configurazioni locali. Hydra lo copia
nella Head, ma non lo rende sicuro automaticamente: mantieni correttamente
ignorati i segreti e controlla sempre `git status`.

### Symlink non sicuri

Un symlink overlay assoluto, rotto o che risolve fuori dal progetto non può
essere ricreato in sicurezza nella Head. Hydra raccoglie tutti i casi rilevati
e chiede:

```text
Unsafe overlay symlinks:
  links/escape
  public/storage
Exclude them and update .hydra.json? [y/N]
```

Rispondendo `y` o `yes`, Hydra aggiunge in fondo a `overlay.copy` esclusioni
letterali e ancorate:

```json
{
  "overlay": {
    "copy": [
      "... .gitignore",
      "!/links/escape",
      "!/public/storage"
    ]
  }
}
```

La pianificazione viene quindi ripetuta e la Head viene creata senza quei
link. Il comportamento è generale e non dipende dal nome `public/storage`:
vale per tutti i symlink non sicuri selezionati dagli overlay. I symlink
relativi che rimangono dentro il progetto continuano invece a essere
materializzati normalmente.

Invio, EOF, `n` o qualunque altra risposta annullano la creazione senza
modificare `.hydra.json`, branch, worktree o inventario. La configurazione
viene salvata atomicamente: non è mai visibile come JSON scritto a metà. Hydra
rifiuta una modifica che rileva rileggendo il file prima della pubblicazione,
ma il filesystem non fornisce un compare-and-swap portabile sul contenuto. Un
editor esterno che salva proprio tra quell'ultimo controllo e il rename può
ancora essere sovrascritto. Non modificare `.hydra.json` mentre questo prompt è
aperto. Dopo una risposta positiva, controlla e versiona subito la modifica:

```bash
git diff -- .hydra.json
git add .hydra.json
git commit -m "chore: exclude unsafe Hydra overlays"
```

L'esclusione confermata rimane anche se in seguito annulli una distinta
richiesta di copia completa: hai già autorizzato una modifica persistente alla
configurazione. File speciali, collisioni con file tracciati e symlink divenuti
non sicuri durante la materializzazione restano errori e non modificano
automaticamente le regole.

### Conferma della copia completa

Hydra materializza ogni file overlay regolare tentando prima il copy-on-write.
Durante la pianificazione prova ogni file senza generalizzare l'esito degli
altri. Se alcuni file richiedono una copia completa, mostra soltanto il
sottoinsieme interessato:

```text
Full copy required: 2 file(s), 1048576 byte(s)
Continue? [y/N]
```

Su Windows nativo o WSL, prima di `Continue?` Hydra mostra anche il link alla
guida CoW della piattaforma. Se il fallback emerge per file tracciati senza un
prompt overlay, il link appare nell'output finale dopo il backend. Una singola
creazione mostra la guida una sola volta.

Rispondono positivamente soltanto `y` e `yes`, senza distinzione tra maiuscole
e minuscole. Invio, EOF o qualunque altra risposta annullano la creazione prima
delle mutazioni Git.

---

## 8. Storage

La modalità predefinita è:

```json
{
  "storage": {
    "mode": "auto"
  }
}
```

Per forzare deterministicamente il fallback sicuro, per esempio in test o
automazioni, puoi configurare:

```json
{
  "storage": {
    "mode": "copy"
  }
}
```

In questa modalità Hydra non tenta il copy-on-write per i file regolari della
nuova Head. Gli overlay continuano a mostrare numero e peso logico e richiedono
la normale conferma esplicita prima di duplicare i byte.

Hydra verifica il volume che ospita le Head:

- su APFS tenta il clone nativo;
- su filesystem Linux compatibili, incluso un volume compatibile montato in
  WSL 2, tenta il reflink `FICLONE`;
- su volumi Windows ReFS compatibili, inclusi Dev Drive compatibili, tenta il
  block clone;
- quando il copy-on-write non è disponibile usa una copia completa isolata.

Per eseguire una diagnostica esplicita sul volume realmente gestito:

```bash
hydra doctor storage
```

Il comando crea una directory temporanea dentro la directory delle Head,
verifica i byte prodotti dal clone nativo e, separatamente quando necessario,
il fallback a copia completa. Un risultato tipico su APFS è:

```text
Storage backend: copy-on-write
Native primitive: APFS clone
Environment: native
Filesystem: unknown
Fallback: full copy (verified)
Mutable hard links: disabled
Isolation: supported
```

Se il clone nativo non è disponibile, Hydra mostra `Storage backend: full
copy` e `Native primitive: unavailable`. Gli hard link mutabili non vengono
mai usati come fallback. Il comando richiede un progetto Hydra inizializzato,
non acquisisce il lock dell’inventario e rimuove tutti gli artefatti della
prova; un fallimento di cleanup viene segnalato con il percorso rimasto.

### Copy-on-write su WSL 2

Su Linux il report identifica anche il filesystem che contiene la directory
delle Head. Su WSL 2 Hydra usa lo stesso adapter Linux della Formula Homebrew:
il root ext4 predefinito e i drive Windows esposti tramite DrvFs/9p possono
rifiutare `FICLONE`, quindi Hydra usa correttamente `full copy`. Per ottenere
copy-on-write, prepara esplicitamente un volume Linux reflink-capable, per
esempio XFS quando disponibile nel kernel WSL, e colloca sullo stesso volume
sia un nuovo clone del progetto sia la futura directory sorella delle Head
prima di eseguire `hydra init`.

Quando WSL ricade su `full copy`, `hydra init`, `hydra doctor storage` e
`hydra head create` mostrano il link alla guida
[WSL 2 Copy-on-Write Setup](wsl-copy-on-write.md):

```text
Storage backend: full copy
Copy-on-write guidance: https://github.com/leonardoLoddo/hydra/blob/main/Docs/user/wsl-copy-on-write.md
```

Da PowerShell verifica prima che la distribuzione sia WSL 2 con `wsl -l -v`.
La guida Microsoft
[Mount a Linux disk in WSL 2](https://learn.microsoft.com/windows/wsl/wsl2-mount-disk)
descrive come collegare dischi fisici e VHD. Creazione, partizionamento,
formattazione e mount possono distruggere dati se viene scelto il target
sbagliato: identifica il volume esatto, conserva i backup necessari e ottieni
l'autorizzazione amministrativa o aziendale prima di procedere. Hydra non
esegue queste operazioni.

Verifica prima la disponibilità del filesystem e poi il volume concreto:

```bash
grep -w xfs /proc/filesystems
cd /mnt/wsl/hydra-data/Shop
hydra init
hydra doctor storage
```

Considera attivo il CoW soltanto se il report mostra `Storage backend:
copy-on-write`, `Native primitive: Linux reflink` e il mount previsto. Puoi
controllare il volume di un percorso con `findmnt -T`. Hydra non crea, formatta
o monta il VHD. Non modificare il locator per spostare un progetto già
inizializzato: crea invece un clone revisionato sul nuovo volume e inizializza
quello, poiché la relocation assistita non è ancora disponibile. Se il fallback
continua, verifica anche che `.hydra.json` non imposti `storage.mode: "copy"`.

Su un volume Windows ReFS compatibile la riga nativa è `Native primitive:
Windows ReFS block clone`. Su NTFS il risultato normale è il fallback verificato
a copia completa. La capacità dipende dal volume reale delle Head, non dalla
lettera dell'unità o dal solo sistema operativo.

### Copy-on-write su Windows con un Dev Drive

Quando Windows nativo ricade su `full copy`, `hydra init`,
`hydra doctor storage` e `hydra head create` mostrano il link alla guida
[Windows Copy-on-Write Setup](windows-copy-on-write.md):

```text
Storage backend: full copy
Native primitive: unavailable
Copy-on-write guidance: https://github.com/leonardoLoddo/hydra/blob/main/Docs/user/windows-copy-on-write.md
```

Il fallback rimane sicuro: Hydra verifica i byte copiati e non usa hard link
mutabili. Per ridurre spazio iniziale e I/O serve invece un volume ReFS
compatibile, preferibilmente un Dev Drive di Windows 11.

Da **Impostazioni > Sistema > Archiviazione > Impostazioni di archiviazione
avanzate > Dischi e volumi**, seleziona **Crea unità di sviluppo**. Servono
permessi amministrativi e almeno 50 GB liberi. Un volume esistente non può
essere convertito in Dev Drive: Windows applica questa designazione durante la
formattazione di un nuovo volume. Un VHDX a espansione dinamica evita di
ripartizionare direttamente un disco fisico, ma ogni creazione, formattazione o
ridimensionamento richiede comunque di verificare la destinazione, conservare
i backup necessari e rispettare le policy aziendali.

Colloca sullo stesso Dev Drive un nuovo clone e la futura directory sorella
delle Head:

```text
D:\Projects\Shop
D:\Projects\Shop.heads
```

Questa disposizione consente il block clone anche per gli overlay. Una
sorgente su un altro volume può costringere singoli file al fallback e far
risultare l'intera Head come `full copy`. Non spostare un'installazione Hydra
esistente modificando locator o metadati locali: se il progetto è su NTFS,
crea e revisiona un nuovo clone sul Dev Drive e inizializza quello.

Da Git Bash:

```bash
cd /d/Projects/Shop
hydra init
hydra doctor storage
```

Considera disponibile il CoW soltanto quando la prova reale mostra:

```text
Storage backend: copy-on-write
Native primitive: Windows ReFS block clone
Fallback: full copy (verified)
Mutable hard links: disabled
Isolation: supported
```

Se Hydra continua a mostrare `full copy`, verifica che la directory delle Head
sia sul volume ReFS previsto, che progetto e Head siano sullo stesso volume e
che `.hydra.json` non imposti deliberatamente `storage.mode: "copy"`; quindi
ripeti `hydra doctor storage`. Hydra non crea, formatta, ridimensiona, monta o
contrassegna come attendibile un Dev Drive. I symlink tracciati o selezionati
dagli overlay rimangono non supportati su Windows nativo.

Per i file tracciati, quando il workspace coincide con il commit scelto Hydra
può riusare direttamente quei file come sorgenti copy-on-write. In presenza di
modifiche tracciate legge invece i contenuti dal commit Git tramite un unico
flusso batch; le modifiche locali non finiscono nella nuova Head.

Il backend effettivo rimane una decisione locale e Hydra lo registra per ogni
Head. La configurazione versionata può imporre la politica `copy`, ma non
memorizza le capacità del volume né il risultato di una materializzazione.

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
- `storage.mode` accetta `auto` oppure `copy`; `copy` forza il fallback sicuro
  per test e automazioni senza eliminare la conferma degli overlay;
- `overlay.copy` contiene regole Gitignore e direttive `...`.

Per `commands.open` e per la strategy `command` di `commands.close`, `program`
e ogni valore di `args` vengono passati separatamente al processo, senza
costruire una stringa di shell. Puoi usare:

- `{name}`;
- `{path}`;
- `{headRef}`;
- `{baseRef}`;
- `{targetRef}`.

I placeholder possono essere parte di un argomento, per esempio
`"--folder={path}"`. Graffe non riconosciute o non bilanciate nel template
vengono rifiutate. Le graffe che appartengono al valore espanso di un percorso
rimangono invece letterali. Il programma configurato è codice fidato del
progetto e non viene eseguito in una sandbox.

`commands.close.removeOnSuccess` è obbligatorio e decide se Hydra deve tentare
la rimozione protetta dopo un exit code zero. Non viene passato o espanso nel
programma.

Hydra rifiuta campi sconosciuti e campi appartenenti a una strategy diversa.
Versiona `.hydra.json`, ma non inserire percorsi assoluti o informazioni
specifiche della macchina.

Il file resta JSON standard, quindi non inserire commenti `//` o `/* ... */`.
Hydra non fornisce uno schema per editor: `$schema` non viene generato
e viene rifiutato come qualunque altro campo sconosciuto. La validazione
autorevole avviene durante la lettura della configurazione da parte di Hydra.

Le esclusioni guidate per symlink non sicuri vengono aggiunte come regole
negative `!/<percorso>` in fondo a `overlay.copy`. La posizione finale è
intenzionale perché la selezione usa la regola corrispondente più recente.

---

## 11. Stato locale: non modificarlo manualmente

Hydra separa:

```text
<git-common-dir>/hydra/project.json
<heads-directory>/.hydra/directory.json
<heads-directory>/.hydra/heads.json
<heads-directory>/.hydra/pending-<name>.json
<heads-directory>/.hydra/recovery-<name>.json
<git-private-worktree-directory>/hydra-head.json
```

- `project.json` individua l’installazione locale da qualunque worktree;
- `directory.json` prova l’ownership tramite `projectId` e `installationId` e,
  senza cambiarne il contenuto, fornisce il target stabile del guard OS;
- `heads.json` contiene l’inventario fisico delle Head locali;
- ogni `pending-<name>.json` conserva l'intento di una creazione non ancora
  pubblicata e viene normalmente rimosso al successo o durante il rollback;
- ogni coppia `recovery-<name>.json` e `hydra-head.json` conserva gli stessi
  metadati esatti della singola Head; una copia valida può consentire il
  recupero se l'altra viene persa, mentre due copie presenti devono coincidere.

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
metadati. Usa `hydra status` o `hydra repair` per ispezionare lo stato locale;
il repair corrente non reinizializza il progetto.

### Manca `heads.json`

Non ricrearlo a mano. Esegui `hydra repair`: per le Head con almeno un record
di recupero verificabile, Hydra può proporre la ricostruzione esatta
dell'inventario anche se il manifest privato è stato perso. Controlla l'elenco
e conferma soltanto se tutte le Head attese sono presenti. Se anche una
worktree Hydra non ha evidenza coerente, o i due record superstiti divergono,
Hydra non scrive uno stato parziale e richiede diagnosi manuale.

Se `heads.json` esiste ma è malformato, Hydra lo conserva e restituisce un
errore. Non cancellarlo per forzare il recupero: preservalo per la diagnosi.

### “configuration version 1 is not supported”

Il formato v1 era sperimentale e non viene migrato perché non è mai stato
distribuito. Ricrea il progetto o la fixture di sviluppo e inizializzalo con il
binario corrente.

### “unknown field `$schema`”

Una build di sviluppo precedente poteva aggiungere questa annotazione a
`.hydra.json`. Hydra non pubblica ancora lo schema indicato: rimuovi la sola
riga `$schema` dalla configurazione versionata e revisiona la modifica prima
di commetterla.

### “--target is required”

La ref passata a `--from` non identifica un branch locale. Specifica il branch
locale previsto per l’integrazione:

```bash
hydra head create experiment --from <COMMIT> --target main
```

### “normalizing the target ref”

Il valore passato a `--target` non identifica un branch locale esistente.
Controlla il nome con `git branch --list` e ripeti il comando con il target
corretto. Hydra non crea una Head parziale quando questa validazione fallisce.

### “directory ownership does not match”

Locator e marker non descrivono la stessa installazione. Non correggere gli ID
a mano e non riutilizzare implicitamente la directory. Il comando `repair`
corrente richiede un’installazione già validabile e non corregge l’identità.

### “cannot safely reconstruct configuration for existing Heads”

La directory appartiene all'installazione locale, ma contiene Head o residui
che dipendono da una policy condivisa non più disponibile. Hydra non rigenera
i default perché potrebbe cambiare branch prefix, overlay o comandi. Non
modificare i metadati locali: recupera la `.hydra.json` autorevole dal controllo
versione o da un backup, quindi usa `hydra status` e `hydra repair`.

### “does not match the versioned directory policy”

La configurazione condivisa e il locator locale risolvono directory diverse.
Questo può accadere dopo modifiche o spostamenti manuali, anche quando il nuovo
percorso configurato non esiste. Non creare la directory mancante e non
modificare il locator: ripristina la configurazione conosciuta senza cancellare
le Head. La relocation assistita non è ancora disponibile.

### Esiste `heads.json.lock`

Un’operazione Hydra potrebbe essere attiva oppure essersi interrotta. Non
eliminare il lock a mano. Esegui `hydra repair`: un lock del formato corrente
viene classificato come attivo quando il guard OS è occupato, oppure come
abbandonato quando il guard può essere riacquisito. Soltanto il secondo viene
proposto per la rimozione e richiede conferma esplicita.

Un lock vuoto, malformato o con una versione diversa da quella corrente causa
un errore di validazione e viene conservato per la diagnosi. Non modificarlo
per farlo apparire recuperabile: non è prevista una migrazione.

### “Head removal is incomplete”

Git ha già rimosso la worktree, ma Hydra non ha completato inventario o pulizia
del branch. Non cancellare manualmente il branch indicato: contiene il punto di
recupero. Esegui:

```bash
hydra repair
```

Se directory e registrazione Git sono assenti ma il branch esiste, Hydra
propone di rimuovere la sola voce stale e conserva il branch.

### “Unsafe overlay symlinks”

Hydra ha selezionato uno o più symlink che non possono essere ricreati dentro
la Head senza uscire dal progetto o dipendere da un percorso assoluto. Accetta
il prompt soltanto se quei link non servono alla Head. Hydra aggiornerà
`.hydra.json`; revisiona e committa la modifica. Se il link è necessario,
rispondi negativamente e sostituiscilo nel progetto con un symlink relativo che
risolva interamente dentro la root.

---

## 13. Regola di manutenzione

Questa guida deve cambiare nello stesso intervento che modifica:

- comandi, argomenti, opzioni, default o output;
- schema e validazione della configurazione;
- flusso base o avanzato;
- comportamento Git, filesystem, overlay o storage visibile all’utente;
- messaggi di errore che richiedono un’azione diversa.

Le pagine inglesi instradate da
[`hydra-user-guide.md`](hydra-user-guide.md) devono cambiare nello stesso
intervento e rimanere allineate a questa guida.

Le guide utente documentano soltanto comportamento dimostrato da codice, help
e test. Roadmap e funzionalità non implementate restano nella documentazione di
prodotto.

Per intenti di prodotto e dettagli tecnici:

- [contesto MVP](../product/hydra-mvp-context.md);
- [inizializzazione](../architecture/project-initialization.md);
- [creazione delle Head](../architecture/head-creation.md);
- [ispezione delle Head](../architecture/head-inspection.md);
- [completamento della shell](../architecture/shell-completions.md).
