# === Homebrew PATH ===
export PATH="/opt/homebrew/bin:$PATH"

# === Warp: usa il prompt shell (PS1) invece di quello built-in ===
export WARP_HONOR_PS1=1

# === ZSH Performance Options ===
HISTSIZE=5000
SAVEHIST=5000
HISTFILE=~/.zsh_history
setopt SHARE_HISTORY HIST_IGNORE_ALL_DUPS HIST_REDUCE_BLANKS HIST_VERIFY HIST_IGNORE_SPACE
setopt AUTO_MENU COMPLETE_IN_WORD ALWAYS_TO_END AUTO_CD

# === Native ZSH Completions (faster than oh-my-zsh) ===
autoload -Uz compinit
# Only regenerate compinit once per day for speed
if [[ -n ~/.zcompdump(#qN.mh+24) ]]; then
  compinit
else
  compinit -C
fi

# Completion styling
zstyle ':completion:*' menu select
zstyle ':completion:*' matcher-list 'm:{a-zA-Z}={A-Za-z}'
zstyle ':completion:*:git-checkout:*' sort false
zstyle ':completion:*:descriptions' format '[%d]'
zstyle ':completion:*' list-colors ${(s.:.)LS_COLORS}

# Docker completions
fpath=(/Users/roberdan/.docker/completions $fpath)

# === PATH setup (consolidated) ===
typeset -U path
path=(
  $HOME/.local/bin
  $HOME/bin
  /opt/homebrew/opt/python@3.11/bin
  /opt/homebrew/opt/python@3.11/libexec/bin
  /usr/local/bin
  /usr/bin
  /bin
  $HOME/.rbenv/bin
  /opt/homebrew/Cellar/mlx/0.20.0/bin
  $HOME/GitHub/MyAIAgents
  "$HOME/Library/Application Support/reflex/bun/bin"
  $path
)
export PATH

# === Environment Variables ===
export BUN_INSTALL="$HOME/Library/Application Support/reflex/bun"
export DOTNET_ROOT="/opt/homebrew/Cellar/dotnet/9.0.8/libexec"
export PGDATA="/opt/homebrew/var/postgresql17"
export PGHOST="localhost"
export PGPORT="5432"
export PGUSER="$USER"
export LC_ALL="en_US.UTF-8"
export LANG="en_US.UTF-8"
export GO111MODULE=on
export CLICOLOR=1
export LSCOLORS=ExGxBxDxCxEgEdxbxgxcxd

# AI API Keys (loaded from secure file)
export QWEN_API_BASE=http://localhost:11434/v1
[[ -f ~/.config/secrets/api-keys.zsh ]] && source ~/.config/secrets/api-keys.zsh

# Ollama optimizations for M5 Max
export OLLAMA_NUM_PARALLEL=4
export OLLAMA_MAX_LOADED_MODELS=2
export OLLAMA_FLASH_ATTENTION=1
export OLLAMA_KV_CACHE_TYPE="q8_0"
export OLLAMA_NUM_GPU=1

# === Aliases ===
# Navigation
alias cdMirrorHR='cd /Users/roberdan/GitHub/MirrorHR'
alias cdMirrorHRCloud='cd /Users/roberdan/GitHub/research-cloud-api/research-cloud-api/reader-research-cloud-api'
alias cdConvergio='cd /Users/roberdan/GitHub/convergio'
alias cdNovoHack='cd /Users/roberdan/Library/CloudStorage/OneDrive-Microsoft/FY26/Customers/\!Novo\ Nordisk/hackathon/AIPP-Hack'
alias cls='clear'
alias dir='yazi'

# Config management
alias editzsh="zed ~/.zshrc"
alias reloadzsh="source ~/.zshrc"
alias reload='source ~/.zshrc'
alias editz='windsurf ~/.zshrc'
alias myalias="grep '^alias' ~/.zshrc"

# Git (custom + common oh-my-zsh aliases)
alias status='git status'
alias push='git push'
alias fetch='git fetch'
alias pull='git pull'
alias commit='git commit -m'
alias ga='git add'
alias gaa='git add --all'
alias gst='git status'
alias gco='git checkout'
alias gcb='git checkout -b'
alias gcm='git commit -m'
alias gp='git push'
alias gl='git pull'
alias gf='git fetch'
alias gd='git diff'
alias gds='git diff --staged'
alias gb='git branch'
alias gba='git branch -a'
alias glog='git log --oneline --graph --decorate'
alias gsta='git stash'
alias gstp='git stash pop'

# Audio
alias setrode='SwitchAudioSource -t output -s "RODECaster Duo Chat" && SwitchAudioSource -t input -s "RODECaster Duo Main Multitrack"'
alias setTeams='SwitchAudioSource -t output -s "AirPods Pro 2 di Roberto" && SwitchAudioSource -t input -s "RODECaster Duo Main Multitrack"'
alias showAudioAlias='alias | grep -E "set(Rode|StudioDisplay|Bose|Teams)"'

# Tools
alias codegraph='./.codegraph/codegraph'
alias cg-reindex='./.codegraph/codegraph index . -f'
alias wildClaude='claude --dangerously-skip-permissions'
alias x='exit'
alias quit='exit'
alias q='exit'
alias bye='exit'
alias tree='eza --tree'
alias top="btop"
alias now='date +"%T"'
alias today='date +"%A, %B %d, %Y"'
alias wttr='curl wttr.in'
alias cal='cal -3'
alias stopwatch='time read -p "Press enter to stop..."'

# PostgreSQL
alias pgstart='brew services start postgresql@17'
alias pgstop='brew services stop postgresql@17'
alias pgrestart='brew services restart postgresql@17'
alias pgstatus='brew services list | grep postgresql'
alias pglog='tail -f /opt/homebrew/var/postgresql17/server.log'
alias psql='psql -h localhost -p 5432 -U $USER'
alias pgbackup='pg_dump -h localhost -p 5432 -U $USER'
alias pgrestore='pg_restore -h localhost -p 5432 -U $USER'

# Python
alias pycheck='echo "python3: $(python3 --version)"; echo "pip3: $(pip3 --version | grep -oE "python [0-9.]+")"'

# Other tools
alias virtualBPM='cd ~/GitHub/VirtualBPM && claude --dangerously-skip-permissions'
alias virtual-bpm='cd ~/GitHub/VirtualBPM && claude --dangerously-skip-permissions'
alias virtualBPM-renew='bash ~/GitHub/VirtualBPM/.claude/renew-token.sh'
alias virtualBPM-check='bash ~/GitHub/VirtualBPM/.claude/renew-token.sh check'
alias vbpm='bash ~/GitHub/VirtualBPM/scripts/webapp.sh'

# === Functions (sourced from external file) ===
[[ -f ~/.config/zsh/functions.zsh ]] && source ~/.config/zsh/functions.zsh

# === Lazy loaders (for speed) ===
rbenv() {
  unfunction rbenv 2>/dev/null
  if command -v rbenv >/dev/null 2>&1; then
    eval "$(command rbenv init - zsh)"
    rbenv "$@"
  fi
}

# Node.js via nvm - add default to PATH for immediate availability
export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
[ -d "$NVM_DIR/versions/node" ] && export PATH="$NVM_DIR/versions/node/$(cat $NVM_DIR/alias/default 2>/dev/null || echo v22.21.1)/bin:$PATH"

nvm() {
  unfunction nvm 2>/dev/null
  [ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh" && nvm "$@"
}


# === Key Bindings ===
bindkey '^[[1;5C' forward-word
bindkey '^[[1;5D' backward-word
bindkey '^[[H' beginning-of-line
bindkey '^[[F' end-of-line
bindkey '^[[3~' delete-char

# === Editor ===
export EDITOR="zed --wait"
export VISUAL="zed --wait"

# === External sources ===
[[ -f ~/.config/openai/credentials ]] && source ~/.config/openai/credentials
[[ -f "$HOME/GitHub/VirtualBPM/.env" ]] && { set -a; source "$HOME/GitHub/VirtualBPM/.env"; set +a; }
[[ "$TERM_PROGRAM" == "vscode" ]] && . "$(code --locate-shell-integration-path zsh)" 2>/dev/null
[[ -f "$HOME/.local/bin/env" ]] && . "$HOME/.local/bin/env"

# Azure OpenAI Realtime loaded from ~/.config/secrets/api-keys.zsh

# === Fast CLI Tools (sourced from Claude config) ===
[[ -f ~/.claude/shell-aliases.sh ]] && source ~/.claude/shell-aliases.sh

# === Zoxide (smart cd) ===
command -v zoxide &>/dev/null && eval "$(zoxide init zsh)"

# Added by LM Studio CLI (lms)
export PATH="$PATH:/Users/roberdan/.lmstudio/bin"
# End of LM Studio CLI section


# Azure DevOps PAT loaded from ~/.config/secrets/api-keys.zsh

# === Kitty Tab Title: folder + git branch/worktree ===
_kitty_tab_title() {
  local dir="${PWD##*/}"
  local branch=""
  if git rev-parse --is-inside-work-tree &>/dev/null 2>&1; then
    local repo_root="$(git rev-parse --show-toplevel 2>/dev/null)"
    dir="${repo_root##*/}"
    branch="$(git symbolic-ref --short HEAD 2>/dev/null || git rev-parse --short HEAD 2>/dev/null)"
    # Mark worktrees
    if [[ "$(git rev-parse --git-dir 2>/dev/null)" == *".git/worktrees/"* ]]; then
      branch="wt:${branch}"
    fi
  fi
  local title="${dir}"
  [[ -n "$branch" ]] && title="${dir} [${branch}]"
  printf '\e]1;%s\a' "$title"
}
autoload -Uz add-zsh-hook
add-zsh-hook precmd _kitty_tab_title
add-zsh-hook chpwd _kitty_tab_title

# Hide zsh PROMPT_EOL_MARK (the % at line start)
PROMPT_EOL_MARK=''

# Use trash instead of rm (safer - files go to macOS Trash)
export PATH="/opt/homebrew/opt/trash/bin:$PATH"
alias rm="trash"

# === MAC-DEV REMOTE CONTROL (Mario Mac M1 Pro) ===
export MAC_DEV_USER="mariodan"

_macdev_host() {
  # Tailscale direct — local blocked by firewall
  if ssh -o ConnectTimeout=3 -o BatchMode=yes mac-dev-ts true 2>/dev/null; then
    echo "mac-dev-ts"
  else
    echo "UNREACHABLE"; return 1
  fi
}

# tlm — connect to Mac M1 tmux (like tlx for Linux)
tlm() {
  local host=$(_macdev_host) || { echo "Mac M1 unreachable (local + Tailscale)"; return 1; }
  echo "Connecting via $host..."
  if [ -n "$KITTY_PID" ]; then
    kitten ssh "$host" -t "/opt/homebrew/bin/tmux new-session -A -s Convergio -c /Users/Shared/GitHub"
  else
    ssh "$host" -t "/opt/homebrew/bin/tmux new-session -A -s Convergio -c /Users/Shared/GitHub"
  fi
}

# Quick commands on Mac M1
mac() {
  local host=$(_macdev_host) || { echo "Mac M1 unreachable"; return 1; }
  echo "Connecting via $host..."
  ssh "$host"
}

mac-claude() {
  local host=$(_macdev_host) || { echo "Mac M1 unreachable"; return 1; }
  ssh "$host" -t "cd /Users/Shared/GitHub/VirtualBPM && claude"
}

mac-run() {
  local host=$(_macdev_host) || { echo "Mac M1 unreachable"; return 1; }
  ssh "$host" "$*"
}

# IDE remote
code-mac() { code --remote ssh-remote+mac-dev-ts "${1:-/Users/Shared/GitHub/VirtualBPM}"; }
cursor-mac() { cursor --remote ssh-remote+mac-dev-ts "${1:-/Users/Shared/GitHub/VirtualBPM}"; }

# Dev Mesh Status — all 3 machines at a glance
mesh-status() {
  local G='\033[0;32m' R='\033[0;31m' Y='\033[1;33m' N='\033[0m' B='\033[1m'
  echo -e "${B}🔗 Dev Mesh Status${N}"
  echo -e "${B}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${N}"

  # This machine
  echo -e "  ${G}●${N} ${B}Mac M5 Max (this)${N} — $(sysctl -n hw.ncpu) cores, $(($(sysctl -n hw.memsize) / 1073741824))GB RAM"

  # Mac M1
  local mhost=$(_macdev_host 2>/dev/null)
  if [[ "$mhost" != "UNREACHABLE" && -n "$mhost" ]]; then
    local minfo=$(ssh -o ConnectTimeout=3 "$mhost" "sysctl -n hw.ncpu; sysctl -n hw.memsize" 2>/dev/null)
    local mcpu=$(echo "$minfo" | head -1); local mmem=$(echo "$minfo" | tail -1)
    echo -e "  ${G}●${N} ${B}Mac M1 Pro${N} ($mhost) — ${mcpu} cores, $((mmem / 1073741824))GB RAM"
  else
    echo -e "  ${R}●${N} ${B}Mac M1 Pro${N} — offline"
  fi

  # Linux
  local lhost=$(_omarchy_host 2>/dev/null)
  if [[ "$lhost" != "UNREACHABLE" && -n "$lhost" ]]; then
    local linfo=$(ssh -o ConnectTimeout=3 "$lhost" "nproc; free -b | grep Mem | awk '{print \$2}'" 2>/dev/null)
    local lcpu=$(echo "$linfo" | head -1); local lmem=$(echo "$linfo" | tail -1)
    echo -e "  ${G}●${N} ${B}Linux omarchy${N} ($lhost) — ${lcpu} cores, $((lmem / 1073741824))GB RAM"
  else
    echo -e "  ${R}●${N} ${B}Linux omarchy${N} — offline"
  fi
  echo -e "${B}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${N}"
}

# === LINUX-DEV REMOTE CONTROL ===
export LINUX_DEV="linux-dev"
export LINUX_USER="roberdan"

# SSH shortcuts
alias lx='ssh linux-dev'
alias lxt='ssh -t linux-dev "~/bin/tmux-mb.sh"'
alias lxdev='ssh -t linux-dev "cd /home/roberdan/GitHub/MirrorBuddy && tmux attach -t mb 2>/dev/null || mbdev"'

# Remote commands
lxe() { ssh linux-dev "$*"; }
lxbuild() { ssh linux-dev "cd /home/roberdan/GitHub/MirrorBuddy && npm run build"; }
lxlint() { ssh linux-dev "cd /home/roberdan/GitHub/MirrorBuddy && npm run lint"; }
lxtest() { ssh linux-dev "cd /home/roberdan/GitHub/MirrorBuddy && npm run test"; }

# Port forwarding
lxport() { ssh -N -L ${1:-3000}:localhost:${1:-3000} linux-dev; }
lxports() {
    echo "Forwarding 3000, 5432, 6379..."
    ssh -N -L 3000:localhost:3000 -L 5432:localhost:5432 -L 6379:localhost:6379 linux-dev
}

# Monitoring
lxstatus() {
    echo "=== linux-dev ===" && ssh linux-dev "uptime; echo ''; free -h | head -2; echo ''; df -h / | tail -1"
}
lxmon() { ssh -t linux-dev "btop || htop || top"; }

# IDE remote (VS Code, Cursor, Zed)
code-linux() { code --remote ssh-remote+linux-dev "${1:-/home/roberdan/GitHub/MirrorBuddy}"; }
cursor-linux() { cursor --remote ssh-remote+linux-dev "${1:-/home/roberdan/GitHub/MirrorBuddy}"; }
zed-linux() { zed "ssh://linux-dev/home/roberdan/GitHub/MirrorBuddy"; }

# Sync code (excludes .env, node_modules, .next, .git)
mbpush() {
    rsync -avz --progress --exclude='.git' --exclude='node_modules' --exclude='.next' --exclude='.env*' \
        "${1:-.}" "linux-dev:/home/roberdan/GitHub/MirrorBuddy/"
}
mbpull() {
    rsync -avz --progress --exclude='.git' --exclude='node_modules' --exclude='.next' --exclude='.env*' \
        "linux-dev:/home/roberdan/GitHub/MirrorBuddy/" "${1:-.}"
}

# Sync .env files (Mac → Linux)
mbenv-push() {
    echo "Syncing .env files Mac → Linux..."
    rsync -avz --progress ~/GitHub/MirrorBuddy/.env* linux-dev:/home/roberdan/GitHub/MirrorBuddy/
}

# Sync .env files (Linux → Mac)
mbenv-pull() {
    echo "Syncing .env files Linux → Mac..."
    rsync -avz --progress linux-dev:/home/roberdan/GitHub/MirrorBuddy/.env* ~/GitHub/MirrorBuddy/
}

# Full sync (code + env)
mbsync() {
    echo "=== Full sync Mac → Linux ==="
    mbpush && mbenv-push
}

# Check if .env files are in sync
mbenv-diff() {
    echo "=== .env differences ==="
    for f in .env .env.production .env.vercel .env.vercel.local; do
        if ssh linux-dev "test -f /home/roberdan/GitHub/MirrorBuddy/$f" 2>/dev/null; then
            diff <(cat ~/GitHub/MirrorBuddy/$f 2>/dev/null) <(ssh linux-dev "cat /home/roberdan/GitHub/MirrorBuddy/$f" 2>/dev/null) && echo "$f: ✓ in sync" || echo "$f: ✗ DIFFERENT"
        else
            echo "$f: ✗ missing on Linux"
        fi
    done
}

# =============================================================================
# Linux Dev Remote Session
# =============================================================================
alias ld="~/.local/scripts/linux-dev.sh"           # Connect to tmux
alias lds="~/.local/scripts/linux-dev.sh status"   # Check session status
alias ldr="~/.local/scripts/linux-dev.sh run"      # Run command remotely

# ntfy.sh notifications - subscribe to your topic
# Topic: mirrorbuddy-dev-roberdan
NTFY_TOPIC="mirrorbuddy-dev-roberdan"
alias ntfy-listen="curl -s ntfy.sh/\$NTFY_TOPIC/raw"
alias ntfy-web="open https://ntfy.sh/\$NTFY_TOPIC"
alias ntfy="~/.local/scripts/ntfy-listener.sh"

# =============================================================================
# Linux Dev - One command to rule them all
# =============================================================================
alias linuxDev="~/.local/scripts/linuxDev.sh"
alias ld="~/.local/scripts/linuxDev.sh"  # Short version

# === Linux smart shortcuts (local network first, Tailscale fallback) ===
_omarchy_host() {
  # Try local first (fast timeout), then Tailscale
  if ssh -o ConnectTimeout=2 -o BatchMode=yes omarchy-local true 2>/dev/null; then
    echo "omarchy-local"
  elif ssh -o ConnectTimeout=3 -o BatchMode=yes omarchy-ts true 2>/dev/null; then
    echo "omarchy-ts"
  else
    echo "UNREACHABLE"
    return 1
  fi
}

tlx() {
  local host=$(_omarchy_host) || { echo "omarchy unreachable (local + Tailscale)"; return 1; }
  local sync_log="/tmp/tlx-presync-$(date +%s).log"

  if [[ "${1:-}" == "--sync" ]]; then
    # Blocking mode: sync first, then connect
    echo "Syncing to $host (blocking)..."
    ~/.claude/scripts/tlx-presync.sh "$host" 2>&1 | tee "$sync_log"
  else
    # Background mode: connect immediately, sync in parallel
    echo "Starting background sync to $host..."
    nohup ~/.claude/scripts/tlx-presync.sh "$host" > "$sync_log" 2>&1 &
    echo "Sync log: $sync_log"
  fi

  echo "Connecting via $host..."
  if [ -n "$KITTY_PID" ]; then
    kitten ssh "$host" -t "tmux new-session -A -s Convergio"
  else
    ssh "$host" -t "tmux new-session -A -s Convergio"
  fi
}

linux() {
  local host=$(_omarchy_host) || { echo "omarchy unreachable"; return 1; }
  echo "Connecting via $host..."
  ssh "$host"
}

linux-claude() {
  local host=$(_omarchy_host) || { echo "omarchy unreachable"; return 1; }
  ssh "$host" -t "cd /home/roberdan/GitHub/MirrorBuddy && claude"
}

linux-plan() {
  local host=$(_omarchy_host) || { echo "omarchy unreachable"; return 1; }
  ssh "$host" -t "cd /home/roberdan/GitHub/MirrorBuddy && claude --resume"
}

# Sync non-git files to Linux worktree
sync-linux() {
  local host=$(_omarchy_host) || { echo "omarchy unreachable"; return 1; }
  local src="${1:-/Users/roberdan/GitHub/MirrorBuddy}"
  local dst="$host:/home/roberdan/GitHub/MirrorBuddy"
  echo "Syncing .env and config files via $host..."
  rsync -avz "$src"/.env "$src"/.env.production "$src"/.env.vercel.local "$dst/"
  rsync -avz "$src"/.mcp.json "$dst/"
  rsync -avz "$src"/docs/busplan/ "$dst/docs/busplan/"
  [ -f "$src/backend/.env" ] && rsync -avz "$src/backend/.env" "$dst/backend/"
  echo "Done!"
}

linux-stats() {
  local host=$(_omarchy_host) || { echo "omarchy unreachable"; return 1; }
  ssh "$host" "uptime && free -h && ps aux --sort=-%cpu | head -6"
}

# === Buongiorno ===
buongiorno() {
  claude_buongiorno "$@"
}

# Dashboard DB sync
alias dbsync="~/.claude/scripts/sync-dashboard-db.sh"
# Dashboard Control Center
alias pianits="~/.claude/scripts/pianits"
# Claude scripts (plan-db.sh, worktree-check.sh, etc.)
export PATH="$HOME/.claude/scripts:$PATH"

# GitHub Copilot CLI with auto-accept
unalias copilot 2>/dev/null
alias copilot="command copilot --yolo"

# Grafana Cloud - MCP Server
export GRAFANA_SERVICE_ACCOUNT_TOKEN="glsa_AXZa5ZVKYfZZ0rtOjG71ZVl5itMz6YGP_556cf195"

[[ -f "/Users/roberdan/.config/kaku/zsh/kaku.zsh" ]] && source "/Users/roberdan/.config/kaku/zsh/kaku.zsh" # Kaku Shell Integration

# === Oh-My-Posh: DEVE essere l'ultima riga — sovrascrive starship da kaku.zsh ===
OMP_THEME="$HOME/.config/oh-my-posh/roberto.omp.json"
eval "$(oh-my-posh init zsh --config "$OMP_THEME")"

# auto-tmux-attach: persistent Convergio session for SSH connections
if [[ -n "$SSH_CONNECTION" && -z "$TMUX" && $- == *i* ]]; then
  exec tmux new-session -As convergio
fi

# CLUI-CC launcher
alias masterClaude="cd /Users/roberdan/GitHub/clui-cc && ./start.command"
