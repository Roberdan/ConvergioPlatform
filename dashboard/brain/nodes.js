/* brain/nodes.js — Node creation, rendering, color mapping, lifecycle */
'use strict';

import {
  BRAIN_CONFIG, BRAIN_IMMERSIVE, PI2,
  cachedColor, meshColor, buildPAL
} from './config.js';

let PAL = null;
function pal() { if (!PAL) PAL = buildPAL(); return PAL; }
function _cssVar(name, fallback) { return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback; }

// Canvas neuron — the visual Neuron used in the force-directed graph
export class Neuron {
  constructor(id, type, label, meta) {
    this.id = id; this.type = type; this.label = label; this.meta = meta || {};
    this.x = 0; this.y = 0; this.vx = 0; this.vy = 0;
    this.radius = type === 'session' ? BRAIN_CONFIG.SESSION_RADIUS : BRAIN_CONFIG.DEFAULT_NODE_RADIUS;
    this.phase = Math.random() * PI2;
    this.birth = performance.now();
    this.scale = 0; this.targetScale = 1;
    this.active = true; this.dying = false; this.deathT = 0;
    this.tool = 'claude'; this.fireT = 0;
  }
  get pal() {
    if ((this.type === 'plan' || this.type === 'task') && this.meta.executor_host) return meshColor(this.meta.executor_host);
    if (this.type === 'plan' || this.type === 'task') return meshColor(this.meta.executor_host || '?');
    return pal()[this.tool] || meshColor(this.tool);
  }
  fire() { this.fireT = performance.now(); }
}

/**
 * Render all neurons onto the canvas.
 * @param {CanvasRenderingContext2D} c
 * @param {Map} neurons
 * @param {number} ts — timestamp
 * @param {{w: number, h: number, hover: string|null}} state
 * @param {Function} getBrainMode
 */
export function renderNodes(c, neurons, ts, state, getBrainMode) {
  const sf = _sf(state.w, state.h);
  const mode = getBrainMode();
  const radiusScale = mode.nodeRadius / BRAIN_IMMERSIVE.nodeRadius;

  for (const [, n] of neurons) {
    if (n.dying) {
      n.scale = Math.max(0, n.scale - BRAIN_CONFIG.NODE_SCALE_OUT_SPEED);
      if (n.scale <= 0) continue;
    } else {
      n.scale += (n.targetScale - n.scale) * BRAIN_CONFIG.NODE_SCALE_LERP;
    }
    n.phase += BRAIN_CONFIG.NODE_PHASE_STEP;
    const pulse = 1 + BRAIN_CONFIG.NODE_PULSE_SCALE * Math.sin(n.phase * BRAIN_CONFIG.NODE_PULSE_SPEED);
    const modeRadiusScale = n.type === 'session' ? 1 : radiusScale;
    const r = n.radius * sf * n.scale * pulse * modeRadiusScale;
    if (r < BRAIN_CONFIG.MIN_VISIBLE_RADIUS) continue;

    const fireAge = ts - n.fireT;
    const fireGlow = fireAge < BRAIN_CONFIG.NODE_FIRE_GLOW_DURATION_MS ? 1 - fireAge / BRAIN_CONFIG.NODE_FIRE_GLOW_DURATION_MS : 0;
    const np = n.pal;

    c.save();
    if (BRAIN_CONFIG.SHADOWS_ENABLED && fireGlow > 0) {
      c.shadowBlur = BRAIN_CONFIG.NODE_SHADOW_BASE + fireGlow * BRAIN_CONFIG.NODE_SHADOW_GLOW; c.shadowColor = np.core;
    } else { c.shadowBlur = 0; }

    const g = c.createRadialGradient(
      n.x - r * BRAIN_CONFIG.NODE_FILL_OFFSET, n.y - r * BRAIN_CONFIG.NODE_FILL_OFFSET,
      r * BRAIN_CONFIG.NODE_FILL_INNER_RADIUS, n.x, n.y, r);
    g.addColorStop(0, cachedColor(np.glow, BRAIN_CONFIG.NODE_FILL_ALPHA_INNER));
    g.addColorStop(BRAIN_CONFIG.FLOW_ACTIVE_PARTIAL, np.core);
    g.addColorStop(1, cachedColor(np.glow, BRAIN_CONFIG.NODE_FILL_ALPHA_OUTER));
    c.fillStyle = g;
    c.beginPath(); c.arc(n.x, n.y, r, 0, PI2); c.fill();

    c.shadowBlur = 0;
    c.strokeStyle = cachedColor(np.glow,
      BRAIN_CONFIG.NODE_RING_ALPHA_BASE + fireGlow * BRAIN_CONFIG.NODE_RING_ALPHA_GLOW +
      BRAIN_CONFIG.NODE_RING_ALPHA_WAVE * Math.sin(n.phase * BRAIN_CONFIG.NODE_RING_WAVE_SPEED));
    c.lineWidth = n.type === 'session' ? BRAIN_CONFIG.SESSION_RING_WIDTH : BRAIN_CONFIG.DEFAULT_RING_WIDTH;
    c.beginPath(); c.arc(n.x, n.y, r + BRAIN_CONFIG.SESSION_RING_WIDTH, 0, PI2); c.stroke();
    c.restore();

    _drawLabels(c, n, r, mode, state, sf);
  }
}

function _drawLabels(c, n, r, mode, state, sf) {
  const fsf = _fontScale(sf);
  const P = pal();

  if (mode.labelVisible && n.type === 'session') {
    const isHover = state.hover === n.id;
    const fs = Math.round(BRAIN_CONFIG.FONT_SIZE_FACTOR * fsf);
    c.font = `${isHover ? 'bold ' : ''}${fs}px "JetBrains Mono",monospace`;
    c.textAlign = 'center';
    const ly = n.y + r + BRAIN_CONFIG.SESSION_LABEL_OFFSET * fsf;
    const tw = c.measureText(n.label).width;
    c.fillStyle = `rgba(10,16,36,${isHover ? BRAIN_CONFIG.SESSION_LABEL_BG_ALPHA_HOVER : BRAIN_CONFIG.SESSION_LABEL_BG_ALPHA})`;
    c.beginPath();
    const ph = fs + BRAIN_CONFIG.SESSION_LABEL_HEIGHT;
    if (c.roundRect) c.roundRect(n.x - tw / 2 - BRAIN_CONFIG.SESSION_LABEL_PADDING_X, ly - ph / 2 - BRAIN_CONFIG.SESSION_RING_WIDTH, tw + BRAIN_CONFIG.SESSION_LABEL_PADDING_X * 2, ph, BRAIN_CONFIG.SESSION_LABEL_RADIUS);
    else c.rect(n.x - tw / 2 - BRAIN_CONFIG.SESSION_LABEL_PADDING_X, ly - ph / 2 - BRAIN_CONFIG.SESSION_RING_WIDTH, tw + BRAIN_CONFIG.SESSION_LABEL_PADDING_X * 2, ph);
    c.fill();
    c.fillStyle = isHover ? _cssVar('--mn-text', '#fff') : _cssVar('--mn-text-muted', '#b0c4dd');
    c.fillText(n.label, n.x, ly + BRAIN_CONFIG.TEXT_BASELINE_OFFSET);
    if (isHover && n.meta.tty) {
      const info = [n.meta.tty, `PID ${n.meta.pid || '?'}`,
        n.meta.cpu != null ? `CPU ${n.meta.cpu}%` : '',
        n.meta.mem != null ? `MEM ${n.meta.mem}%` : ''].filter(Boolean).join(' · ');
      c.font = `${Math.round(BRAIN_CONFIG.SESSION_INFO_FONT_SIZE * fsf)}px "JetBrains Mono",monospace`;
      c.fillStyle = P.sub.core;
      c.fillText(info, n.x, ly + BRAIN_CONFIG.SESSION_LABEL_OFFSET * fsf);
    }
  }
  if (mode.labelVisible && (n.type === 'plan' || (n.type === 'task' && (state.hover === n.id || n.meta.status === 'in_progress')))) {
    _drawPlanTaskLabel(c, n, r, state, fsf);
  }
}

function _drawPlanTaskLabel(c, n, r, state, fsf) {
  const isHover = state.hover === n.id;
  const host = n.meta.executor_host || n.meta.host || '';
  const mp = meshColor(host || '?');
  c.textAlign = 'center';
  const ly = n.y + r + BRAIN_CONFIG.PLAN_LABEL_OFFSET * fsf;
  const fs = Math.round((n.type === 'plan' ? BRAIN_CONFIG.PLAN_FONT_SIZE : BRAIN_CONFIG.TASK_FONT_SIZE) * fsf);
  c.font = `${isHover ? 'bold ' : ''}${fs}px "JetBrains Mono",monospace`;
  const lbl = n.label.substring(0, n.type === 'plan' ? BRAIN_CONFIG.PLAN_LABEL_MAX : BRAIN_CONFIG.TASK_LABEL_MAX);
  const tw = c.measureText(lbl).width;
  const ph = fs + BRAIN_CONFIG.PLAN_LABEL_HEIGHT;
  c.fillStyle = 'rgba(10,16,36,0.7)';
  c.beginPath();
  if (c.roundRect) c.roundRect(n.x - tw / 2 - BRAIN_CONFIG.PLAN_LABEL_PADDING_X, ly - ph / 2 - 1, tw + BRAIN_CONFIG.PLAN_LABEL_PADDING_X * 2, ph, BRAIN_CONFIG.PLAN_LABEL_RADIUS);
  else c.rect(n.x - tw / 2 - BRAIN_CONFIG.PLAN_LABEL_PADDING_X, ly - ph / 2 - 1, tw + BRAIN_CONFIG.PLAN_LABEL_PADDING_X * 2, ph);
  c.fill();
  c.fillStyle = isHover ? _cssVar('--mn-text', '#fff') : _cssVar('--mn-text-muted', '#c8d0e8');
  c.fillText(lbl, n.x, ly + BRAIN_CONFIG.SESSION_RING_WIDTH);
  if (!host) return;
  const bfs = Math.round(BRAIN_CONFIG.BADGE_FONT_SIZE * fsf);
  c.font = `bold ${bfs}px "JetBrains Mono",monospace`;
  const bw = c.measureText(host).width;
  const bx = n.x - bw / 2 - BRAIN_CONFIG.BADGE_PADDING_X, by = ly + BRAIN_CONFIG.BADGE_OFFSET_Y * fsf;
  const bh = bfs + BRAIN_CONFIG.BADGE_HEIGHT;
  c.fillStyle = `${mp.glow}0.25)`;
  c.beginPath();
  if (c.roundRect) c.roundRect(bx, by, bw + BRAIN_CONFIG.BADGE_PADDING_X * 2, bh, BRAIN_CONFIG.BADGE_RADIUS);
  else c.rect(bx, by, bw + BRAIN_CONFIG.BADGE_PADDING_X * 2, bh);
  c.fill();
  c.strokeStyle = `${mp.glow}0.5)`;
  c.lineWidth = BRAIN_CONFIG.BADGE_BORDER_WIDTH; c.stroke();
  c.fillStyle = mp.core;
  c.fillText(host, n.x, by + bfs - 1);
}

function _sf(w, h) { return Math.sqrt((w * h) / BRAIN_CONFIG.CANVAS_REF_AREA); }
function _fontScale(sf) {
  return Math.max(BRAIN_CONFIG.FONT_SCALE_MIN, Math.min(BRAIN_CONFIG.FONT_SCALE_MAX, Math.pow(sf, BRAIN_CONFIG.FONT_SCALE_EXPONENT)));
}
