#!/bin/bash
# Script to protect sensitive files of the Telegram bot
# Set ownership and secure permissions for messages/ and data/

BOT_USER="telegrambot"
BOT_DIR="/opt/telegram-bot"

# 1. Check that the directory exists
if [ ! -d "$BOT_DIR" ]; then
  echo "❌ Cartella $BOT_DIR non trovata"
  exit 1
fi

# 2. Set ownership to the bot
echo "🔹 Imposto proprietà di $BOT_USER su messages/ e data/..."
chown -R "$BOT_USER":"$BOT_USER" "$BOT_DIR/messages" "$BOT_DIR/data"

# 3. Set secure permissions
echo "🔹 Imposto permessi sicuri sui file JSON..."
chmod 640 "$BOT_DIR/messages/"*.json          # read-only for bot
chmod 660 "$BOT_DIR/data/seen_welcome.json"   # read/write for bot
chmod 750 "$BOT_DIR/data"                     # directory accessible only to the bot

# 4. Verifica finale
echo "🔹 Permessi correnti:"
ls -l "$BOT_DIR/messages"
ls -l "$BOT_DIR/data"

echo "✅ Protezione file sensibili applicata con successo"
exit 0
