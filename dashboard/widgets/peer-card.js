/**
 * Reusable peer card component for mesh views.
 * Renders a single peer with OS icon, status badge, role, and resource gauges.
 */

const STYLE_ID = 'mn-peer-card-style';

function injectStyles() {
  if (document.getElementById(STYLE_ID)) return;
  const style = document.createElement('style');
  style.id = STYLE_ID;
  style.textContent = `
    .mn-peer-card {
      padding: 1rem;
      border-radius: var(--mn-radius, 8px);
      background: var(--mn-surface);
      border: 1px solid var(--mn-border);
    }
    .mn-peer-card__header {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      margin-bottom: 0.75rem;
    }
    .mn-peer-card__os {
      font-size: 1.25rem;
    }
    .mn-peer-card__name {
      font-weight: 700;
      color: var(--mn-text);
    }
    .mn-peer-card__gauges {
      display: flex;
      gap: 1rem;
    }
  `;
  document.head.appendChild(style);
}

function esc(s) {
  const d = document.createElement('div');
  d.textContent = s;
  return d.innerHTML;
}

function osIcon(os) {
  if (os === 'macos' || os === 'darwin') return '\uD83C\uDF4E';
  return '\uD83D\uDC27';
}

function statusBadge(isOnline) {
  const variant = isOnline ? 'success' : 'danger';
  const label = isOnline ? 'online' : 'offline';
  return `<span class="mn-badge mn-badge--${variant}">${label}</span>`;
}

/**
 * Create a peer card DOM element.
 * @param {{peer_name: string, os: string, role: string, is_online: boolean, cpu_percent: number, memory_mb: number}} peer
 * @returns {HTMLElement}
 */
export function createPeerCard(peer) {
  injectStyles();

  const card = document.createElement('div');
  card.className = 'mn-card mn-peer-card';

  const cpuVal = Math.round(peer.cpu_percent || 0);
  const memVal = Math.round(peer.memory_mb || 0);

  card.innerHTML = `
    <div class="mn-peer-card__header">
      <span class="mn-peer-card__os">${osIcon(peer.os)}</span>
      <strong class="mn-peer-card__name">${esc(peer.peer_name)}</strong>
      ${statusBadge(peer.is_online)}
      <span class="mn-badge">${esc(peer.role || 'worker')}</span>
    </div>
    <div class="mn-peer-card__gauges">
      <mn-gauge value="${cpuVal}" max="100" unit="%" label="CPU" size="sm"></mn-gauge>
      <mn-gauge value="${memVal}" max="65536" unit="MB" label="Memory" size="sm"></mn-gauge>
    </div>
  `;

  return card;
}

export default createPeerCard;
