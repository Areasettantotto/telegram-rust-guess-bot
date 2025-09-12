#!/bin/bash
# Script per controllare il token del bot Telegram (mascherato)

ENV_FILE="/opt/telegram-bot/.env"

# 1. Verifica che esista il file .env
if [ ! -f "$ENV_FILE" ]; then
  echo "❌ Errore: file .env non trovato in $ENV_FILE"
  exit 1
fi

# 2. Estrai il TELOXIDE_TOKEN
TELOXIDE_TOKEN=$(grep -E '^TELOXIDE_TOKEN=' "$ENV_FILE" | cut -d '=' -f2-)

if [ -z "$TELOXIDE_TOKEN" ]; then
  echo "❌ Errore: TELOXIDE_TOKEN non trovato in $ENV_FILE"
  exit 1
fi

# 3. Controlla formato minimo (numero:id)
if [[ ! "$TELOXIDE_TOKEN" =~ ^[0-9]+:[A-Za-z0-9_-]+$ ]]; then
  echo "❌ Errore: TELOXIDE_TOKEN non valido"
  exit 1
fi

# Mostra solo che è valido, senza stampare il token
echo "✅ TELOXIDE_TOKEN presente e formato corretto"
exit 0
