#!/bin/bash
# /opt/telegram-bot/check-messages.sh
# Controllo sicurezza file messages/*.json (dimensione e chiavi obbligatorie)

MSG_DIR="/opt/telegram-bot/messages"
MAX_SIZE=65536  # 64 KiB
REQUIRED_KEYS=("welcome_prompt" "cannot_start")

echo "🔹 Controllo file JSON in $MSG_DIR ..."

for f in "$MSG_DIR"/*.json; do
    if [ ! -f "$f" ]; then
        continue
    fi

    # Controllo dimensione
    size=$(stat -c%s "$f")
    if [ "$size" -gt "$MAX_SIZE" ]; then
        echo "❌ $f troppo grande ($size bytes)"
        exit 1
    fi

    # Controllo chiavi obbligatorie
    for key in "${REQUIRED_KEYS[@]}"; do
        if ! jq -e --arg key "$key" 'has($key)' "$f" >/dev/null; then
            echo "❌ $f manca chiave obbligatoria: $key"
            exit 1
        fi
    done
done

echo "✅ Tutti i file messages/*.json sono validi"
exit 0
