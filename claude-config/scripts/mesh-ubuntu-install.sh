#!/usr/bin/env bash
set -euo pipefail
# mesh-ubuntu-install.sh — Write Ubuntu ISO to USB and run post-install for mesh
# Usage: mesh-ubuntu-install.sh [burn|post-install|migrate-services]
# Run on the target machine (e.g. <your-hostname>)

ISO="$HOME/ubuntu-install/ubuntu-24.04.4-desktop-amd64.iso"
G='\033[0;32m' Y='\033[1;33m' R='\033[0;31m' B='\033[1m' N='\033[0m'

case "${1:-help}" in
burn)
  echo -e "${B}USB devices:${N}"
  lsblk -d -o NAME,SIZE,MODEL,TRAN | grep -i usb || { echo -e "${R}No USB found. Plug in a USB drive.${N}"; exit 1; }
  echo ""
  read -rp "Enter USB device name (e.g. sda): " USB_DEV
  USB="/dev/$USB_DEV"

  [[ -b "$USB" ]] || { echo -e "${R}$USB not a block device${N}"; exit 1; }
  [[ -f "$ISO" ]] || { echo -e "${R}ISO not found: $ISO${N}"; exit 1; }

  echo -e "${Y}WARNING: This will ERASE $USB ($(lsblk -d -n -o SIZE "$USB"))${N}"
  read -rp "Type YES to continue: " CONFIRM
  [[ "$CONFIRM" == "YES" ]] || { echo "Aborted."; exit 1; }

  echo -e "${B}Writing ISO to $USB...${N}"
  sudo dd if="$ISO" of="$USB" bs=4M status=progress oflag=sync
  sync
  echo -e "${G}Done. Remove USB, plug into target machine, boot from USB.${N}"
  echo ""
  echo -e "${B}During Ubuntu install:${N}"
  echo "  - Choose: Erase disk and install Ubuntu"
  echo "  - Hostname: <your-chosen-hostname>  (e.g. <your-hostname>)"
  echo "  - Username: <your-username>          (e.g. roberdan)"
  echo "  - Enable: Install third-party drivers"
  echo "  - After install: reboot, remove USB"
  echo ""
  echo -e "${B}After Ubuntu boots, run:${N}"
  echo "  mesh-ubuntu-install.sh post-install"
  ;;

post-install)
  echo -e "${B}Post-install: preparing for mesh...${N}"

  # 1. Essential packages
  echo -e "${B}[1/9] Installing packages...${N}"
  sudo apt-get update -qq
  sudo apt-get install -y -qq \
    git curl wget ssh openssh-server sqlite3 jq tmux unzip \
    build-essential pkg-config libssl-dev libsqlite3-dev \
    gnome-remote-desktop 2>&1 | tail -5
  echo -e "${G}Done${N}"

  # 2. Enable SSH
  echo -e "${B}[2/9] Enabling SSH...${N}"
  sudo systemctl enable --now ssh
  echo -e "${G}SSH active${N}"

  # 3. Enable RDP (GNOME Remote Desktop)
  echo -e "${B}[3/9] Enabling GNOME Remote Desktop (RDP)...${N}"
  gsettings set org.gnome.desktop.remote-desktop.rdp enable true 2>/dev/null || \
    echo -e "${Y}Enable RDP from Settings > Sharing > Remote Desktop${N}"
  echo -e "${G}RDP configured${N}"

  # 4. Install Node.js (for Claude Code)
  echo -e "${B}[4/9] Installing Node.js...${N}"
  if ! command -v node >/dev/null 2>&1; then
    curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
    sudo apt-get install -y -qq nodejs 2>&1 | tail -3
  fi
  echo -e "${G}Node $(node --version)${N}"

  # 5. Install Claude Code
  echo -e "${B}[5/9] Installing Claude Code...${N}"
  if ! command -v claude >/dev/null 2>&1; then
    sudo npm install -g @anthropic-ai/claude-code 2>&1 | tail -3
  fi
  echo -e "${G}Claude $(claude --version 2>/dev/null | head -1)${N}"

  # 6. Install GitHub CLI + Copilot
  echo -e "${B}[6/9] Installing GitHub CLI + Copilot...${N}"
  if ! command -v gh >/dev/null 2>&1; then
    curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | \
      sudo dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | \
      sudo tee /etc/apt/sources.list.d/github-cli.list > /dev/null
    sudo apt-get update -qq && sudo apt-get install -y -qq gh 2>&1 | tail -3
  fi
  gh extension install github/gh-copilot 2>/dev/null || true
  echo -e "${G}gh $(gh --version | head -1)${N}"

  # 7. Install Tailscale
  echo -e "${B}[7/9] Installing Tailscale...${N}"
  if ! command -v tailscale >/dev/null 2>&1; then
    curl -fsSL https://tailscale.com/install.sh | sh 2>&1 | tail -3
  fi
  echo -e "${G}Tailscale installed${N}"

  # 8. Install Docker
  echo -e "${B}[8/9] Installing Docker...${N}"
  if ! command -v docker >/dev/null 2>&1; then
    sudo apt-get install -y -qq ca-certificates gnupg
    sudo install -m 0755 -d /etc/apt/keyrings
    curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg 2>/dev/null
    sudo chmod a+r /etc/apt/keyrings/docker.gpg
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu $(. /etc/os-release && echo "$VERSION_CODENAME") stable" | \
      sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
    sudo apt-get update -qq && sudo apt-get install -y -qq docker-ce docker-ce-cli containerd.io docker-compose-plugin 2>&1 | tail -3
    sudo usermod -aG docker "$USER"
    echo -e "${Y}Log out and back in for docker group to take effect${N}"
  fi
  echo -e "${G}Docker $(docker --version 2>/dev/null || echo 'installed')${N}"

  # 9. Install Rust toolchain
  echo -e "${B}[9/9] Installing Rust toolchain...${N}"
  if ! command -v rustc >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y 2>&1 | tail -3
    source "$HOME/.cargo/env"
  fi
  echo -e "${G}Rust $(rustc --version 2>/dev/null)${N}"

  echo ""
  echo -e "${B}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${N}"
  echo -e "${B}Next steps:${N}"
  echo ""
  echo "  1. Tailscale: sudo tailscale up  (on this machine)"
  echo "  2. SSH key:   ssh-copy-id roberdan@<new-ip>"
  echo "  3. Provision: mesh-provision-node.sh <hostname>"
  echo "  4. Auth:      mesh-auth-sync.sh --peer <hostname>"
  echo "  5. gh login:  ssh <hostname> 'gh auth login'"
  echo "  6. Verify:    mesh-preflight.sh --peer <hostname>"
  echo ""
  echo -e "${B}To migrate services from old install:${N}"
  echo "  mesh-ubuntu-install.sh migrate-services"
  echo ""
  echo -e "${B}RDP: connect from Mac Finder with Cmd+K > rdp://<ip>${N}"
  echo -e "${B}Or install Microsoft Remote Desktop from App Store${N}"
  echo -e "${B}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${N}"
  ;;

migrate-services)
  SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  exec "$SCRIPT_DIR/mesh-migrate-services.sh"
  ;;

*)
  echo "Usage: mesh-ubuntu-install.sh [burn|post-install|migrate-services]"
  echo ""
  echo "  burn              - Write Ubuntu ISO to USB drive"
  echo "  post-install      - Run after Ubuntu is installed (installs all mesh tools)"
  echo "  migrate-services  - Recreate Docker, nightly guardians, elephant from old install"
  ;;
esac
