# Siri Integration for Convergio

Convergio si integra con Siri tramite Shortcuts (app Comandi Rapidi di macOS/iOS).

## Setup (una volta)

1. Apri l'app **Comandi Rapidi** su Mac
2. Importa gli shortcut dalla cartella `scripts/siri/`
3. Ogni shortcut diventa un comando Siri: "Hey Siri, Convergio stato"

## Shortcut disponibili

| Comando Siri | Cosa fa |
|---|---|
| "Convergio stato" | Mostra piani attivi e task rimasti |
| "Convergio costi" | Quanto hai speso in totale |
| "Convergio kernel" | Stato del kernel (modelli, uptime) |
| "Convergio nodo" | Readiness check del nodo |

## Come creare manualmente uno Shortcut

1. Apri **Comandi Rapidi**
2. Crea nuovo → nome "Convergio stato"
3. Aggiungi azione **Ottieni contenuto dell'URL**: `http://localhost:8420/api/plan-db/list`
4. Aggiungi azione **Ottieni valore dal dizionario**: chiave `plans`
5. Aggiungi azione **Conta**: conta gli elementi con status "doing"
6. Aggiungi azione **Mostra risultato** / **Leggi testo ad alta voce**

Oppure usa lo script shell come azione:

1. Aggiungi azione **Esegui script shell**
2. Script:
```bash
RESULT=$(curl -sf http://localhost:8420/api/plan-db/list | python3 -c "
import json,sys
d=json.loads(sys.stdin.read())
doing=[p for p in d.get('plans',[]) if p.get('status')=='doing']
tasks=sum(p.get('tasks_total',0)-p.get('tasks_done',0) for p in doing)
print(f'Hai {len(doing)} piani attivi con {tasks} task rimasti.')
")
echo "$RESULT"
```
3. Attiva "Leggi testo ad alta voce" nell'output

## Da iPhone (fuori casa)

Gli Shortcut si sincronizzano via iCloud. Ma `localhost:8420` non e' raggiungibile da iPhone.
Per usare da remoto: il bot Telegram @ConvergioBot funziona ovunque.
