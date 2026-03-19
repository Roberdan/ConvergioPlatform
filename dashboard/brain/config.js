/* brain/config.js — All BRAIN_CONFIG constants and palette definitions */
'use strict';

const _style = () => getComputedStyle(document.documentElement);
const cssVar = (name, fallback) => _style().getPropertyValue(name).trim() || fallback;

export const PI2 = Math.PI * 2;

export const BRAIN_CONFIG = {
  MESH_HUE_SPREAD: 137.508, MESH_SATURATION: 82, MESH_LIGHTNESS: 62, RGB_MAX: 255,
  SESSION_RADIUS: 18, DEFAULT_NODE_RADIUS: 8, CANVAS_REF_AREA: 480 * 800,
  FONT_SCALE_MIN: 0.7, FONT_SCALE_MAX: 1.4, FONT_SCALE_EXPONENT: 0.35,
  FRAME_BUDGET_MS: 8, DENSITY_THRESHOLD: 40, DENSITY_MIN_FACTOR: 0.4,
  REPULSION_K: 90, REPULSION_RADIUS_MULTIPLIER: 3.5,
  NODE_THROTTLE_THRESHOLD: 50, NODE_THROTTLE_STRIDE: 2,
  SPREAD_FACTOR: 0.38, GRAVITY_STRONG: 0.01, GRAVITY_WEAK: 0.0015,
  SPRING_SESSION_MULTIPLIER: 2.2, SPRING_DEFAULT_MULTIPLIER: 1.4, SPRING_FORCE: 0.002,
  REPULSION_FORCE_SCALE: 0.5, DAMPING: 0.82,
  BOUNDS_MARGIN_MIN: 20, BOUNDS_MARGIN_PADDING: 15, BOUNDS_REBOUND: 0.3, GRID_SIZE: 50,
  FLOW_ACTIVE_FULL: 1.0, FLOW_ACTIVE_PARTIAL: 0.6, FLOW_DONE: 0.1, FLOW_IDLE_BASE: 0.35,
  FIRE_GLOW_DURATION_MS: 1200, DEFAULT_SYNAPSE_STRENGTH: 0.3,
  SYNAPSE_ACTIVE_BASE_ALPHA: 0.2, SYNAPSE_ACTIVE_GLOW_ALPHA: 0.4,
  SYNAPSE_DONE_ALPHA: 0.12, SYNAPSE_IDLE_BASE_ALPHA: 0.12,
  SYNAPSE_IDLE_STRENGTH_ALPHA: 0.1, SYNAPSE_IDLE_GLOW_ALPHA: 0.3,
  SYNAPSE_CURVATURE_BASE: 0.1, SYNAPSE_CURVATURE_WAVE: 0.05, SYNAPSE_CURVATURE_SPEED: 0.001,
  SYNAPSE_WIDTH_ACTIVE: 1.5, SYNAPSE_WIDTH_ACTIVE_GLOW: 2.5,
  SYNAPSE_WIDTH_DONE: 0.8, SYNAPSE_WIDTH_IDLE: 0.8,
  SYNAPSE_SHADOW_BASE: 6, SYNAPSE_SHADOW_GLOW: 12, SYNAPSE_SHADOW_ALPHA: 0.4,
  PULSE_THRESHOLD: 0.3, PULSE_RADIUS_BASE: 2, PULSE_RADIUS_WAVE: 1.5,
  PULSE_ALPHA_BASE: 0.3, PULSE_ALPHA_GLOW: 0.4, PULSE_SHADOW_BLUR: 8,
  PULSE_CHECK_SPEED: 0.003, PULSE_WAVE_SPEED: 0.005,
  PARTICLE_FIRE_BASE: 3, PARTICLE_FIRE_VARIANCE: 4,
  PARTICLE_FIRE_SPEED_MIN: 0.2, PARTICLE_FIRE_SPEED_RANGE: 0.5,
  PARTICLE_FIRE_SIZE_MIN: 1.5, PARTICLE_FIRE_SIZE_RANGE: 2.5,
  PARTICLE_FLOW_SPAWN_RATE: 0.08, PARTICLE_FLOW_SPEED_MIN: 0.15,
  PARTICLE_FLOW_SPEED_RANGE: 0.35, PARTICLE_FLOW_SIZE_MIN: 1, PARTICLE_FLOW_SIZE_RANGE: 2,
  PARTICLE_DT: 0.016, PARTICLE_TRAIL_MAX: 6,
  PARTICLE_TRAIL_ALPHA: 0.4, PARTICLE_TRAIL_WIDTH: 0.8,
  PARTICLE_FADE_START: 0.85, PARTICLE_FADE_RANGE: 0.15, PARTICLE_FADE_IN_RANGE: 0.1,
  MAX_PARTICLES: 120, SHADOWS_ENABLED: false,
  NODE_SCALE_OUT_SPEED: 0.03, NODE_SCALE_LERP: 0.08,
  NODE_PHASE_STEP: 0.003, NODE_PULSE_SCALE: 0.06, NODE_PULSE_SPEED: 3,
  NODE_FIRE_GLOW_DURATION_MS: 600, NODE_SHADOW_BASE: 4, NODE_SHADOW_GLOW: 6,
  NODE_FILL_OFFSET: 0.25, NODE_FILL_INNER_RADIUS: 0.05,
  NODE_FILL_ALPHA_INNER: 0.95, NODE_FILL_ALPHA_OUTER: 0.7,
  NODE_RING_ALPHA_BASE: 0.4, NODE_RING_ALPHA_GLOW: 0.5,
  NODE_RING_ALPHA_WAVE: 0.1, NODE_RING_WAVE_SPEED: 2,
  MIN_VISIBLE_RADIUS: 0.5, SESSION_RING_WIDTH: 2, DEFAULT_RING_WIDTH: 1.2,
  FONT_SIZE_FACTOR: 11, SESSION_LABEL_OFFSET: 14,
  SESSION_LABEL_BG_ALPHA_HOVER: 0.85, SESSION_LABEL_BG_ALPHA: 0.65,
  SESSION_LABEL_PADDING_X: 6, SESSION_LABEL_HEIGHT: 4, SESSION_LABEL_RADIUS: 4,
  TEXT_BASELINE_OFFSET: 3, SESSION_INFO_FONT_SIZE: 9,
  PLAN_LABEL_OFFSET: 10, PLAN_FONT_SIZE: 10, TASK_FONT_SIZE: 8,
  PLAN_LABEL_MAX: 22, TASK_LABEL_MAX: 25,
  PLAN_LABEL_PADDING_X: 4, PLAN_LABEL_HEIGHT: 3, PLAN_LABEL_RADIUS: 3,
  BADGE_FONT_SIZE: 7, BADGE_PADDING_X: 3, BADGE_OFFSET_Y: 4,
  BADGE_HEIGHT: 3, BADGE_RADIUS: 2, BADGE_BORDER_WIDTH: 0.5,
  SESSION_DESC_MAX: 18, CHILD_DESC_MAX: 22,
  SESSION_ORBIT_RADIUS: 0.28, SESSION_SPAWN_JITTER: 50,
  SESSION_FIRE_DELAY_MS: 300, CHILD_FIRE_DELAY_MS: 200,
  PLAN_RADIUS_MIN: 14, PLAN_RADIUS_MAX: 36, PLAN_RADIUS_BASE: 10, PLAN_RADIUS_SCALE: 8,
  PLAN_NAME_MAX: 20, PLAN_ORBIT_FACTOR: 0.32,
  PLAN_ORBIT_JITTER_ANGLE: 0.3, PLAN_ORBIT_JITTER_POS: 40,
  TASK_LINES_RADIUS_MIN: 5, TASK_LINES_RADIUS_MAX: 22, TASK_FALLBACK_RADIUS_MAX: 14,
  TASK_RADIUS_BASE: 4, TASK_RADIUS_SCALE: 0.8, TASK_TOKEN_DIVISOR: 5000,
  TASK_PENDING_RADIUS: 3, TASK_ORBIT_MIN: 40, TASK_ORBIT_RANGE: 60,
  WAVE_LINK_STRENGTH: 0.15, AMBIENT_FIRE_CHANCE: 0.02,
  MINUTE_SECONDS: 60, HOUR_SECONDS: 3600, TOKENS_KILO: 1000, TOKENS_MEGA: 1000000,
  TOOLTIP_LINE_HEIGHT: 18, TOOLTIP_PADDING: 12, TOOLTIP_TEXT_WIDTH_FACTOR: 9,
  TOOLTIP_FALLBACK_WIDTH: 120, TOOLTIP_MAX_WIDTH: 420,
  TOOLTIP_OFFSET: 15, TOOLTIP_EDGE_MARGIN: 10, CANVAS_MIN_SIZE: 10,
  HIT_PADDING: 14, TOUCH_CLEAR_DELAY_MS: 2000, POLL_INTERVAL_MS: 8000,
  WS_RETRY_BASE_MS: 1000, WS_RETRY_MAX_MS: 30000, WS_RETRY_EXP_BASE: 2,
  BOOT_DELAY_MS: 100, GRADIENT_CACHE_MAX_SIZE: 500, GRADIENT_CACHE_SWEEP_FRAMES: 60,
  CANVAS_ARIA_LABEL: 'Neural network visualization showing active sessions and task connections'
};

export const BRAIN_EMBEDDED = {
  nodeRadius: 6, synapseWidth: 0.8, particleCountScale: 0.3,
  labelVisible: false, controlsVisible: false, fireEffects: 'perimeter'
};
export const BRAIN_IMMERSIVE = {
  nodeRadius: 14, synapseWidth: 2, particleCountScale: 1.0,
  labelVisible: true, controlsVisible: true, fireEffects: 'full'
};

// Palette — reads CSS custom properties, falls back to design tokens
export function buildPAL() {
  return {
    claude: { h: 35, core: cssVar('--mn-brain-session', '#ffb020'), glow: `rgba(${_hexToRgb(cssVar('--mn-brain-session', '#ffb020'))},`, ring: cssVar('--mn-brain-session-ring', '#ffd06088') },
    copilot: { h: 210, core: cssVar('--mn-brain-copilot', '#20a0ff'), glow: `rgba(${_hexToRgb(cssVar('--mn-brain-copilot', '#20a0ff'))},`, ring: cssVar('--mn-brain-copilot-ring', '#60c0ff88') },
    opencode: { h: 150, core: cssVar('--mn-brain-opencode', '#00e080'), glow: `rgba(${_hexToRgb(cssVar('--mn-brain-opencode', '#00e080'))},`, ring: cssVar('--mn-brain-opencode-ring', '#40ffa088') },
    sub: { core: cssVar('--mn-brain-sub', '#00e5ff'), glow: `rgba(${_hexToRgb(cssVar('--mn-brain-sub', '#00e5ff'))},` },
    synapse: `rgba(${_hexToRgb(cssVar('--mn-brain-synapse', '#00e5ff'))},`,
    green: cssVar('--mn-success', '#00ff88'),
    dim: cssVar('--mn-brain-dim', '#2a3456')
  };
}

function _hexToRgb(hex) {
  hex = hex.replace('#', '');
  if (hex.length === 3) hex = hex[0] + hex[0] + hex[1] + hex[1] + hex[2] + hex[2];
  const n = parseInt(hex.substring(0, 6), 16);
  return `${(n >> 16) & 255},${(n >> 8) & 255},${n & 255}`;
}

const _colorCache = new Map();
export function cachedColor(base, alpha) {
  const key = base + '|' + (alpha * 1000 | 0);
  let v = _colorCache.get(key);
  if (!v) { v = `${base}${alpha.toFixed(3)})`; _colorCache.set(key, v); }
  return v;
}

export const gradientCache = new Map();

function hslToRgb(h, s, l) {
  s /= 100; l /= 100;
  const k = n => (n + h / 30) % 12;
  const a = s * Math.min(l, 1 - l);
  const f = n => l - a * Math.max(-1, Math.min(k(n) - 3, 9 - k(n), 1));
  return [Math.round(f(0) * BRAIN_CONFIG.RGB_MAX), Math.round(f(8) * BRAIN_CONFIG.RGB_MAX), Math.round(f(4) * BRAIN_CONFIG.RGB_MAX)];
}

const _meshColorCache = {};
export function meshColor(name) {
  if (!name) name = '?';
  if (_meshColorCache[name]) return _meshColorCache[name];
  let hash = 0;
  for (let i = 0; i < name.length; i++) hash = ((hash << 5) - hash + name.charCodeAt(i)) | 0;
  const hue = ((Math.abs(hash) * BRAIN_CONFIG.MESH_HUE_SPREAD) % 360);
  const core = `hsl(${hue},${BRAIN_CONFIG.MESH_SATURATION}%,${BRAIN_CONFIG.MESH_LIGHTNESS}%)`;
  const c = hslToRgb(hue, BRAIN_CONFIG.MESH_SATURATION, BRAIN_CONFIG.MESH_LIGHTNESS);
  const glow = `rgba(${c[0]},${c[1]},${c[2]},`;
  const ring = `rgba(${c[0]},${c[1]},${c[2]},0.5)`;
  _meshColorCache[name] = { core, glow, ring, hue, label: name };
  return _meshColorCache[name];
}

window.brainMeshColor = meshColor;
