#!/bin/bash
# Script per proteggere i file sensibili del bot Telegram
# Imposta proprietà e permessi sicuri per messages/ e data/

BOT_USER="telegrambot"
BOT_DIR="/opt/telegram-bot"

# 1. Controllo che la cartella esista
if [ ! -d "$BOT_DIR" ]; then
  echo "❌ Cartella $BOT_DIR non trovata"
  exit 1
fi

# 2. Imposta proprietà al bot
echo "🔹 Imposto proprietà di $BOT_USER su messages/ e data/..."
chown -R "$BOT_USER":"$BOT_USER" "$BOT_DIR/messages" "$BOT_DIR/data"

# 3. Imposta permessi sicuri
echo "🔹 Imposto permessi sicuri sui file JSON..."
chmod 640 "$BOT_DIR/messages/"*.json        # solo lettura bot
chmod 660 "$BOT_DIR/data/seen_welcome.json" # lettura/scrittura bot
chmod 750 "$BOT_DIR/data"                    # cartella accessibile solo a bot

# 4. Verifica finale
echo "🔹 Permessi correnti:"
ls -l "$BOT_DIR/messages"
ls -l "$BOT_DIR/data"

echo "✅ Protezione file sensibili applicata con successo"
exit 0
