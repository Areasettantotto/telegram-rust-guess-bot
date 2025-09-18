#!/usr/bin/env bash
# Minimal server deploy script executed from your workstation.
# Usage: ./deploy_remote.sh user@host [branch]
# Example: ./deploy_remote.sh root@funny-hoover master

set -euo pipefail

if [ "$#" -lt 1 ]; then
  echo "Usage: $0 user@host [branch]"
  exit 2
fi

REMOTE="$1"
BRANCH="${2:-master}"
REPO_DIR="/opt/telegram-bot"
SERVICE="telegram-bot.service"
SSH_OPTS="-o BatchMode=yes -o ConnectTimeout=10 -o StrictHostKeyChecking=accept-new"

echo "Deploy to ${REMOTE} branch=${BRANCH} ..."

ssh ${SSH_OPTS} "${REMOTE}" bash -se -- "${BRANCH}" <<'REMOTE_EOF'
set -euo pipefail

# Branch is passed as $1 from the local shell invocation to avoid here-doc expansion issues
BRANCH_REMOTE="$1"

REPO_DIR="/opt/telegram-bot"

echo "-> Switch to repo dir ${REPO_DIR}"
if [ ! -d "$REPO_DIR" ]; then
  echo "ERROR: ${REPO_DIR} not found on remote"
  exit 3
fi

cd "$REPO_DIR"

echo "-> Ensure git fetch/pull as telegrambot"
sudo -u telegrambot -H git fetch origin || { echo "git fetch failed"; exit 4; }
# Prefer switch, fall back to checkout; quote branch properly
sudo -u telegrambot -H git switch -- "$BRANCH_REMOTE" 2>/dev/null || sudo -u telegrambot -H git checkout -- "$BRANCH_REMOTE"
sudo -u telegrambot -H git pull origin -- "$BRANCH_REMOTE" || { echo "git pull failed"; exit 5; }

echo "-> Validate message files"
if [ -x "./check-messages.sh" ]; then
  sudo -u telegrambot -H bash -lc "./check-messages.sh" || { echo "check-messages.sh failed"; exit 6; }
else
  echo "WARNING: check-messages.sh not found or not executable; skipping validation"
fi

echo "-> Build release (as telegrambot)"
sudo -u telegrambot -H bash -lc "cd ${REPO_DIR} && cargo build --release" || { echo "cargo build failed"; exit 7; }

echo "-> Fix ownership and secure data"
sudo chown -R telegrambot:telegrambot "${REPO_DIR}"
if [ -x "./secure-data.sh" ]; then
  sudo -u telegrambot -H bash -lc "./secure-data.sh" || { echo "secure-data.sh failed"; exit 8; }
else
  echo "WARNING: secure-data.sh not found or not executable; skipping"
fi

echo "-> Ensure data directory and seen_welcome.json perms"
if [ -d "${REPO_DIR}/data" ]; then
  sudo chmod -R g+rwX "${REPO_DIR}/data" || true
  if [ -f "${REPO_DIR}/data/seen_welcome.json" ]; then
    sudo chmod 640 "${REPO_DIR}/data/seen_welcome.json" || true
  fi
else
  echo "WARNING: ${REPO_DIR}/data not found; skipping data perms"
fi

echo "-> Reload systemd and restart service"
sudo systemctl daemon-reload
sudo systemctl restart ${SERVICE} || { echo "service restart failed"; exit 9; }

echo "-> Service status (short)"
sudo systemctl status ${SERVICE} --no-pager --lines=10 || true

echo "-> Last journal entries for service"
sudo journalctl -u ${SERVICE} -n 200 --no-pager || true

REMOTE_EOF

echo "Deploy finished."
