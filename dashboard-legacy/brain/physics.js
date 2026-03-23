/* brain/physics.js — Force-directed layout: repulsion, attraction, centering */
'use strict';

import { BRAIN_CONFIG } from './config.js';

class SpatialHash {
  constructor(cellSize) {
    this.cellSize = Math.max(1, cellSize);
    this.grid = new Map();
  }
  clear() { this.grid.clear(); }
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

/**
 * Run one physics tick. Tries WASM bridge first, falls back to JS.
 * @param {Map} neurons — id -> Neuron
 * @param {Array} synapses
 * @param {number} w — canvas width
 * @param {number} h — canvas height
 * @param {{forceTick: number}} tick — mutable counter object
 */
export function updatePhysics(neurons, synapses, w, h, tick) {
  // WASM bridge — delegate if available
  if (typeof window.brainWasmStep === 'function' && window.brainWasmReady && window.brainWasmReady()) {
    const wasmHandled = window.brainWasmStep(neurons, synapses, w, h, BRAIN_CONFIG);
    if (wasmHandled) return;
  }

  const nodes = [...neurons.values()].filter(n => !n.dying);
  const cx = w / 2, cy = h / 2;
  const sf = _scaleFactor(w, h);
  const frameStart = performance.now();
  const nodeCount = nodes.length;

  // Adaptive repulsion — shrink when many nodes to fit canvas
  const densityFactor = nodeCount > BRAIN_CONFIG.DENSITY_THRESHOLD
    ? Math.max(BRAIN_CONFIG.DENSITY_MIN_FACTOR, BRAIN_CONFIG.DENSITY_THRESHOLD / nodeCount)
    : 1;
  const k = BRAIN_CONFIG.REPULSION_K * sf * densityFactor;
  const repulsionRadius = k * BRAIN_CONFIG.REPULSION_RADIUS_MULTIPLIER;

  const spatialHash = new SpatialHash(repulsionRadius);
  for (const node of nodes) spatialHash.insert(node);

  const throttleStride = nodeCount > BRAIN_CONFIG.NODE_THROTTLE_THRESHOLD ? BRAIN_CONFIG.NODE_THROTTLE_STRIDE : 1;
  const runRepulsion = throttleStride === 1 || (++tick.forceTick % throttleStride) === 0;
  let budgetExceeded = false;

  const spreadX = w * BRAIN_CONFIG.SPREAD_FACTOR;
  const spreadY = h * BRAIN_CONFIG.SPREAD_FACTOR;

  for (const n of nodes) {
    if (performance.now() - frameStart > BRAIN_CONFIG.FRAME_BUDGET_MS) { budgetExceeded = true; break; }
    // Elliptical gravity
    const dx = cx - n.x, dy = cy - n.y;
    const normDist = Math.sqrt((dx * dx) / (spreadX * spreadX) + (dy * dy) / (spreadY * spreadY));
    const grav = normDist > 1 ? BRAIN_CONFIG.GRAVITY_STRONG : BRAIN_CONFIG.GRAVITY_WEAK;
    n.vx += dx * grav;
    n.vy += dy * grav;

    if (!runRepulsion) continue;
    for (const m of spatialHash.getNearby(n.x, n.y, repulsionRadius)) {
      if (performance.now() - frameStart > BRAIN_CONFIG.FRAME_BUDGET_MS) { budgetExceeded = true; break; }
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
    for (const syn of synapses) {
      if (performance.now() - frameStart > BRAIN_CONFIG.FRAME_BUDGET_MS) break;
      const a = neurons.get(syn.from), b = neurons.get(syn.to);
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
  const sf2 = _scaleFactor(w, h);
  for (const n of nodes) {
    n.vx *= BRAIN_CONFIG.DAMPING; n.vy *= BRAIN_CONFIG.DAMPING;
    n.x += n.vx; n.y += n.vy;
    const margin = Math.max(BRAIN_CONFIG.BOUNDS_MARGIN_MIN, n.radius * sf2 + BRAIN_CONFIG.BOUNDS_MARGIN_PADDING);
    const xMin = margin, xMax = w - margin;
    const yMin = margin, yMax = h - margin;
    if (n.x < xMin) { n.vx += (xMin - n.x) * BRAIN_CONFIG.BOUNDS_REBOUND; n.x = xMin; }
    if (n.x > xMax) { n.vx -= (n.x - xMax) * BRAIN_CONFIG.BOUNDS_REBOUND; n.x = xMax; }
    if (n.y < yMin) { n.vy += (yMin - n.y) * BRAIN_CONFIG.BOUNDS_REBOUND; n.y = yMin; }
    if (n.y > yMax) { n.vy -= (n.y - yMax) * BRAIN_CONFIG.BOUNDS_REBOUND; n.y = yMax; }
  }
}

function _scaleFactor(w, h) {
  return Math.sqrt((w * h) / BRAIN_CONFIG.CANVAS_REF_AREA);
}
