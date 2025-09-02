#!/bin/bash
# Script rapido per aggiornare e riavviare il bot Telegram

set -euo pipefail

# Vai nella cartella del bot
cd /opt/telegram-bot || { echo "❌ Cartella /opt/telegram-bot non trovata"; exit 1; }

# Aggiorna il codice dal repository
echo "⬇️  Pulling latest changes from GitHub..."
git pull origin master || { echo "❌ Errore in git pull"; exit 1; }

# Compila il bot in modalità release
echo "⚙️  Building bot..."
cargo build --release || { echo "❌ Errore nella compilazione"; exit 1; }

# Riavvia il servizio systemd
echo "🔄 Restarting systemd service..."
systemctl restart telegram-bot || { echo "❌ Errore nel riavvio del servizio"; exit 1; }

# Mostra lo stato del servizio
echo "📡 Bot status:"
if systemctl is-active --quiet telegram-bot; then
    echo "✅ Bot attivo e funzionante"
else
    echo "❌ Bot fermo, controlla con: journalctl -u telegram-bot -n 50 --no-pager"
fi
