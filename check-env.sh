#!/bin/bash
# Script to check the Telegram bot token

ENV_FILE="/opt/telegram-bot/.env"

# 1. Check that the .env file exists
if [ ! -f "$ENV_FILE" ]; then
  echo "❌ Errore: file .env non trovato in $ENV_FILE"
  exit 1
fi

# 2. Extract the TELOXIDE_TOKEN
TELOXIDE_TOKEN=$(grep -E '^TELOXIDE_TOKEN=' "$ENV_FILE" | cut -d '=' -f2-)

if [ -z "$TELOXIDE_TOKEN" ]; then
  echo "❌ Errore: TELOXIDE_TOKEN non trovato in $ENV_FILE"
  exit 1
fi

# 3. Check minimum format (number:id)
if [[ ! "$TELOXIDE_TOKEN" =~ ^[0-9]+:[A-Za-z0-9_-]+$ ]]; then
  echo "❌ Errore: TELOXIDE_TOKEN non valido"
  exit 1
fi

# Show only that it is valid, without printing the token
echo "✅ TELOXIDE_TOKEN presente e formato corretto"
exit 0
