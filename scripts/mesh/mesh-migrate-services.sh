#!/usr/bin/env bash
set -euo pipefail
# mesh-migrate-services.sh — Recreate Docker, nightly guardians, elephant on Ubuntu
# Usage: mesh-migrate-services.sh
# Prerequisite: mesh-ubuntu-install.sh post-install + mesh-provision-node.sh already run

G='\033[0;32m' Y='\033[1;33m' R='\033[0;31m' B='\033[1m' N='\033[0m'

echo -e "${B}Migrating linux-worker services to Ubuntu...${N}"
echo -e "${Y}Prerequisite: mesh-provision-node.sh already run${N}"

SYSTEMD_USER="$HOME/.config/systemd/user"
mkdir -p "$SYSTEMD_USER"

# 1. Docker containers (postgres + pgvector)
echo -e "${B}[1/4] Recreating Docker containers...${N}"
if command -v docker >/dev/null 2>&1; then
  docker run -d --name mirrorbuddy-db --restart unless-stopped \
    -e POSTGRES_PASSWORD=mirrorbuddy -e POSTGRES_DB=mirrorbuddy \
    -p 5432:5432 -v mirrorbuddy_data:/var/lib/postgresql/data \
    postgres:16-alpine 2>/dev/null && echo -e "${G}mirrorbuddy-db created${N}" || echo -e "${Y}mirrorbuddy-db already exists${N}"

  docker run -d --name virtualbpm-db --restart unless-stopped \
    -e POSTGRES_PASSWORD=virtualbpm -e POSTGRES_DB=virtualbpm \
    -p 5433:5432 -v virtualbpm_data:/var/lib/postgresql/data \
    pgvector/pgvector:pg16 2>/dev/null && echo -e "${G}virtualbpm-db created${N}" || echo -e "${Y}virtualbpm-db already exists${N}"
else
  echo -e "${R}Docker not installed — run post-install first${N}"
fi

# 2. Nightly guardian services
echo -e "${B}[2/4] Installing nightly guardian services...${N}"

cat > "$SYSTEMD_USER/mirrorbuddy-nightly-guardian.service" << 'EOF'
[Unit]
Description=MirrorBuddy Nightly Guardian
After=network-online.target docker.service

[Service]
Type=oneshot
ExecStart=/bin/bash -c 'source $HOME/.cargo/env; $HOME/.claude/scripts/nightly-guardian.sh mirrorbuddy'
Environment=HOME=%h CLAUDE_HOME=%h/.claude PATH=/usr/local/bin:/usr/bin:/bin
EOF

cat > "$SYSTEMD_USER/mirrorbuddy-nightly-guardian.timer" << 'EOF'
[Unit]
Description=MirrorBuddy Nightly Guardian Timer

[Timer]
OnCalendar=*-*-* 03:00:00
Persistent=true

[Install]
WantedBy=timers.target
EOF

cat > "$SYSTEMD_USER/virtualbpm-nightly-guardian.timer" << 'EOF'
[Unit]
Description=VirtualBPM Nightly Guardian Timer

[Timer]
OnCalendar=*-*-* 04:00:00
Persistent=true

[Install]
WantedBy=timers.target
EOF

cat > "$SYSTEMD_USER/virtualbpm-nightly-guardian.service" << 'EOF'
[Unit]
Description=VirtualBPM Nightly Guardian
After=network-online.target docker.service

[Service]
Type=oneshot
ExecStart=/bin/bash -c 'source $HOME/.cargo/env; $HOME/.claude/scripts/nightly-guardian.sh virtualbpm'
Environment=HOME=%h CLAUDE_HOME=%h/.claude PATH=/usr/local/bin:/usr/bin:/bin
EOF

systemctl --user daemon-reload
systemctl --user enable mirrorbuddy-nightly-guardian.timer
systemctl --user enable virtualbpm-nightly-guardian.timer
systemctl --user start mirrorbuddy-nightly-guardian.timer
systemctl --user start virtualbpm-nightly-guardian.timer
echo -e "${G}Nightly guardians enabled${N}"

# 3. Elephant service
echo -e "${B}[3/4] Installing elephant service...${N}"
cat > "$SYSTEMD_USER/elephant.service" << 'EOF'
[Unit]
Description=Elephant Memory Service
After=network-online.target

[Service]
ExecStart=/bin/bash -c 'source $HOME/.cargo/env; $HOME/GitHub/ConvergioPlatform/daemon/target/release/convergio-platform-daemon elephant start'
Restart=always
RestartSec=10
Environment=HOME=%h CLAUDE_HOME=%h/.claude PATH=/usr/local/bin:/usr/bin:/bin

[Install]
WantedBy=default.target
EOF

systemctl --user daemon-reload
systemctl --user enable elephant
systemctl --user restart elephant
echo -e "${G}Elephant service active${N}"

# 4. Enable lingering (services run without login)
echo -e "${B}[4/4] Enabling user lingering...${N}"
sudo loginctl enable-linger "$USER"
echo -e "${G}Lingering enabled — services run at boot${N}"

echo ""
echo -e "${B}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${N}"
echo -e "${G}Service migration complete.${N}"
echo ""
echo "  Check timers:  systemctl --user list-timers"
echo "  Check daemons: systemctl --user status elephant claude-mesh-daemon mesh-heartbeat"
echo "  Check docker:  docker ps"
echo -e "${B}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${N}"
