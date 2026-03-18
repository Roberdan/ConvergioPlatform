/* brain-canvas.js — Neural network force-directed graph visualization */
(() => {
  'use strict';
  const PI2 = Math.PI * 2;
  const BRAIN_CONFIG = {
    MESH_HUE_SPREAD: 137.508,
    MESH_SATURATION: 82,
    MESH_LIGHTNESS: 62,
    RGB_MAX: 255,
    SESSION_RADIUS: 18,
    DEFAULT_NODE_RADIUS: 8,
    CANVAS_REF_AREA: 480 * 800,
    FONT_SCALE_MIN: 0.7,
    FONT_SCALE_MAX: 1.4,
    FONT_SCALE_EXPONENT: 0.35,
    FRAME_BUDGET_MS: 8,
    DENSITY_THRESHOLD: 40,
    DENSITY_MIN_FACTOR: 0.4,
    REPULSION_K: 90,
    REPULSION_RADIUS_MULTIPLIER: 3.5,
    NODE_THROTTLE_THRESHOLD: 50,
    NODE_THROTTLE_STRIDE: 2,
    SPREAD_FACTOR: 0.38,
    GRAVITY_STRONG: 0.01,
    GRAVITY_WEAK: 0.0015,
    SPRING_SESSION_MULTIPLIER: 2.2,
    SPRING_DEFAULT_MULTIPLIER: 1.4,
    SPRING_FORCE: 0.002,
    REPULSION_FORCE_SCALE: 0.5,
    DAMPING: 0.82,
    BOUNDS_MARGIN_MIN: 20,
    BOUNDS_MARGIN_PADDING: 15,
    BOUNDS_REBOUND: 0.3,
    GRID_SIZE: 50,
    FLOW_ACTIVE_FULL: 1.0,
    FLOW_ACTIVE_PARTIAL: 0.6,
    FLOW_DONE: 0.1,
    FLOW_IDLE_BASE: 0.35,
    FIRE_GLOW_DURATION_MS: 1200,
    DEFAULT_SYNAPSE_STRENGTH: 0.3,
    SYNAPSE_ACTIVE_BASE_ALPHA: 0.2,
    SYNAPSE_ACTIVE_GLOW_ALPHA: 0.4,
    SYNAPSE_DONE_ALPHA: 0.12,
    SYNAPSE_IDLE_BASE_ALPHA: 0.12,
    SYNAPSE_IDLE_STRENGTH_ALPHA: 0.1,
    SYNAPSE_IDLE_GLOW_ALPHA: 0.3,
    SYNAPSE_CURVATURE_BASE: 0.1,
    SYNAPSE_CURVATURE_WAVE: 0.05,
    SYNAPSE_CURVATURE_SPEED: 0.001,
    SYNAPSE_WIDTH_ACTIVE: 1.5,
    SYNAPSE_WIDTH_ACTIVE_GLOW: 2.5,
    SYNAPSE_WIDTH_DONE: 0.8,
    SYNAPSE_WIDTH_IDLE: 0.8,
    SYNAPSE_SHADOW_BASE: 6,
    SYNAPSE_SHADOW_GLOW: 12,
    SYNAPSE_SHADOW_ALPHA: 0.4,
    PULSE_THRESHOLD: 0.3,
    PULSE_RADIUS_BASE: 2,
    PULSE_RADIUS_WAVE: 1.5,
    PULSE_ALPHA_BASE: 0.3,
    PULSE_ALPHA_GLOW: 0.4,
    PULSE_SHADOW_BLUR: 8,
    PULSE_CHECK_SPEED: 0.003,
    PULSE_WAVE_SPEED: 0.005,
    PARTICLE_FIRE_BASE: 3,
    PARTICLE_FIRE_VARIANCE: 4,
    PARTICLE_FIRE_SPEED_MIN: 0.2,
    PARTICLE_FIRE_SPEED_RANGE: 0.5,
    PARTICLE_FIRE_SIZE_MIN: 1.5,
    PARTICLE_FIRE_SIZE_RANGE: 2.5,
    PARTICLE_FLOW_SPAWN_RATE: 0.08,
    PARTICLE_FLOW_SPEED_MIN: 0.15,
    PARTICLE_FLOW_SPEED_RANGE: 0.35,
    PARTICLE_FLOW_SIZE_MIN: 1,
    PARTICLE_FLOW_SIZE_RANGE: 2,
    PARTICLE_DT: 0.016,
    PARTICLE_TRAIL_MAX: 6,
    PARTICLE_TRAIL_ALPHA: 0.4,
    PARTICLE_TRAIL_WIDTH: 0.8,
    PARTICLE_FADE_START: 0.85,
    PARTICLE_FADE_RANGE: 0.15,
    PARTICLE_FADE_IN_RANGE: 0.1,
    MAX_PARTICLES: 120,
    SHADOWS_ENABLED: false,  // Disable expensive blur — CSS glow is enough
    NODE_SCALE_OUT_SPEED: 0.03,
    NODE_SCALE_LERP: 0.08,
    NODE_PHASE_STEP: 0.003,
    NODE_PULSE_SCALE: 0.06,
    NODE_PULSE_SPEED: 3,
    NODE_FIRE_GLOW_DURATION_MS: 600,
    NODE_SHADOW_BASE: 4,
    NODE_SHADOW_GLOW: 6,
    NODE_FILL_OFFSET: 0.25,
    NODE_FILL_INNER_RADIUS: 0.05,
    NODE_FILL_ALPHA_INNER: 0.95,
    NODE_FILL_ALPHA_OUTER: 0.7,
    NODE_RING_ALPHA_BASE: 0.4,
    NODE_RING_ALPHA_GLOW: 0.5,
    NODE_RING_ALPHA_WAVE: 0.1,
    NODE_RING_WAVE_SPEED: 2,
    MIN_VISIBLE_RADIUS: 0.5,
    SESSION_RING_WIDTH: 2,
    DEFAULT_RING_WIDTH: 1.2,
    FONT_SIZE_FACTOR: 11,
    SESSION_LABEL_OFFSET: 14,
    SESSION_LABEL_BG_ALPHA_HOVER: 0.85,
    SESSION_LABEL_BG_ALPHA: 0.65,
    SESSION_LABEL_PADDING_X: 6,
    SESSION_LABEL_HEIGHT: 4,
    SESSION_LABEL_RADIUS: 4,
    TEXT_BASELINE_OFFSET: 3,
    SESSION_INFO_FONT_SIZE: 9,
    PLAN_LABEL_OFFSET: 10,
    PLAN_FONT_SIZE: 10,
    TASK_FONT_SIZE: 8,
    PLAN_LABEL_MAX: 22,
    TASK_LABEL_MAX: 25,
    PLAN_LABEL_PADDING_X: 4,
    PLAN_LABEL_HEIGHT: 3,
    PLAN_LABEL_RADIUS: 3,
    BADGE_FONT_SIZE: 7,
    BADGE_PADDING_X: 3,
    BADGE_OFFSET_Y: 4,
    BADGE_HEIGHT: 3,
    BADGE_RADIUS: 2,
    BADGE_BORDER_WIDTH: 0.5,
    SESSION_DESC_MAX: 18,
    CHILD_DESC_MAX: 22,
    SESSION_ORBIT_RADIUS: 0.28,
    SESSION_SPAWN_JITTER: 50,
    SESSION_FIRE_DELAY_MS: 300,
    CHILD_FIRE_DELAY_MS: 200,
    PLAN_RADIUS_MIN: 14,
    PLAN_RADIUS_MAX: 36,
    PLAN_RADIUS_BASE: 10,
    PLAN_RADIUS_SCALE: 8,
    PLAN_NAME_MAX: 20,
    PLAN_ORBIT_FACTOR: 0.32,
    PLAN_ORBIT_JITTER_ANGLE: 0.3,
    PLAN_ORBIT_JITTER_POS: 40,
    TASK_LINES_RADIUS_MIN: 5,
    TASK_LINES_RADIUS_MAX: 22,
    TASK_FALLBACK_RADIUS_MAX: 14,
    TASK_RADIUS_BASE: 4,
    TASK_RADIUS_SCALE: 0.8,
    TASK_TOKEN_DIVISOR: 5000,
    TASK_PENDING_RADIUS: 3,
    TASK_ORBIT_MIN: 40,
    TASK_ORBIT_RANGE: 60,
    WAVE_LINK_STRENGTH: 0.15,
    AMBIENT_FIRE_CHANCE: 0.02,
    MINUTE_SECONDS: 60,
    HOUR_SECONDS: 3600,
    TOKENS_KILO: 1000,
    TOKENS_MEGA: 1000000,
    TOOLTIP_LINE_HEIGHT: 18,
    TOOLTIP_PADDING: 12,
    TOOLTIP_TEXT_WIDTH_FACTOR: 9,
    TOOLTIP_FALLBACK_WIDTH: 120,
    TOOLTIP_MAX_WIDTH: 420,
    TOOLTIP_OFFSET: 15,
    TOOLTIP_EDGE_MARGIN: 10,
    CANVAS_MIN_SIZE: 10,
    HIT_PADDING: 14,
    TOUCH_CLEAR_DELAY_MS: 2000,
    POLL_INTERVAL_MS: 8000,
    WS_RETRY_BASE_MS: 1000,
    WS_RETRY_MAX_MS: 30000,
    WS_RETRY_EXP_BASE: 2,
    BOOT_DELAY_MS: 100,
    GRADIENT_CACHE_MAX_SIZE: 500,
    GRADIENT_CACHE_SWEEP_FRAMES: 60,
    CANVAS_ARIA_LABEL: 'Neural network visualization showing active sessions and task connections'
  };
  const _colorCache = new Map();
  function cachedColor(base, alpha) {
    const key = base + '|' + (alpha * 1000 | 0);
    let v = _colorCache.get(key);
    if (!v) { v = `${base}${alpha.toFixed(3)})`; _colorCache.set(key, v); }
    return v;
  }
  const BRAIN_EMBEDDED = {
    nodeRadius: 6,
    synapseWidth: 0.8,
    particleCountScale: 0.3,
    labelVisible: false,
    controlsVisible: false,
    fireEffects: 'perimeter'
  };
  const BRAIN_IMMERSIVE = {
    nodeRadius: 14,
    synapseWidth: 2,
    particleCountScale: 1.0,
    labelVisible: true,
    controlsVisible: true,
    fireEffects: 'full'
  };
  const _gradientCache = new Map();
  const PAL = {
    claude: { h: 35, core: '#ffb020', glow: 'rgba(255,176,32,', ring: '#ffd06088' },
    copilot: { h: 210, core: '#20a0ff', glow: 'rgba(32,160,255,', ring: '#60c0ff88' },
    opencode: { h: 150, core: '#00e080', glow: 'rgba(0,224,128,', ring: '#40ffa088' },
    sub: { core: '#00e5ff', glow: 'rgba(0,229,255,' },
    synapse: 'rgba(0,229,255,', green: '#00ff88', dim: '#2a3456'
  };

  // Hash-based color generator — same name always produces same color
  const _meshColorCache = {};
  // Deterministic vivid colors — hash-seeded with high hue separation
  function meshColor(name) {
    if (!name) name = '?';
    if (_meshColorCache[name]) return _meshColorCache[name];
    let hash = 0;
    for (let i = 0; i < name.length; i++) hash = ((hash << 5) - hash + name.charCodeAt(i)) | 0;
    // Use golden ratio to spread hues maximally from hash seed
    const hue = ((Math.abs(hash) * BRAIN_CONFIG.MESH_HUE_SPREAD) % 360);
    const sat = BRAIN_CONFIG.MESH_SATURATION;
    const lit = BRAIN_CONFIG.MESH_LIGHTNESS;
    const core = `hsl(${hue},${sat}%,${lit}%)`;
    // Pre-compute rgba prefix for glow (approximate from HSL)
    const c = hslToRgb(hue, sat, lit);
    const glow = `rgba(${c[0]},${c[1]},${c[2]},`;
    const ring = `rgba(${c[0]},${c[1]},${c[2]},0.5)`;
    _meshColorCache[name] = { core, glow, ring, hue, label: name };
    return _meshColorCache[name];
  }
  function hslToRgb(h, s, l) {
    s /= 100; l /= 100;
    const k = n => (n + h / 30) % 12;
    const a = s * Math.min(l, 1 - l);
    const f = n => l - a * Math.max(-1, Math.min(k(n) - 3, 9 - k(n), 1));
    return [
      Math.round(f(0) * BRAIN_CONFIG.RGB_MAX),
      Math.round(f(8) * BRAIN_CONFIG.RGB_MAX),
      Math.round(f(4) * BRAIN_CONFIG.RGB_MAX)
    ];
  }
  // Export for mesh panel to use the same colors
  window.brainMeshColor = meshColor;

  /* ─── Node / Edge state ─── */
  class Neuron {
    constructor(id, type, label, meta) {
      this.id = id; this.type = type; this.label = label; this.meta = meta || {};
      this.x = 0; this.y = 0; this.vx = 0; this.vy = 0;
      this.radius = type === 'session' ? BRAIN_CONFIG.SESSION_RADIUS : getBrainMode().nodeRadius;
      this.phase = Math.random() * PI2;
      this.birth = performance.now();
      this.scale = 0; this.targetScale = 1;
      this.active = true; this.dying = false; this.deathT = 0;
      this.tool = 'claude';
      this.fireT = 0;
    }
    get pal() {
      if ((this.type === 'plan' || this.type === 'task') && this.meta.executor_host) return meshColor(this.meta.executor_host);
      if (this.type === 'plan' || this.type === 'task') return meshColor(this.meta.executor_host || '?');
      return PAL[this.tool] || meshColor(this.tool);
    }
    fire() { this.fireT = performance.now(); }
  }
  class Synapse {
    constructor(from, to) {
      this.from = from; this.to = to;
      this.particles = []; this.lastFire = 0; this.strength = BRAIN_CONFIG.DEFAULT_SYNAPSE_STRENGTH;
      this.flowRate = 0; // 0=dormant, 1=max flow (continuous particles)
    }
    fire() {
      this.lastFire = performance.now();
      const mode = getBrainMode();
      const modeMaxParticles = Math.max(8, Math.floor(BRAIN_CONFIG.MAX_PARTICLES * mode.particleCountScale));
      const fireCount = Math.max(1, Math.floor(
        (BRAIN_CONFIG.PARTICLE_FIRE_BASE + Math.floor(Math.random() * BRAIN_CONFIG.PARTICLE_FIRE_VARIANCE)) * mode.particleCountScale
      ));
      const count = Math.min(modeMaxParticles - this.particles.length, fireCount);
      for (let i = 0; i < count; i++) {
        this.particles.push({
          t: 0,
          speed: BRAIN_CONFIG.PARTICLE_FIRE_SPEED_MIN + Math.random() * BRAIN_CONFIG.PARTICLE_FIRE_SPEED_RANGE,
          size: BRAIN_CONFIG.PARTICLE_FIRE_SIZE_MIN + Math.random() * BRAIN_CONFIG.PARTICLE_FIRE_SIZE_RANGE,
          trail: new Array(BRAIN_CONFIG.PARTICLE_TRAIL_MAX),
          trailIdx: 0,
          trailLen: 0
        });
      }
    }
    // Continuous flow — spawns particles at a rate
    flow(rate) {
      this.flowRate = rate;
      const mode = getBrainMode();
      const modeMaxParticles = Math.max(8, Math.floor(BRAIN_CONFIG.MAX_PARTICLES * mode.particleCountScale));
      const spawnRate = rate * BRAIN_CONFIG.PARTICLE_FLOW_SPAWN_RATE * mode.particleCountScale;
      if (rate > 0 && this.particles.length < modeMaxParticles && Math.random() < spawnRate) {
        this.particles.push({
          t: 0,
          speed: BRAIN_CONFIG.PARTICLE_FLOW_SPEED_MIN + Math.random() * BRAIN_CONFIG.PARTICLE_FLOW_SPEED_RANGE,
          size: BRAIN_CONFIG.PARTICLE_FLOW_SIZE_MIN + Math.random() * BRAIN_CONFIG.PARTICLE_FLOW_SIZE_RANGE,
          trail: new Array(BRAIN_CONFIG.PARTICLE_TRAIL_MAX),
          trailIdx: 0,
          trailLen: 0
        });
      }
    }
  }
  class SpatialHash {
    constructor(cellSize) {
      this.cellSize = Math.max(1, cellSize);
      this.grid = new Map();
    }
    clear() {
      this.grid.clear();
    }
    insert(node) {
      const key = this._key(node.x, node.y);
      const bucket = this.grid.get(key);
      if (bucket) bucket.push(node);
      else this.grid.set(key, [node]);
    }
    getNearby(x, y, radius) {
      const minX = Math.floor((x - radius) / this.cellSize);
      const maxX = Math.floor((x + radius) / this.cellSize);
      const minY = Math.floor((y - radius) / this.cellSize);
      const maxY = Math.floor((y + radius) / this.cellSize);
      const nearby = [];
      for (let gx = minX; gx <= maxX; gx++) {
        for (let gy = minY; gy <= maxY; gy++) {
          const bucket = this.grid.get(`${gx},${gy}`);
          if (bucket) nearby.push(...bucket);
        }
      }
      return nearby;
    }
    _key(x, y) {
      return `${Math.floor(x / this.cellSize)},${Math.floor(y / this.cellSize)}`;
    }
  }

  const S = {
    container: null, canvas: null, ctx: null, w: 0, h: 0, dpr: 1,
    raf: 0, running: true, lastTs: 0,
    webglRenderer: null,
    neurons: new Map(), synapses: [], coreNeuron: null,
    pollT: 0, ws: null, wsRetry: 0, wsT: 0,
    sessions: [], agents: [], brainData: null, forceTick: 0, frameCount: 0, gradientSweepFrame: 0,
    hover: null, mouse: { x: -1, y: -1 }
  };

  function getBrainMode() {
    const container = document.getElementById('brain-canvas-container') || S.container;
    if (!container) return BRAIN_EMBEDDED;
    return container.offsetWidth > 500 ? BRAIN_IMMERSIVE : BRAIN_EMBEDDED;
  }
  function applyBrainMode(mode = getBrainMode()) {
    if (!S.container) return;
    S.container.classList.toggle('brain-immersive', mode === BRAIN_IMMERSIVE);
    S.container.classList.toggle('brain-embedded', mode === BRAIN_EMBEDDED);
    S.container.style.height = mode === BRAIN_EMBEDDED ? '300px' : '100%';
    const controls = document.getElementById('brain-controls');
    if (controls) controls.style.display = mode.controlsVisible ? '' : 'none';
  }

  /* ─── Scale factor — adapts all sizes to canvas area ─── */
  function scaleFactor() {
    const area = S.w * S.h;
    return Math.sqrt(area / BRAIN_CONFIG.CANVAS_REF_AREA);
  }
  // Dampened scale for fonts — grows much slower than node sizes
  function fontScale() {
    const sf = scaleFactor();
    return Math.max(BRAIN_CONFIG.FONT_SCALE_MIN, Math.min(BRAIN_CONFIG.FONT_SCALE_MAX, Math.pow(sf, BRAIN_CONFIG.FONT_SCALE_EXPONENT)));
  }

  /* ─── Force-directed layout ─── */
  function applyForces() {
    // Try WASM physics first
    if (typeof window.brainWasmStep === 'function' && window.brainWasmReady && window.brainWasmReady()) {
      const wasmHandled = window.brainWasmStep(S.neurons, S.synapses, S.w, S.h, BRAIN_CONFIG);
      if (wasmHandled) return;
    }
    const nodes = [...S.neurons.values()].filter(n => !n.dying);
    const cx = S.w / 2, cy = S.h / 2;
    const sf = scaleFactor();
    const frameStart = performance.now();
    // Adaptive k — shrink repulsion when many nodes to fit within canvas
    const nodeCount = nodes.length;
    const densityFactor = nodeCount > BRAIN_CONFIG.DENSITY_THRESHOLD
      ? Math.max(BRAIN_CONFIG.DENSITY_MIN_FACTOR, BRAIN_CONFIG.DENSITY_THRESHOLD / nodeCount)
      : 1;
    const k = BRAIN_CONFIG.REPULSION_K * sf * densityFactor;
    const repulsionRadius = k * BRAIN_CONFIG.REPULSION_RADIUS_MULTIPLIER;
    const spatialHash = new SpatialHash(repulsionRadius);
    for (const node of nodes) spatialHash.insert(node);
    const throttleStride = nodeCount > BRAIN_CONFIG.NODE_THROTTLE_THRESHOLD ? BRAIN_CONFIG.NODE_THROTTLE_STRIDE : 1;
    const runRepulsion = throttleStride === 1 || (++S.forceTick % throttleStride) === 0;
    let budgetExceeded = false;
    // Elliptical spread — use BOTH dimensions, not just the smaller one
    const spreadX = S.w * BRAIN_CONFIG.SPREAD_FACTOR;
    const spreadY = S.h * BRAIN_CONFIG.SPREAD_FACTOR;

    for (const n of nodes) {
      if (performance.now() - frameStart > BRAIN_CONFIG.FRAME_BUDGET_MS) {
        budgetExceeded = true;
        break;
      }
      // Elliptical gravity — normalized distance to center ellipse
      const dx = cx - n.x, dy = cy - n.y;
      const normDist = Math.sqrt((dx * dx) / (spreadX * spreadX) + (dy * dy) / (spreadY * spreadY));
      const grav = normDist > 1 ? BRAIN_CONFIG.GRAVITY_STRONG : BRAIN_CONFIG.GRAVITY_WEAK;
      n.vx += dx * grav;
      n.vy += dy * grav;

      if (!runRepulsion) continue;
      // Repulsion
      for (const m of spatialHash.getNearby(n.x, n.y, repulsionRadius)) {
        if (performance.now() - frameStart > BRAIN_CONFIG.FRAME_BUDGET_MS) {
          budgetExceeded = true;
          break;
        }
        if (m === n) continue;
        const rx = n.x - m.x, ry = n.y - m.y;
        const d = Math.sqrt(rx * rx + ry * ry) || 1;
        if (d < repulsionRadius) {
          const f = (k * k) / (d * d) * BRAIN_CONFIG.REPULSION_FORCE_SCALE;
          n.vx += (rx / d) * f;
          n.vy += (ry / d) * f;
        }
      }
      if (budgetExceeded) break;
    }
    // Spring force along synapses
    if (!budgetExceeded) {
      for (const syn of S.synapses) {
        if (performance.now() - frameStart > BRAIN_CONFIG.FRAME_BUDGET_MS) {
          budgetExceeded = true;
          break;
        }
        const a = S.neurons.get(syn.from), b = S.neurons.get(syn.to);
        if (!a || !b) continue;
        const dx = b.x - a.x, dy = b.y - a.y;
        const d = Math.sqrt(dx * dx + dy * dy) || 1;
        const ideal = (a.type === 'session' && b.type === 'session')
          ? k * BRAIN_CONFIG.SPRING_SESSION_MULTIPLIER
          : k * BRAIN_CONFIG.SPRING_DEFAULT_MULTIPLIER;
        const f = (d - ideal) * BRAIN_CONFIG.SPRING_FORCE;
        const fx = (dx / d) * f, fy = (dy / d) * f;
        a.vx += fx; a.vy += fy;
        b.vx -= fx; b.vy -= fy;
      }
    }
    // Integrate with damping + hard bounds
    const sf2 = scaleFactor();
    for (const n of nodes) {
      n.vx *= BRAIN_CONFIG.DAMPING; n.vy *= BRAIN_CONFIG.DAMPING;
      n.x += n.vx; n.y += n.vy;
      const margin = Math.max(BRAIN_CONFIG.BOUNDS_MARGIN_MIN, n.radius * sf2 + BRAIN_CONFIG.BOUNDS_MARGIN_PADDING);
      const xMin = margin, xMax = S.w - margin;
      const yMin = margin, yMax = S.h - margin;
      if (n.x < xMin) { n.vx += (xMin - n.x) * BRAIN_CONFIG.BOUNDS_REBOUND; n.x = xMin; }
      if (n.x > xMax) { n.vx -= (n.x - xMax) * BRAIN_CONFIG.BOUNDS_REBOUND; n.x = xMax; }
      if (n.y < yMin) { n.vy += (yMin - n.y) * BRAIN_CONFIG.BOUNDS_REBOUND; n.y = yMin; }
      if (n.y > yMax) { n.vy -= (n.y - yMax) * BRAIN_CONFIG.BOUNDS_REBOUND; n.y = yMax; }
    }
  }

  /* ─── Rendering ─── */
  function drawGrid(c) {
    c.strokeStyle = 'rgba(0,229,255,0.02)'; c.lineWidth = 0.5;
    for (let x = 0; x < S.w; x += BRAIN_CONFIG.GRID_SIZE) { c.beginPath(); c.moveTo(x, 0); c.lineTo(x, S.h); c.stroke(); }
    for (let y = 0; y < S.h; y += BRAIN_CONFIG.GRID_SIZE) { c.beginPath(); c.moveTo(0, y); c.lineTo(S.w, y); c.stroke(); }
  }

  function getCachedGradient(ctx, ax, ay, bx, by, colorA, colorB) {
    if (S.frameCount - S.gradientSweepFrame >= BRAIN_CONFIG.GRADIENT_CACHE_SWEEP_FRAMES) {
      _gradientCache.clear();
      S.gradientSweepFrame = S.frameCount;
    }
    const key = `${Math.round(ax)},${Math.round(ay)},${Math.round(bx)},${Math.round(by)},${colorA},${colorB}`;
    if (_gradientCache.size > BRAIN_CONFIG.GRADIENT_CACHE_MAX_SIZE) _gradientCache.clear();
    let grad = _gradientCache.get(key);
    if (!grad) {
      grad = ctx.createLinearGradient(ax, ay, bx, by);
      grad.addColorStop(0, colorA);
      grad.addColorStop(1, colorB);
      _gradientCache.set(key, grad);
    }
    return grad;
  }

  function drawSynapses(c, ts) {
    const mode = getBrainMode();
    const synapseScale = mode.synapseWidth / BRAIN_IMMERSIVE.synapseWidth;
    for (const syn of S.synapses) {
      const a = S.neurons.get(syn.from), b = S.neurons.get(syn.to);
      if (!a || !b || a.scale < 0.1 || b.scale < 0.1) continue;

      // Determine flow state from connected neurons — include 'running' for live sessions/agents
      const aActive = a.meta.status === 'in_progress' || a.meta.status === 'submitted' || a.meta.status === 'running';
      const bActive = b.meta.status === 'in_progress' || b.meta.status === 'submitted' || b.meta.status === 'running';
      const aDone = a.meta.status === 'done';
      const bDone = b.meta.status === 'done';
      const anyActive = aActive || bActive;
      const bothDone = aDone && bDone;

      // Continuous flow — always alive, intensity based on state
      if (anyActive) syn.flow(aActive && bActive ? BRAIN_CONFIG.FLOW_ACTIVE_FULL : BRAIN_CONFIG.FLOW_ACTIVE_PARTIAL);
      else if (bothDone) syn.flow(BRAIN_CONFIG.FLOW_DONE);
      else syn.flow(BRAIN_CONFIG.FLOW_IDLE_BASE);

      const age = ts - syn.lastFire;
      const fireGlow = age < BRAIN_CONFIG.FIRE_GLOW_DURATION_MS ? 1 - age / BRAIN_CONFIG.FIRE_GLOW_DURATION_MS : 0;

      // Synapse color — blend from source node color
      const srcPal = a.pal;
      const baseAlpha = anyActive ? BRAIN_CONFIG.SYNAPSE_ACTIVE_BASE_ALPHA + fireGlow * BRAIN_CONFIG.SYNAPSE_ACTIVE_GLOW_ALPHA
        : bothDone ? BRAIN_CONFIG.SYNAPSE_DONE_ALPHA
        : BRAIN_CONFIG.SYNAPSE_IDLE_BASE_ALPHA + syn.strength * BRAIN_CONFIG.SYNAPSE_IDLE_STRENGTH_ALPHA + fireGlow * BRAIN_CONFIG.SYNAPSE_IDLE_GLOW_ALPHA;

      // Curved path
      const curvature = BRAIN_CONFIG.SYNAPSE_CURVATURE_BASE + (anyActive ? BRAIN_CONFIG.SYNAPSE_CURVATURE_WAVE * Math.sin(ts * BRAIN_CONFIG.SYNAPSE_CURVATURE_SPEED) : 0);
      const mx = (a.x + b.x) / 2 + (a.y - b.y) * curvature;
      const my = (a.y + b.y) / 2 - (a.x - b.x) * curvature;

      // Draw main line — use source node color
      c.beginPath(); c.moveTo(a.x, a.y); c.quadraticCurveTo(mx, my, b.x, b.y);
      const lineWidth = anyActive
        ? BRAIN_CONFIG.SYNAPSE_WIDTH_ACTIVE + fireGlow * BRAIN_CONFIG.SYNAPSE_WIDTH_ACTIVE_GLOW
        : bothDone ? BRAIN_CONFIG.SYNAPSE_WIDTH_DONE : BRAIN_CONFIG.SYNAPSE_WIDTH_IDLE;
      c.lineWidth = lineWidth * synapseScale;

      if (anyActive || fireGlow > 0) {
        // Gradient along synapse — source color to dest color
        const startColor = cachedColor(srcPal.glow, baseAlpha);
        const endColor = cachedColor(b.pal.glow, baseAlpha * BRAIN_CONFIG.FLOW_ACTIVE_PARTIAL);
        c.strokeStyle = getCachedGradient(c, a.x, a.y, b.x, b.y, startColor, endColor);
        if (BRAIN_CONFIG.SHADOWS_ENABLED) {
          c.shadowBlur = BRAIN_CONFIG.SYNAPSE_SHADOW_BASE + fireGlow * BRAIN_CONFIG.SYNAPSE_SHADOW_GLOW;
          c.shadowColor = cachedColor(srcPal.glow, fireGlow * BRAIN_CONFIG.SYNAPSE_SHADOW_ALPHA);
        }
      } else {
        c.strokeStyle = cachedColor(srcPal.glow, baseAlpha);
        c.shadowBlur = 0;
      }
      c.stroke(); c.shadowBlur = 0;

      // Pulsing glow ring at midpoint for active connections
      if (anyActive && Math.sin(ts * BRAIN_CONFIG.PULSE_CHECK_SPEED + syn.strength * 10) > BRAIN_CONFIG.PULSE_THRESHOLD) {
        const pulseR = BRAIN_CONFIG.PULSE_RADIUS_BASE + Math.sin(ts * BRAIN_CONFIG.PULSE_WAVE_SPEED) * BRAIN_CONFIG.PULSE_RADIUS_WAVE;
        c.save(); c.globalAlpha = BRAIN_CONFIG.PULSE_ALPHA_BASE + fireGlow * BRAIN_CONFIG.PULSE_ALPHA_GLOW;
        c.fillStyle = srcPal.core;
        if (BRAIN_CONFIG.SHADOWS_ENABLED) {
          c.shadowBlur = BRAIN_CONFIG.PULSE_SHADOW_BLUR; c.shadowColor = srcPal.core;
        }
        c.beginPath(); c.arc(mx, my, pulseR, 0, PI2); c.fill();
        c.restore();
      }

      // Particles — with trails
      const headBuckets = [null, null, null, null, null];
      for (let i = syn.particles.length - 1; i >= 0; i--) {
        const p = syn.particles[i];
        p.t += p.speed * BRAIN_CONFIG.PARTICLE_DT;
        if (p.t >= 1) { syn.particles.splice(i, 1); continue; }
        const u = 1 - p.t;
        const px = u * u * a.x + 2 * u * p.t * mx + p.t * p.t * b.x;
        const py = u * u * a.y + 2 * u * p.t * my + p.t * p.t * b.y;

        // Circular buffer instead of shift()
        const ti = p.trailIdx;
        p.trail[ti] = { x: px, y: py };
        p.trailIdx = (ti + 1) % BRAIN_CONFIG.PARTICLE_TRAIL_MAX;
        if (p.trailLen < BRAIN_CONFIG.PARTICLE_TRAIL_MAX) p.trailLen++;

        // Draw trail
        if (p.trailLen > 1) {
          c.save();
          const tmax = BRAIN_CONFIG.PARTICLE_TRAIL_MAX;
          for (let t = 0; t < p.trailLen - 1; t++) {
            const ci = (p.trailIdx - p.trailLen + t + tmax * 2) % tmax;
            const ni = (ci + 1) % tmax;
            const pt0 = p.trail[ci];
            const pt1 = p.trail[ni];
            if (!pt0 || !pt1) continue;
            const trailAlpha = (t / p.trailLen) * BRAIN_CONFIG.PARTICLE_TRAIL_ALPHA;
            c.beginPath();
            c.moveTo(pt0.x, pt0.y);
            c.lineTo(pt1.x, pt1.y);
            c.strokeStyle = cachedColor(srcPal.glow, trailAlpha);
            c.lineWidth = p.size * (t / p.trailLen) * BRAIN_CONFIG.PARTICLE_TRAIL_WIDTH;
            c.stroke();
          }
          c.restore();
        }

        // Queue particle head by alpha bucket
        const alpha = p.t > BRAIN_CONFIG.PARTICLE_FADE_START
          ? (1 - p.t) / BRAIN_CONFIG.PARTICLE_FADE_RANGE
          : Math.min(1, p.t / BRAIN_CONFIG.PARTICLE_FADE_IN_RANGE);
        const bucket = Math.min(4, Math.max(0, (alpha * 5) | 0));
        if (!headBuckets[bucket]) headBuckets[bucket] = [];
        headBuckets[bucket].push({ x: px, y: py, size: p.size });
      }
      // Batch particle heads by alpha bucket
      for (let bkt = 0; bkt < headBuckets.length; bkt++) {
        const bucketItems = headBuckets[bkt];
        if (!bucketItems || bucketItems.length === 0) continue;
        c.save();
        c.globalAlpha = (bkt + 0.5) / 5;
        c.fillStyle = srcPal.core;
        if (BRAIN_CONFIG.SHADOWS_ENABLED) {
          c.shadowBlur = BRAIN_CONFIG.PULSE_SHADOW_BLUR; c.shadowColor = srcPal.core;
        }
        c.beginPath();
        for (let j = 0; j < bucketItems.length; j++) {
          const h = bucketItems[j];
          c.moveTo(h.x + h.size, h.y);
          c.arc(h.x, h.y, h.size, 0, PI2);
        }
        c.fill();
        c.restore();
      }
    }
  }

  function drawNeurons(c, ts) {
    const sf = scaleFactor();
    const mode = getBrainMode();
    const radiusScale = mode.nodeRadius / BRAIN_IMMERSIVE.nodeRadius;
    for (const [, n] of S.neurons) {
      // Animate scale
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
      const pal = n.pal;

      c.save();
      // Crisp solid fill — minimal glow
      if (BRAIN_CONFIG.SHADOWS_ENABLED && fireGlow > 0) {
        c.shadowBlur = BRAIN_CONFIG.NODE_SHADOW_BASE + fireGlow * BRAIN_CONFIG.NODE_SHADOW_GLOW;
        c.shadowColor = pal.core;
      } else {
        c.shadowBlur = 0;
      }
      // Solid gradient (not washed out)
      const g = c.createRadialGradient(
        n.x - r * BRAIN_CONFIG.NODE_FILL_OFFSET,
        n.y - r * BRAIN_CONFIG.NODE_FILL_OFFSET,
        r * BRAIN_CONFIG.NODE_FILL_INNER_RADIUS,
        n.x,
        n.y,
        r
      );
      g.addColorStop(0, cachedColor(pal.glow, BRAIN_CONFIG.NODE_FILL_ALPHA_INNER));
      g.addColorStop(BRAIN_CONFIG.FLOW_ACTIVE_PARTIAL, pal.core);
      g.addColorStop(1, cachedColor(pal.glow, BRAIN_CONFIG.NODE_FILL_ALPHA_OUTER));
      c.fillStyle = g;
      c.beginPath(); c.arc(n.x, n.y, r, 0, PI2); c.fill();
      // Crisp ring
      c.shadowBlur = 0;
      c.strokeStyle = cachedColor(
        pal.glow,
        BRAIN_CONFIG.NODE_RING_ALPHA_BASE + fireGlow * BRAIN_CONFIG.NODE_RING_ALPHA_GLOW + BRAIN_CONFIG.NODE_RING_ALPHA_WAVE * Math.sin(n.phase * BRAIN_CONFIG.NODE_RING_WAVE_SPEED)
      );
      c.lineWidth = n.type === 'session' ? BRAIN_CONFIG.SESSION_RING_WIDTH : BRAIN_CONFIG.DEFAULT_RING_WIDTH;
      c.beginPath(); c.arc(n.x, n.y, r + BRAIN_CONFIG.SESSION_RING_WIDTH, 0, PI2); c.stroke();
      c.restore();

      // Label
      if (mode.labelVisible && n.type === 'session') {
        const isHover = S.hover === n.id;
        const fsf = fontScale();
        const fs = Math.round(BRAIN_CONFIG.FONT_SIZE_FACTOR * fsf);
        c.font = `${isHover ? 'bold ' : ''}${fs}px "JetBrains Mono",monospace`;
        c.textAlign = 'center';
        const ly = n.y + r + BRAIN_CONFIG.SESSION_LABEL_OFFSET * fsf;
        const lbl = n.label;
        const tw = c.measureText(lbl).width;
        c.fillStyle = `rgba(10,16,36,${isHover ? BRAIN_CONFIG.SESSION_LABEL_BG_ALPHA_HOVER : BRAIN_CONFIG.SESSION_LABEL_BG_ALPHA})`;
        c.beginPath();
        const ph = fs + BRAIN_CONFIG.SESSION_LABEL_HEIGHT;
        if (c.roundRect) c.roundRect(n.x - tw / 2 - BRAIN_CONFIG.SESSION_LABEL_PADDING_X, ly - ph / 2 - BRAIN_CONFIG.SESSION_RING_WIDTH, tw + BRAIN_CONFIG.SESSION_LABEL_PADDING_X * 2, ph, BRAIN_CONFIG.SESSION_LABEL_RADIUS);
        else { c.rect(n.x - tw / 2 - BRAIN_CONFIG.SESSION_LABEL_PADDING_X, ly - ph / 2 - BRAIN_CONFIG.SESSION_RING_WIDTH, tw + BRAIN_CONFIG.SESSION_LABEL_PADDING_X * 2, ph); }
        c.fill();
        c.fillStyle = isHover ? '#fff' : '#b0c4dd';
        c.fillText(lbl, n.x, ly + BRAIN_CONFIG.TEXT_BASELINE_OFFSET);
        if (isHover && n.meta.tty) {
          const info = [n.meta.tty, `PID ${n.meta.pid || '?'}`,
            n.meta.cpu != null ? `CPU ${n.meta.cpu}%` : '',
            n.meta.mem != null ? `MEM ${n.meta.mem}%` : ''].filter(Boolean).join(' · ');
          c.font = `${Math.round(BRAIN_CONFIG.SESSION_INFO_FONT_SIZE * fsf)}px "JetBrains Mono",monospace`;
          c.fillStyle = PAL.sub.core;
          c.fillText(info, n.x, ly + BRAIN_CONFIG.SESSION_LABEL_OFFSET * fsf);
        }
      }
      // Plan/task labels with mesh node badge
      if (mode.labelVisible && (n.type === 'plan' || (n.type === 'task' && (S.hover === n.id || n.meta.status === 'in_progress')))) {
        const isHover = S.hover === n.id;
        const host = n.meta.executor_host || n.meta.host || '';
        const mp = meshColor(host || '?');
        const fsf = fontScale();
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
        c.fillStyle = isHover ? '#fff' : '#c8d0e8';
        c.fillText(lbl, n.x, ly + BRAIN_CONFIG.SESSION_RING_WIDTH);
        // Mesh node badge below — compact
        if (host) {
          const bfs = Math.round(BRAIN_CONFIG.BADGE_FONT_SIZE * fsf);
          c.font = `bold ${bfs}px "JetBrains Mono",monospace`;
          const badge = host;
          const bw = c.measureText(badge).width;
          const bx = n.x - bw / 2 - BRAIN_CONFIG.BADGE_PADDING_X, by = ly + BRAIN_CONFIG.BADGE_OFFSET_Y * fsf;
          const bh = bfs + BRAIN_CONFIG.BADGE_HEIGHT;
          c.fillStyle = `${mp.glow}0.25)`;
          c.beginPath();
          if (c.roundRect) c.roundRect(bx, by, bw + BRAIN_CONFIG.BADGE_PADDING_X * 2, bh, BRAIN_CONFIG.BADGE_RADIUS);
          else c.rect(bx, by, bw + BRAIN_CONFIG.BADGE_PADDING_X * 2, bh);
          c.fill();
          c.strokeStyle = `${mp.glow}0.5)`;
          c.lineWidth = BRAIN_CONFIG.BADGE_BORDER_WIDTH;
          c.stroke();
          c.fillStyle = mp.core;
          c.fillText(badge, n.x, by + bfs - 1);
        }
      }
    }
  }

  function scheduleFrame() {
    if (S.raf) return;
    S.raf = requestAnimationFrame(render);
  }

  function render(ts) {
    S.raf = 0;
    if (!S.ctx || !S.running) return;
    try {
      S.lastTs = ts;
      S.frameCount++;
      // Frame budget: skip render if previous frame took too long
      if (S.frameCount > 10) {
        const dt = ts - (S._prevTs || 0);
        if (dt > 0 && dt < 12) { // Running fast, render normally
          S._skipNext = false;
        } else if (dt > 25) { // Slow frame, skip next render
          S._skipNext = !S._skipNext;
          if (S._skipNext) { scheduleFrame(); return; }
        }
      }
      S._prevTs = ts;
      applyForces();
      const mode = getBrainMode();
      applyBrainMode(mode);
      // Ambient synapse firing — continuous per-frame for always-alive feel
      const synapsePool = mode.fireEffects === 'perimeter'
        ? S.synapses.filter((syn) => {
            const a = S.neurons.get(syn.from);
            const b = S.neurons.get(syn.to);
            return a?.type === 'session' || b?.type === 'session';
          })
        : S.synapses;
      // Throttled ambient fire — max 1 per 3 frames
      if (synapsePool.length && S.frameCount % 3 === 0 && Math.random() < BRAIN_CONFIG.AMBIENT_FIRE_CHANCE) {
        const syn = synapsePool[Math.floor(Math.random() * synapsePool.length)];
        syn.fire(); const t = S.neurons.get(syn.to); if (t) t.fire();
      }

      if (S.webglRenderer) {
        S.webglRenderer.render(S.neurons, S.synapses, ts, BRAIN_CONFIG);
        const c = S.ctx;
        c.clearRect(0, 0, S.w, S.h);
        drawTooltip(c);
      } else {
        const c = S.ctx;
        c.clearRect(0, 0, S.w, S.h);
        drawGrid(c);
        drawSynapses(c, ts);
        drawNeurons(c, ts);
        drawTooltip(c);
      }
    } catch (e) {
      console.warn('[brain] render error (loop continues):', e);
    }
    scheduleFrame();
  }

  /* ─── Data → Graph sync ─── */
  function parseMeta(s) { try { return typeof s === 'string' ? JSON.parse(s) : (s || {}); } catch { return {}; } }
  // Tool-agnostic: extract tool key from agent_type or session_id
  // Known: "claude-cli" → "claude", "copilot-cli" → "copilot", "opencode" → "opencode"
  // Future tools auto-detected from type field
  function toolOf(id, type) {
    const t = (type || id || '').toLowerCase();
    if (t.includes('copilot')) return 'copilot';
    if (t.includes('opencode')) return 'opencode';
    if (t.includes('claude')) return 'claude';
    // Unknown tool: extract first word before "-cli" or "-agent"
    const m = t.match(/^([a-z]+)(?:-cli|-agent)?/);
    return m ? m[1] : 'unknown';
  }
  // Human-readable tool display name (capitalized)
  function toolDisplayName(toolKey) {
    const names = { claude: 'Claude', copilot: 'Copilot', opencode: 'OpenCode' };
    return names[toolKey] || (toolKey.charAt(0).toUpperCase() + toolKey.slice(1));
  }
  function fmtDur(s) { if (!s || s < 0) return ''; if (s < BRAIN_CONFIG.MINUTE_SECONDS) return `${Math.round(s)}s`; if (s < BRAIN_CONFIG.HOUR_SECONDS) return `${Math.round(s / BRAIN_CONFIG.MINUTE_SECONDS)}m`; return `${(s / BRAIN_CONFIG.HOUR_SECONDS).toFixed(1)}h`; }
  // Shorten model IDs to human-friendly names. Tool-agnostic — works with any provider.
  // "claude-opus-4-6" → "opus", "gpt-5.3-codex" → "codex", "deepseek-r1" → "r1"
  function shortModel(m) {
    if (!m) return '';
    const lo = m.toLowerCase();
    // Skip if model IS the tool type (session-scanner fallback)
    if (lo === 'claude-cli' || lo === 'copilot-cli' || lo === 'opencode') return '';
    // Anthropic models
    if (lo.includes('opus')) return 'opus';
    if (lo.includes('sonnet')) return 'sonnet';
    if (lo.includes('haiku')) return 'haiku';
    // OpenAI models
    if (lo.includes('codex')) return 'codex';
    if (/gpt-5/.test(lo)) return 'gpt5';
    if (/gpt-4/.test(lo)) return 'gpt4';
    if (/\bo[34]-/.test(lo) || /\bo[34]$/.test(lo)) return lo.match(/o[34][^ ]*/)?.[0] || 'o';
    // DeepSeek, Gemini, Llama, Mistral, Ollama local models
    if (lo.includes('deepseek')) return lo.replace(/.*deepseek-?/, 'ds-').substring(0, 8);
    if (lo.includes('gemini')) return lo.replace(/.*gemini-?/, 'gem-').substring(0, 8);
    if (lo.includes('llama')) return lo.replace(/.*llama-?/, 'llama').substring(0, 8);
    if (lo.includes('mistral')) return lo.replace(/.*mistral-?/, 'mist-').substring(0, 8);
    if (lo.includes('qwen')) return lo.replace(/.*qwen-?/, 'qwen').substring(0, 8);
    // Generic: strip common prefixes, keep short
    return m.replace(/^(claude-|gpt-|copilot-|opencode-)/i, '').substring(0, 12);
  }
  // Build a rich label: "Claude opus · Planning" or "Copilot codex · T3-01"
  // Tool-agnostic: works for claude, copilot, opencode, or any future CLI
  function richSessionLabel(tool, tty, model, description) {
    const name = toolDisplayName(tool);
    const sm = shortModel(model);
    const desc = (description || '').substring(0, BRAIN_CONFIG.SESSION_DESC_MAX).trim();
    if (desc && sm) return `${name} ${sm} · ${desc}`;
    if (desc) return `${name} · ${desc}`;
    if (sm && sm !== 'cli') return `${name} ${sm}`;
    if (tty) return `${name} ${tty}`;
    return name;
  }
  // Sub-agent label: prefer description, then short model
  function richChildLabel(model, description, type) {
    const desc = (description || '').substring(0, BRAIN_CONFIG.CHILD_DESC_MAX).trim();
    const sm = shortModel(model);
    if (desc && sm) return `${sm}: ${desc}`;
    if (desc) return desc;
    if (sm && sm !== 'cli') return sm;
    return type || 'agent';
  }
  function fmtTok(n) { if (!n) return '0'; if (n > BRAIN_CONFIG.TOKENS_MEGA) return `${(n / BRAIN_CONFIG.TOKENS_MEGA).toFixed(1)}M`; if (n > BRAIN_CONFIG.TOKENS_KILO) return `${(n / BRAIN_CONFIG.TOKENS_KILO).toFixed(1)}k`; return String(n); }

  function syncGraph() {
    const now = performance.now();
    const active = S.sessions.filter(s => s.status === 'running');
    const activeIds = new Set();

    // Session neurons (radial layout around center)
    for (let i = 0; i < active.length; i++) {
      const sess = active[i];
      const meta = parseMeta(sess.metadata);
      const tool = toolOf(sess.session_id, sess.type);
      const tty = meta.tty || '';
      const label = richSessionLabel(tool, tty, sess.model, sess.description);
      activeIds.add(sess.session_id);

      // Enrich meta with API data
      meta.status = sess.status; // propagate session status for synapse flow
      meta.duration_s = sess.duration_s; meta.tokens_total = sess.tokens_total;
      meta.tokens_in = sess.tokens_in; meta.tokens_out = sess.tokens_out;
      meta.cost_usd = sess.cost_usd; meta.model = sess.model;
      meta.description = sess.description; meta.started_at = sess.started_at;

      if (!S.neurons.has(sess.session_id)) {
        const n = new Neuron(sess.session_id, 'session', label, meta);
        n.tool = tool;
        // Radial initial position
        const angle = (i / Math.max(active.length, 1)) * PI2 - Math.PI / 2;
        const radius = Math.min(S.w, S.h) * BRAIN_CONFIG.SESSION_ORBIT_RADIUS;
        n.x = S.w / 2 + Math.cos(angle) * radius;
        n.y = S.h / 2 + Math.sin(angle) * radius;
        n.targetRadius = radius; n.targetAngle = angle;
        S.neurons.set(sess.session_id, n);
        // Connect to same-tool sessions only (Claude↔Claude, Copilot↔Copilot) — not all-to-all
        for (const [oid, other] of S.neurons) {
          if (oid !== sess.session_id && other.type === 'session' && !other.dying && other.tool === tool) {
            S.synapses.push(new Synapse(sess.session_id, oid));
          }
        }
        setTimeout(() => { const nn = S.neurons.get(sess.session_id); if (nn) { nn.fire(); fireSynapsesFor(sess.session_id); } }, BRAIN_CONFIG.SESSION_FIRE_DELAY_MS);
      } else {
        const n = S.neurons.get(sess.session_id);
        n.label = label; n.meta = meta; n.active = true;
      }

      // Sub-agent neurons
      for (const child of (sess.children || [])) {
        if (child.status !== 'running') continue;
        activeIds.add(child.agent_id);
        if (!S.neurons.has(child.agent_id)) {
          const cn = new Neuron(child.agent_id, 'sub', richChildLabel(child.model, child.description, child.type), {
            status: child.status, model: child.model, description: child.description,
            duration_s: child.duration_s, tokens_total: child.tokens_total, cost_usd: child.cost_usd
          });
          cn.tool = tool;
          const parent = S.neurons.get(sess.session_id);
          cn.x = (parent?.x || S.w / 2) + (Math.random() - 0.5) * BRAIN_CONFIG.SESSION_SPAWN_JITTER;
          cn.y = (parent?.y || S.h / 2) + (Math.random() - 0.5) * BRAIN_CONFIG.SESSION_SPAWN_JITTER;
          S.neurons.set(child.agent_id, cn);
          S.synapses.push(new Synapse(sess.session_id, child.agent_id));
          setTimeout(() => fireSynapsesFor(child.agent_id), BRAIN_CONFIG.CHILD_FIRE_DELAY_MS);
        } else {
          const existing = S.neurons.get(child.agent_id);
          existing.label = richChildLabel(child.model, child.description, child.type);
          existing.meta = { status: child.status, model: child.model, description: child.description, duration_s: child.duration_s, tokens_total: child.tokens_total };
        }
      }
    }

    // Plan neurons — radius proportional to sqrt(task count), colored by mesh node
    for (const plan of (S.brainData?.plans || [])) {
      const pid = `plan-${plan.id}`;
      activeIds.add(pid);
      const tc = plan.tasks_total || 1;
      // Dramatic sizing: sqrt scale, 14 base → up to 36 for 10+ tasks
      const planRadius = Math.max(BRAIN_CONFIG.PLAN_RADIUS_MIN,
        Math.min(BRAIN_CONFIG.PLAN_RADIUS_MAX, BRAIN_CONFIG.PLAN_RADIUS_BASE + Math.sqrt(tc) * BRAIN_CONFIG.PLAN_RADIUS_SCALE));
      if (!S.neurons.has(pid)) {
        const pn = new Neuron(pid, 'plan', `#${plan.id} ${(plan.name || '').substring(0, BRAIN_CONFIG.PLAN_NAME_MAX)}`, {
          name: plan.name, status: plan.status, progress: plan.progress_pct,
          tasks_done: plan.tasks_done, tasks_total: plan.tasks_total,
          host: plan.execution_host, executor_host: plan.execution_host
        });
        pn.tool = 'claude';
        pn.radius = planRadius;
        // Distribute plans in an orbital ring around center
        const planIdx = (S.brainData?.plans || []).indexOf(plan);
        const totalPlans = (S.brainData?.plans || []).length;
        const angle = (planIdx / Math.max(1, totalPlans)) * PI2 + Math.random() * BRAIN_CONFIG.PLAN_ORBIT_JITTER_ANGLE;
        const orbitX = S.w * BRAIN_CONFIG.PLAN_ORBIT_FACTOR;
        const orbitY = S.h * BRAIN_CONFIG.PLAN_ORBIT_FACTOR;
        pn.x = S.w / 2 + Math.cos(angle) * orbitX + (Math.random() - 0.5) * BRAIN_CONFIG.PLAN_ORBIT_JITTER_POS;
        pn.y = S.h / 2 + Math.sin(angle) * orbitY + (Math.random() - 0.5) * BRAIN_CONFIG.PLAN_ORBIT_JITTER_POS;
        S.neurons.set(pid, pn);
      } else {
        const en = S.neurons.get(pid);
        en.meta = { name: plan.name, progress: plan.progress_pct, tasks_done: plan.tasks_done, tasks_total: plan.tasks_total, host: plan.execution_host, executor_host: plan.execution_host };
        en.radius = planRadius;
      }
    }

    // Task neurons — radius proportional to lines changed, colored by executor_host
    const waveGroups = {};
    for (const task of (S.brainData?.tasks || [])) {
      const tid = `task-${task.id}`;
      activeIds.add(tid);
      const planNid = `plan-${task.plan_id}`;
      const waveKey = `${task.plan_id}-${task.wave_id || 'W0'}`;
      if (!waveGroups[waveKey]) waveGroups[waveKey] = [];
      waveGroups[waveKey].push(tid);
      let linesAdded = 0;
      try { const od = typeof task.output_data === 'string' ? JSON.parse(task.output_data) : task.output_data; linesAdded = od?.lines_added || 0; } catch {}
      // Radius driven by lines: pending=3, active=sqrt(lines)*0.8, min 5, max 22
      const dynRadius = linesAdded > 0
        ? Math.max(BRAIN_CONFIG.TASK_LINES_RADIUS_MIN,
          Math.min(BRAIN_CONFIG.TASK_LINES_RADIUS_MAX, BRAIN_CONFIG.TASK_RADIUS_BASE + Math.sqrt(linesAdded) * BRAIN_CONFIG.TASK_RADIUS_SCALE))
        : Math.max(BRAIN_CONFIG.TASK_LINES_RADIUS_MIN,
          Math.min(BRAIN_CONFIG.TASK_FALLBACK_RADIUS_MAX, BRAIN_CONFIG.TASK_RADIUS_BASE + (task.tokens || 0) / BRAIN_CONFIG.TASK_TOKEN_DIVISOR));
      const taskRadius = task.status === 'pending' ? BRAIN_CONFIG.TASK_PENDING_RADIUS : dynRadius;
      if (!S.neurons.has(tid)) {
        const tn = new Neuron(tid, 'task', (task.title || '').substring(0, BRAIN_CONFIG.TASK_LABEL_MAX), {
          title: task.title, status: task.status, priority: task.priority,
          type: task.task_type, plan_name: task.plan_name, wave_name: task.wave_id || task.wave_name,
          executor_host: task.executor_host, model: task.model,
          tokens: task.tokens, lines_added: linesAdded
        });
        tn.tool = 'claude';
        tn.radius = taskRadius;
        // Tasks orbit their parent plan like moons
        const planN = S.neurons.get(planNid);
        const tAngle = Math.random() * PI2;
        const tDist = BRAIN_CONFIG.TASK_ORBIT_MIN + Math.random() * BRAIN_CONFIG.TASK_ORBIT_RANGE;
        tn.x = (planN?.x || S.w / 2) + Math.cos(tAngle) * tDist;
        tn.y = (planN?.y || S.h / 2) + Math.sin(tAngle) * tDist;
        S.neurons.set(tid, tn);
        if (S.neurons.has(planNid)) S.synapses.push(new Synapse(planNid, tid));
        if (task.executor_session_id && S.neurons.has(task.executor_session_id)) {
          S.synapses.push(new Synapse(task.executor_session_id, tid));
        }
      } else {
        const en = S.neurons.get(tid);
        en.meta = { ...en.meta, status: task.status, executor_host: task.executor_host, model: task.model, tokens: task.tokens, lines_added: linesAdded };
        en.radius = taskRadius;
        if (en.meta._prevStatus && en.meta._prevStatus !== task.status) {
          en.fire(); fireSynapsesFor(tid);
        }
        en.meta._prevStatus = task.status;
      }
    }
    // Connect tasks in same wave (task relationships)
    for (const [, group] of Object.entries(waveGroups)) {
      if (group.length < 2) continue;
      for (let i = 1; i < group.length; i++) {
        const a = group[i - 1], b = group[i];
        if (!S.synapses.find(s => (s.from === a && s.to === b) || (s.from === b && s.to === a))) {
          const syn = new Synapse(a, b);
          syn.strength = BRAIN_CONFIG.WAVE_LINK_STRENGTH;
          S.synapses.push(syn);
        }
      }
    }

    // Mark dead + cleanup
    for (const [id, n] of S.neurons) {
      if (!activeIds.has(id) && !n.dying) { n.dying = true; n.deathT = now; }
    }
    for (const [id, n] of S.neurons) {
      if (n.dying && n.scale <= 0) { S.neurons.delete(id); S.synapses = S.synapses.filter(s => s.from !== id && s.to !== id); }
    }

    // Ambient synapse firing — now in render loop for continuous activity
    updateStats();
  }

  function fireSynapsesFor(id) {
    for (const syn of S.synapses) {
      if (syn.from === id || syn.to === id) syn.fire();
    }
  }

  function updateStats() {
    const el = document.getElementById('brain-stats');
    if (!el) return;
    const running = S.sessions.filter(s => s.status === 'running');
    // Dynamic tool counts: {claude: 2, copilot: 3, opencode: 1} → "2C/3P/1O"
    const toolCounts = {};
    running.forEach(s => { const t = toolOf(s.session_id, s.type); toolCounts[t] = (toolCounts[t] || 0) + 1; });
    const toolStr = Object.entries(toolCounts).map(([t, n]) => `${n}${toolDisplayName(t).charAt(0)}`).join('/');
    const plans = (S.brainData?.plans || []).length;
    const tasks = (S.brainData?.tasks || []).length;
    const syns = S.synapses.length;
    el.textContent = `${running.length} sessions · ${toolStr || '0'} · ${plans} plans · ${tasks} tasks · ${syns} synapses`;
  }

  /* ─── Hover tooltip (summary) ─── */
  function drawTooltip(c) {
    if (!S.hover || !getBrainMode().labelVisible) return;
    const n = S.neurons.get(S.hover);
    if (!n) return;
    const m = n.meta || {};
    const lines = [];
    if (n.type === 'session') {
      lines.push(n.label);
      if (m.tty) lines.push(`TTY ${m.tty} · PID ${m.pid || '?'}`);
      if (m.cpu != null) lines.push(`CPU ${m.cpu}% · MEM ${m.mem || 0}%`);
      if (m.duration_s) lines.push(`Duration: ${fmtDur(m.duration_s)}`);
      if (m.tokens_total) lines.push(`Tokens: ${fmtTok(m.tokens_total)}`);
      // Show model unless it's just the tool type (e.g. "claude-cli", "copilot-cli")
      if (m.model && !m.model.endsWith('-cli') && m.model !== n.meta?.agent_type) lines.push(`Model: ${m.model}`);
      if (m.description && m.description.trim().length > 2 && !m.description.includes('/bin/')) {
        lines.push(`Task: ${m.description.trim().substring(0, 80)}`);
      }
      if (m.cwd && m.cwd !== 'unknown') {
        const proj = m.cwd.split('/').pop();
        if (proj) lines.push(`Dir: ${proj}`);
      }
    } else if (n.type === 'plan') {
      lines.push(m.name || n.label);
      lines.push(`Progress: ${m.tasks_done || 0}/${m.tasks_total || 0} (${m.progress || 0}%)`);
      if (m.host) lines.push(`Node: ${m.host}`);
    } else if (n.type === 'task') {
      lines.push(m.title || n.label);
      lines.push(`Status: ${m.status || '?'}${m.priority ? ' · ' + m.priority : ''}`);
      if (m.executor_host) lines.push(`Node: ${m.executor_host}`);
      if (m.model) lines.push(`Model: ${m.model}`);
      if (m.tokens) lines.push(`Tokens: ${fmtTok(m.tokens)}`);
      if (m.lines_added) lines.push(`Lines: +${m.lines_added}`);
      if (m.wave_name) lines.push(`Wave: ${m.wave_name}`);
      if (m.plan_name) lines.push(`Plan: ${m.plan_name.substring(0, 50)}`);
    } else {
      // Sub-agents and other neurons
      lines.push(n.label);
      if (m.model && !m.model.endsWith('-cli')) lines.push(`Model: ${m.model}`);
      if (m.duration_s) lines.push(`Duration: ${fmtDur(m.duration_s)}`);
      if (m.tokens_total) lines.push(`Tokens: ${fmtTok(m.tokens_total)}`);
      if (m.cost_usd) lines.push(`Cost: $${Number(m.cost_usd).toFixed(4)}`);
      if (m.description && m.description !== n.label) lines.push(m.description.substring(0, 80));
    }

    const lh = BRAIN_CONFIG.TOOLTIP_LINE_HEIGHT, pad = BRAIN_CONFIG.TOOLTIP_PADDING;
    const maxW = Math.max(...lines.map(l => c.measureText ? BRAIN_CONFIG.TOOLTIP_TEXT_WIDTH_FACTOR * l.length : BRAIN_CONFIG.TOOLTIP_FALLBACK_WIDTH));
    const tw = Math.min(BRAIN_CONFIG.TOOLTIP_MAX_WIDTH, maxW + pad * 2);
    const th = lines.length * lh + pad * 2;
    let tx = n.x + n.radius + BRAIN_CONFIG.TOOLTIP_OFFSET, ty = n.y - th / 2;
    if (tx + tw > S.w - BRAIN_CONFIG.TOOLTIP_EDGE_MARGIN) tx = n.x - n.radius - tw - BRAIN_CONFIG.TOOLTIP_OFFSET;
    if (ty < BRAIN_CONFIG.TOOLTIP_EDGE_MARGIN) ty = BRAIN_CONFIG.TOOLTIP_EDGE_MARGIN;
    if (ty + th > S.h - BRAIN_CONFIG.TOOLTIP_EDGE_MARGIN) ty = S.h - th - BRAIN_CONFIG.TOOLTIP_EDGE_MARGIN;

    c.save();
    c.fillStyle = 'rgba(10,10,10,0.88)';
    c.strokeStyle = `${n.pal.glow}0.3)`;
    c.lineWidth = 1; c.shadowBlur = 4; c.shadowColor = `${n.pal.glow}0.15)`;
    c.beginPath();
    if (c.roundRect) c.roundRect(tx, ty, tw, th, 6);
    else c.rect(tx, ty, tw, th);
    c.fill(); c.stroke(); c.shadowBlur = 0;
    c.font = 'calc(13px * var(--mn-a11y-font-scale, 1)) "Barlow Condensed","JetBrains Mono",monospace'; c.textAlign = 'left';
    lines.forEach((line, i) => {
      c.fillStyle = i === 0 ? '#fff' : (line.startsWith('Cmd:') ? PAL.sub.core : '#8899bb');
      c.fillText(line, tx + pad, ty + pad + (i + 1) * lh - 3);
    });
    c.restore();
  }

  /* ─── Click detail panel ─── */
  function showDetailPanel(id) {
    const n = S.neurons.get(id);
    if (!n) return;
    let panel = document.getElementById('brain-detail');
    if (!panel) {
      panel = document.createElement('div'); panel.id = 'brain-detail';
      panel.style.cssText = 'position:absolute;right:12px;top:12px;width:380px;max-height:560px;overflow-y:auto;background:var(--bg-card, #111);border:1px solid var(--border, #2a2a2a);border-radius:var(--radius-md, 12px);padding:16px;z-index:10;font:13px var(--font-mono, "Barlow Condensed", monospace);color:var(--text, #9e9e9e);backdrop-filter:blur(12px);';
      S.container.appendChild(panel);
    }
    const m = n.meta || {};
    let html = `<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:10px"><span style="font:bold 14px 'JetBrains Mono',monospace;color:${n.pal.core}">${n.label}</span><span style="cursor:pointer;color:#5a6080;font-size:16px" onclick="this.parentElement.parentElement.remove()">✕</span></div>`;
    const row = (k, v) => v ? `<div style="display:flex;justify-content:space-between;padding:2px 0"><span style="color:#5a6080">${k}</span><span style="color:#e0e4f0">${v}</span></div>` : '';
    if (n.type === 'session') {
      html += row('PID', m.pid); html += row('TTY', m.tty);
      html += row('CPU', m.cpu != null ? m.cpu + '%' : ''); html += row('MEM', m.mem != null ? m.mem + '%' : '');
      html += row('Duration', fmtDur(m.duration_s)); html += row('Model', m.model);
      html += row('Tokens In', fmtTok(m.tokens_in)); html += row('Tokens Out', fmtTok(m.tokens_out));
      html += row('Total Tok', fmtTok(m.tokens_total)); html += row('Cost', m.cost_usd ? '$' + Number(m.cost_usd).toFixed(4) : '');
      html += row('Started', m.started_at || '');
      if (m.description && m.description.trim().length > 2) {
        html += `<div style="margin-top:8px;padding-top:8px;border-top:1px solid #1a2040"><span style="color:#5a6080">Last command</span><div style="color:#00e5ff;margin-top:4px;word-break:break-all">${esc(m.description.trim().substring(0, 120))}</div></div>`;
      }
      // Show related recent agents
      const children = (S.sessions.find(s => s.session_id === id)?.children || []);
      if (children.length) {
        html += `<div style="margin-top:8px;padding-top:8px;border-top:1px solid #1a2040"><span style="color:#5a6080">Sub-agents (${children.length})</span>`;
        children.forEach(ch => {
          const st = ch.status === 'running' ? '\u25CF' : '\u25CB';
          const chLabel = richChildLabel(ch.model, ch.description, ch.type);
          html += `<div style="padding:2px 0">${st} ${chLabel.substring(0, 28)} ${fmtDur(ch.duration_s)} ${fmtTok(ch.tokens_total)}tok</div>`;
        });
        html += '</div>';
      }
      // Show recent completed agents for this session
      const recent = (S.brainData?.recent || []).filter(r => r.parent_session === id).slice(0, 5);
      if (recent.length) {
        html += `<div style="margin-top:8px;padding-top:8px;border-top:1px solid #1a2040"><span style="color:#5a6080">Recent completed</span>`;
        recent.forEach(r => {
          const ok = r.status === 'completed' ? '✓' : '✗';
          const rLabel = richChildLabel(r.model, r.description, r.type);
          html += `<div style="padding:2px 0;color:${r.status === 'completed' ? '#00ff88' : '#ff3366'}">${ok} ${rLabel.substring(0, 24)} ${fmtDur(r.duration_s)} ${fmtTok(r.tokens_total)}tok</div>`;
        });
        html += '</div>';
      }
    } else if (n.type === 'plan') {
      html += row('Status', m.status); html += row('Node', m.host || m.executor_host);
      html += row('Progress', `${m.tasks_done || 0}/${m.tasks_total || 0}`);
      html += `<div style="margin:6px 0;height:4px;background:#1a2040;border-radius:2px"><div style="height:100%;width:${m.progress || 0}%;background:linear-gradient(90deg,${meshColor(m.host || m.executor_host).core},#00ff88);border-radius:2px"></div></div>`;
      const planTasks = (S.brainData?.tasks || []).filter(t => t.plan_id === m.id || t.plan_id == n.id.replace('plan-',''));
      if (planTasks.length) {
        html += `<div style="margin-top:8px;border-top:1px solid #1a2040;padding-top:8px"><span style="color:#5a6080">Tasks</span>`;
        planTasks.forEach(t => {
          const dot = t.status === 'in_progress' ? '\u25CF' : '\u25CB';
          html += `<div style="padding:2px 0">${dot} ${(t.title || '').substring(0, 35)} <span style="color:#5a6080">${t.priority || ''}</span></div>`;
        });
        html += '</div>';
      }
    } else if (n.type === 'task') {
      html += row('Status', m.status); html += row('Node', m.executor_host);
      html += row('Model', m.model); html += row('Priority', m.priority); html += row('Type', m.type);
      html += row('Tokens', fmtTok(m.tokens)); html += row('Lines', m.lines_added ? `+${m.lines_added}` : '');
      html += row('Wave', m.wave_name); html += row('Plan', m.plan_name);
      if (m.title) html += `<div style="margin-top:6px;color:#e0e4f0">${m.title}</div>`;
    } else {
      html += row('Model', m.model); html += row('Duration', fmtDur(m.duration_s));
      html += row('Tokens', fmtTok(m.tokens_total));
      if (m.description) html += `<div style="margin-top:6px;color:#00e5ff;word-break:break-all">${esc(m.description.substring(0, 120))}</div>`;
    }
    panel.innerHTML = html;
  }

  /* ─── Maranello neuralNodes sync ─── */
  function syncMaranelloNeuralNodes(sessions, agents) {
    // Use the data-driven API (v4.5.0) — pass real session data to Maranello
    if (window.MaranelloEnhancer?.syncBrainData) {
      window.MaranelloEnhancer.syncBrainData(sessions, agents);
    }
    // Keep custom canvas always visible — Maranello neuralNodes provides ambient bg only
    if (S.canvas) S.canvas.style.opacity = '1';
  }

  /* ─── Polling ─── */
  function pollData() {
    fetch('/api/brain').then(r => r.json()).then(data => {
      S.brainData = data;
      const sessions = data.sessions || [];
      const agents = data.agents || [];
      const childMap = new Map();
      agents.forEach(a => {
        if (a.parent_session) {
          if (!childMap.has(a.parent_session)) childMap.set(a.parent_session, []);
          childMap.get(a.parent_session).push(a);
        }
      });
      S.sessions = sessions.map(s => ({
        session_id: s.agent_id, type: s.type || 'claude-cli', status: s.status,
        metadata: s.metadata, description: s.description, started_at: s.started_at,
        duration_s: s.duration_s, tokens_total: s.tokens_total, tokens_in: s.tokens_in,
        tokens_out: s.tokens_out, cost_usd: s.cost_usd, model: s.model,
        children: (childMap.get(s.agent_id) || []).map(c => ({
          agent_id: c.agent_id, type: c.type, model: c.model, description: c.description,
          status: c.status || 'running', duration_s: c.duration_s, tokens_total: c.tokens_total, cost_usd: c.cost_usd
        }))
      }));
      S.agents = agents;
      window._dashboardAgentData = { sessions: S.sessions, orphan_agents: [] };
      syncGraph();
      scheduleFrame();
      syncMaranelloNeuralNodes(sessions, agents);
    }).catch(() => {
      // Fallback to old endpoints
      Promise.all([
        fetch('/api/sessions').then(r => r.json()).catch(() => []),
        fetch('/api/agents').then(r => r.json()).catch(() => ({ running: [] }))
      ]).then(([rawSessions, agentData]) => {
        const running = agentData.running || [];
        const childMap = new Map();
        running.forEach(a => { if (a.parent_session) { if (!childMap.has(a.parent_session)) childMap.set(a.parent_session, []); childMap.get(a.parent_session).push(a); } });
        S.sessions = (rawSessions || []).map(s => ({
          session_id: s.agent_id, type: s.type, status: s.status, metadata: s.metadata,
          description: s.description, duration_s: s.duration_s, tokens_total: s.tokens_total, model: s.model,
          children: (childMap.get(s.agent_id) || []).map(c => ({ agent_id: c.agent_id, type: c.type, model: c.model, description: c.description, status: c.status || 'running', duration_s: c.duration_s }))
        }));
        syncGraph();
        scheduleFrame();
      });
    });
  }

  /* ─── Mouse / Touch interaction ─── */
  function canvasXY(e) {
    // body.zoom compensation: offsetX/Y are in viewport pixels, canvas uses CSS pixels
    const zoom = parseFloat(document.body.style.zoom) || 1;
    if (e.touches) {
      // Touch events don't have offsetX/Y — fall back to clientX with rect
      const rect = S.canvas.getBoundingClientRect();
      const cx = (e.touches[0].clientX - rect.left) / zoom;
      const cy = (e.touches[0].clientY - rect.top) / zoom;
      return { x: cx * S.w / (rect.width / zoom), y: cy * S.h / (rect.height / zoom) };
    }
    return { x: e.offsetX / zoom, y: e.offsetY / zoom };
  }
  function hitTest(x, y) {
    const mode = getBrainMode();
    const radiusScale = mode.nodeRadius / BRAIN_IMMERSIVE.nodeRadius;
    for (const [id, n] of S.neurons) {
      if (n.dying) continue;
      const dx = x - n.x, dy = y - n.y;
      const hitRadius = (n.type === 'session' ? n.radius : n.radius * radiusScale) + BRAIN_CONFIG.HIT_PADDING;
      if (dx * dx + dy * dy < hitRadius * hitRadius) return id;
    }
    return null;
  }
  function onMouseMove(e) {
    const p = canvasXY(e);
    S.mouse.x = p.x; S.mouse.y = p.y;
    S.hover = hitTest(p.x, p.y);
    S.canvas.style.cursor = S.hover ? 'pointer' : 'default';
  }
  function onMouseLeave() { S.hover = null; S.mouse.x = -1; S.mouse.y = -1; }
  function onClick(e) {
    const p = canvasXY(e);
    const hit = hitTest(p.x, p.y);
    if (hit) { fireSynapsesFor(hit); const n = S.neurons.get(hit); if (n) n.fire(); }
  }
  function onTouchStart(e) {
    if (!e.touches || !e.touches.length) return;
    const p = canvasXY(e);
    const hit = hitTest(p.x, p.y);
    S.hover = hit;
    if (hit) { fireSynapsesFor(hit); const n = S.neurons.get(hit); if (n) n.fire(); e.preventDefault(); }
  }
  function onTouchEnd() { setTimeout(() => { S.hover = null; }, BRAIN_CONFIG.TOUCH_CLEAR_DELAY_MS); }

  /* ─── Resize ─── */
  function resize() {
    if (!S.container || !S.canvas) return;
    applyBrainMode(getBrainMode());
    S.dpr = window.devicePixelRatio || 1;
    S.w = Math.max(BRAIN_CONFIG.CANVAS_MIN_SIZE, S.container.clientWidth);
    S.h = Math.max(BRAIN_CONFIG.CANVAS_MIN_SIZE, S.container.clientHeight);
    S.canvas.width = Math.floor(S.w * S.dpr);
    S.canvas.height = Math.floor(S.h * S.dpr);
    S.canvas.style.width = S.w + 'px';
    S.canvas.style.height = S.h + 'px';
    S.ctx.setTransform(S.dpr, 0, 0, S.dpr, 0, 0);
    if (S.webglRenderer) S.webglRenderer.resize(S.w, S.h);
  }

  /* ─── WebSocket ─── */
  const wsUrl = () => `${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.host}/ws/brain`;
  function connectWs() {
    try { S.ws = new WebSocket(wsUrl()); } catch { S.ws = null; }
    if (!S.ws) return;
    S.ws.onopen = () => { S.wsRetry = 0; };
    S.ws.onmessage = () => pollData();
    S.ws.onerror = () => S.ws?.close();
    S.ws.onclose = () => {
      clearTimeout(S.wsT);
      S.wsT = setTimeout(connectWs,
        Math.min(BRAIN_CONFIG.WS_RETRY_MAX_MS, BRAIN_CONFIG.WS_RETRY_BASE_MS * Math.pow(BRAIN_CONFIG.WS_RETRY_EXP_BASE, S.wsRetry++)));
    };
  }

  /* ─── ⓘ Help/Legend toggle ─── */
  function addHelpButton() {
    const controls = document.getElementById('brain-controls');
    if (!controls || document.getElementById('brain-help-btn')) return;
    const btn = document.createElement('button');
    btn.id = 'brain-help-btn'; btn.className = 'brain-ctrl-btn'; btn.title = 'Legend';
    btn.innerHTML = '<svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="8" cy="8" r="6.5"/><path d="M6.5 6.2a1.8 1.8 0 0 1 3.3.9c0 1.2-1.8 1-1.8 2.4M8 11.5v.01"/></svg>';
    btn.onclick = function() {
      var leg = document.getElementById('brain-legend');
      if (leg) { leg.remove(); return; }
      leg = document.createElement('div'); leg.id = 'brain-legend';
      leg.style.cssText = 'position:absolute;left:12px;bottom:12px;background:var(--bg-card, #111);border:1px solid var(--border, #2a2a2a);border-radius:var(--radius-md, 12px);padding:10px 14px;z-index:15;font:calc(9px * var(--mn-a11y-font-scale, 1)) var(--font-mono, "Barlow Condensed", monospace);color:var(--text-dim, #616161);backdrop-filter:blur(10px);line-height:1.8;cursor:pointer;';
      leg.onclick = function() { leg.remove(); };
      leg.innerHTML = '<span style="color:var(--text, #9e9e9e);font-size:10px;letter-spacing:1px">NEURAL GRAPH</span><br><br>'
        + '<span style="color:var(--accent, #FFC72C)">●</span> Claude &nbsp; <span style="color:var(--info, #4EA8DE)">●</span> Copilot &nbsp; <span style="color:var(--success, #00A651)">●</span> OpenCode &nbsp; <span style="color:var(--accent, #FFC72C)">●</span> Sub-agent<br>'
        + '<span style="color:var(--success, #00A651)">●</span> Plan &nbsp; <span style="color:var(--accent, #FFC72C)">●</span> Task &nbsp; <span style="color:var(--text-dim)">(other tools auto-colored)</span><br><br>'
        + '<span style="color:var(--text, #9e9e9e)">Brightness</span> = CPU activity<br>'
        + '<span style="color:var(--text, #9e9e9e)">Size</span> = sub-agents count<br>'
        + '<span style="color:var(--text, #9e9e9e)">Lines</span> = synapses<br><br>'
        + '<span style="color:var(--text-dim)">Hover → details | Tap on mobile</span>';
      S.container.appendChild(leg);
    };
    controls.insertBefore(btn, controls.firstChild);
  }

  /* ─── Lifecycle ─── */
  function onVis() {
    S.running = !document.hidden;
    if (document.hidden) {
      if (S.pollT) { clearInterval(S.pollT); S.pollT = 0; }
    } else if (!S.pollT) {
      S.pollT = setInterval(pollData, BRAIN_CONFIG.POLL_INTERVAL_MS);
    }
    if (S.running) scheduleFrame();
    if (!S.running && S.raf) { cancelAnimationFrame(S.raf); S.raf = 0; }
  }
  window.initBrainCanvas = function(id) {
    window.destroyBrainCanvas();
    S.container = document.getElementById(id || 'brain-canvas-container');
    if (!S.container) return;
    _gradientCache.clear();
    S.frameCount = 0;
    S.gradientSweepFrame = 0;
    S.running = true;
    S.canvas = document.createElement('canvas');
    // z-index:2 — data layer on top of Maranello neuralNodes ambient (z-index:1)
    S.canvas.style.cssText = 'display:block;width:100%;height:100%;border-radius:var(--radius-md, 12px);position:relative;z-index:2;';
    S.canvas.setAttribute('role', 'img');
    S.canvas.setAttribute('aria-label', BRAIN_CONFIG.CANVAS_ARIA_LABEL);
    S.container.appendChild(S.canvas);
    S.ctx = S.canvas.getContext('2d', { alpha: true }); resize();
    if (typeof BrainWebGLRenderer !== 'undefined') {
      try {
        S.webglRenderer = new BrainWebGLRenderer(S.container);
        S.canvas.style.opacity = '0';
        S.canvas.style.background = 'transparent';
        console.log('[brain] WebGL renderer active');
      } catch (e) {
        S.webglRenderer = null;
        S.canvas.style.opacity = '1';
        console.warn('[brain] WebGL failed, using Canvas2D:', e.message);
      }
    } else {
      S.canvas.style.opacity = '1';
    }
    S.ro = new ResizeObserver(resize); S.ro.observe(S.container);
    S.canvas.addEventListener('mousemove', onMouseMove);
    S.canvas.addEventListener('mouseleave', onMouseLeave);
    S.canvas.addEventListener('click', onClick);
    S.canvas.addEventListener('touchstart', onTouchStart, { passive: false });
    S.canvas.addEventListener('touchend', onTouchEnd);
    document.addEventListener('visibilitychange', onVis);
    pollData();
    connectWs(); addHelpButton();
    scheduleFrame();
    onVis();
  };
  window.destroyBrainCanvas = function() {
    if (S.raf) cancelAnimationFrame(S.raf); S.raf = 0;
    if (S.ro) S.ro.disconnect(); S.ro = null;
    if (S.ws) S.ws.close(); S.ws = null; clearTimeout(S.wsT); S.wsT = 0;
    if (S.pollT) clearInterval(S.pollT); S.pollT = 0;
    if (S.canvas) {
      S.canvas.removeEventListener('mousemove', onMouseMove);
      S.canvas.removeEventListener('mouseleave', onMouseLeave);
      S.canvas.removeEventListener('click', onClick);
      S.canvas.removeEventListener('touchstart', onTouchStart);
      S.canvas.removeEventListener('touchend', onTouchEnd);
    }
    if (S.webglRenderer) {
      S.webglRenderer.destroy();
      S.webglRenderer = null;
    }
    document.removeEventListener('visibilitychange', onVis);
    if (S.container) S.container.innerHTML = '';
    S.container = S.canvas = S.ctx = S.webglRenderer = null;
    _gradientCache.clear();
    S.frameCount = 0; S.gradientSweepFrame = 0;
    S.neurons.clear(); S.synapses = []; S.sessions = []; S.agents = [];
  };
  window.updateBrainData = function() { pollData(); };
  window.toggleBrainFreeze = function() {
    S.running = !S.running;
    const btn = document.getElementById('brain-pause-btn');
    if (btn) btn.innerHTML = S.running ? (window.Icons ? Icons.pause(14) : '\u23F8') : (window.Icons ? Icons.start(14) : '\u25B6');
    if (S.running) scheduleFrame();
    else if (S.raf) { cancelAnimationFrame(S.raf); S.raf = 0; }
  };
  window.rewindBrain = function() { S.neurons.clear(); S.synapses = []; pollData(); };
  window.resizeBrainCanvas = function() { resize(); };
  window.brainResize = function() {
    resize();
    scheduleFrame();
  };
  window.brainExpandTo = function(targetContainer) {
    const container = document.getElementById('brain-canvas-container') || S.container;
    if (!container) return;
    const target = typeof targetContainer === 'string'
      ? document.getElementById(targetContainer) || document.querySelector(targetContainer)
      : targetContainer;
    if (!target) return;
    if (container.parentElement !== target) target.appendChild(container);
    S.container = container;
    if (S.ro) {
      S.ro.disconnect();
      S.ro.observe(S.container);
    }
    resize();
    scheduleFrame();
  };

  const _boot = () => window.initBrainCanvas('brain-canvas-container');
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', _boot);
  else setTimeout(_boot, BRAIN_CONFIG.BOOT_DELAY_MS);
})();
