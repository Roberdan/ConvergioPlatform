/* brain/effects.js — Particles, glow, fire, synapse rendering */
'use strict';

import {
  BRAIN_CONFIG, BRAIN_IMMERSIVE, PI2,
  cachedColor, gradientCache
} from './config.js';

const _prefersReducedMotion = () =>
  window.matchMedia('(prefers-reduced-motion: reduce)').matches;

/** Synapse with particle system */
export class Synapse {
  constructor(from, to) {
    this.from = from; this.to = to;
    this.particles = []; this.lastFire = 0;
    this.strength = BRAIN_CONFIG.DEFAULT_SYNAPSE_STRENGTH;
    this.flowRate = 0;
  }
  fire(getBrainMode) {
    this.lastFire = performance.now();
    if (_prefersReducedMotion()) return;
    const mode = getBrainMode();
    const modeMax = Math.max(8, Math.floor(BRAIN_CONFIG.MAX_PARTICLES * mode.particleCountScale));
    const fireCount = Math.max(1, Math.floor(
      (BRAIN_CONFIG.PARTICLE_FIRE_BASE + Math.floor(Math.random() * BRAIN_CONFIG.PARTICLE_FIRE_VARIANCE)) * mode.particleCountScale));
    const count = Math.min(modeMax - this.particles.length, fireCount);
    for (let i = 0; i < count; i++) {
      this.particles.push(_newParticle(
        BRAIN_CONFIG.PARTICLE_FIRE_SPEED_MIN, BRAIN_CONFIG.PARTICLE_FIRE_SPEED_RANGE,
        BRAIN_CONFIG.PARTICLE_FIRE_SIZE_MIN, BRAIN_CONFIG.PARTICLE_FIRE_SIZE_RANGE));
    }
  }
  flow(rate, getBrainMode) {
    this.flowRate = rate;
    if (_prefersReducedMotion()) return;
    const mode = getBrainMode();
    const modeMax = Math.max(8, Math.floor(BRAIN_CONFIG.MAX_PARTICLES * mode.particleCountScale));
    const spawnRate = rate * BRAIN_CONFIG.PARTICLE_FLOW_SPAWN_RATE * mode.particleCountScale;
    if (rate > 0 && this.particles.length < modeMax && Math.random() < spawnRate) {
      this.particles.push(_newParticle(
        BRAIN_CONFIG.PARTICLE_FLOW_SPEED_MIN, BRAIN_CONFIG.PARTICLE_FLOW_SPEED_RANGE,
        BRAIN_CONFIG.PARTICLE_FLOW_SIZE_MIN, BRAIN_CONFIG.PARTICLE_FLOW_SIZE_RANGE));
    }
  }
}

function _newParticle(speedMin, speedRange, sizeMin, sizeRange) {
  return {
    t: 0,
    speed: speedMin + Math.random() * speedRange,
    size: sizeMin + Math.random() * sizeRange,
    trail: new Array(BRAIN_CONFIG.PARTICLE_TRAIL_MAX),
    trailIdx: 0, trailLen: 0
  };
}

// Gradient cache with periodic sweep
function getCachedGradient(ctx, ax, ay, bx, by, colorA, colorB, frameCount, sweepFrame) {
  if (frameCount - sweepFrame.v >= BRAIN_CONFIG.GRADIENT_CACHE_SWEEP_FRAMES) {
    gradientCache.clear(); sweepFrame.v = frameCount;
  }
  const key = `${Math.round(ax)},${Math.round(ay)},${Math.round(bx)},${Math.round(by)},${colorA},${colorB}`;
  if (gradientCache.size > BRAIN_CONFIG.GRADIENT_CACHE_MAX_SIZE) gradientCache.clear();
  let grad = gradientCache.get(key);
  if (!grad) {
    grad = ctx.createLinearGradient(ax, ay, bx, by);
    grad.addColorStop(0, colorA); grad.addColorStop(1, colorB);
    gradientCache.set(key, grad);
  }
  return grad;
}

/** Draw the grid background */
export function drawGrid(c, w, h) {
  c.strokeStyle = 'rgba(0,229,255,0.02)'; c.lineWidth = 0.5;
  for (let x = 0; x < w; x += BRAIN_CONFIG.GRID_SIZE) { c.beginPath(); c.moveTo(x, 0); c.lineTo(x, h); c.stroke(); }
  for (let y = 0; y < h; y += BRAIN_CONFIG.GRID_SIZE) { c.beginPath(); c.moveTo(0, y); c.lineTo(w, y); c.stroke(); }
}

/**
 * Render all synapse lines + particles.
 * @param {CanvasRenderingContext2D} c
 * @param {Array<Synapse>} synapses
 * @param {Map} neurons
 * @param {number} ts
 * @param {Function} getBrainMode
 * @param {{frameCount: number, sweepFrame: {v: number}}} fc
 */
export function renderEffects(c, synapses, neurons, ts, getBrainMode, fc) {
  const mode = getBrainMode();
  const synapseScale = mode.synapseWidth / BRAIN_IMMERSIVE.synapseWidth;
  const reducedMotion = _prefersReducedMotion();

  for (const syn of synapses) {
    const a = neurons.get(syn.from), b = neurons.get(syn.to);
    if (!a || !b || a.scale < 0.1 || b.scale < 0.1) continue;

    const aActive = a.meta.status === 'in_progress' || a.meta.status === 'submitted' || a.meta.status === 'running';
    const bActive = b.meta.status === 'in_progress' || b.meta.status === 'submitted' || b.meta.status === 'running';
    const aDone = a.meta.status === 'done'; const bDone = b.meta.status === 'done';
    const anyActive = aActive || bActive; const bothDone = aDone && bDone;

    if (anyActive) syn.flow(aActive && bActive ? BRAIN_CONFIG.FLOW_ACTIVE_FULL : BRAIN_CONFIG.FLOW_ACTIVE_PARTIAL, getBrainMode);
    else if (bothDone) syn.flow(BRAIN_CONFIG.FLOW_DONE, getBrainMode);
    else syn.flow(BRAIN_CONFIG.FLOW_IDLE_BASE, getBrainMode);

    const age = ts - syn.lastFire;
    const fireGlow = age < BRAIN_CONFIG.FIRE_GLOW_DURATION_MS ? 1 - age / BRAIN_CONFIG.FIRE_GLOW_DURATION_MS : 0;
    const srcPal = a.pal;
    const baseAlpha = anyActive
      ? BRAIN_CONFIG.SYNAPSE_ACTIVE_BASE_ALPHA + fireGlow * BRAIN_CONFIG.SYNAPSE_ACTIVE_GLOW_ALPHA
      : bothDone ? BRAIN_CONFIG.SYNAPSE_DONE_ALPHA
      : BRAIN_CONFIG.SYNAPSE_IDLE_BASE_ALPHA + syn.strength * BRAIN_CONFIG.SYNAPSE_IDLE_STRENGTH_ALPHA + fireGlow * BRAIN_CONFIG.SYNAPSE_IDLE_GLOW_ALPHA;

    const curvature = BRAIN_CONFIG.SYNAPSE_CURVATURE_BASE + (anyActive ? BRAIN_CONFIG.SYNAPSE_CURVATURE_WAVE * Math.sin(ts * BRAIN_CONFIG.SYNAPSE_CURVATURE_SPEED) : 0);
    const mx = (a.x + b.x) / 2 + (a.y - b.y) * curvature;
    const my = (a.y + b.y) / 2 - (a.x - b.x) * curvature;

    c.beginPath(); c.moveTo(a.x, a.y); c.quadraticCurveTo(mx, my, b.x, b.y);
    const lineWidth = anyActive
      ? BRAIN_CONFIG.SYNAPSE_WIDTH_ACTIVE + fireGlow * BRAIN_CONFIG.SYNAPSE_WIDTH_ACTIVE_GLOW
      : bothDone ? BRAIN_CONFIG.SYNAPSE_WIDTH_DONE : BRAIN_CONFIG.SYNAPSE_WIDTH_IDLE;
    c.lineWidth = lineWidth * synapseScale;

    if (anyActive || fireGlow > 0) {
      const startColor = cachedColor(srcPal.glow, baseAlpha);
      const endColor = cachedColor(b.pal.glow, baseAlpha * BRAIN_CONFIG.FLOW_ACTIVE_PARTIAL);
      c.strokeStyle = getCachedGradient(c, a.x, a.y, b.x, b.y, startColor, endColor, fc.frameCount, fc.sweepFrame);
      if (BRAIN_CONFIG.SHADOWS_ENABLED) {
        c.shadowBlur = BRAIN_CONFIG.SYNAPSE_SHADOW_BASE + fireGlow * BRAIN_CONFIG.SYNAPSE_SHADOW_GLOW;
        c.shadowColor = cachedColor(srcPal.glow, fireGlow * BRAIN_CONFIG.SYNAPSE_SHADOW_ALPHA);
      }
    } else {
      c.strokeStyle = cachedColor(srcPal.glow, baseAlpha); c.shadowBlur = 0;
    }
    c.stroke(); c.shadowBlur = 0;

    // Pulse ring at midpoint
    if (!reducedMotion && anyActive && Math.sin(ts * BRAIN_CONFIG.PULSE_CHECK_SPEED + syn.strength * 10) > BRAIN_CONFIG.PULSE_THRESHOLD) {
      const pulseR = BRAIN_CONFIG.PULSE_RADIUS_BASE + Math.sin(ts * BRAIN_CONFIG.PULSE_WAVE_SPEED) * BRAIN_CONFIG.PULSE_RADIUS_WAVE;
      c.save(); c.globalAlpha = BRAIN_CONFIG.PULSE_ALPHA_BASE + fireGlow * BRAIN_CONFIG.PULSE_ALPHA_GLOW;
      c.fillStyle = srcPal.core;
      if (BRAIN_CONFIG.SHADOWS_ENABLED) { c.shadowBlur = BRAIN_CONFIG.PULSE_SHADOW_BLUR; c.shadowColor = srcPal.core; }
      c.beginPath(); c.arc(mx, my, pulseR, 0, PI2); c.fill(); c.restore();
    }

    // Particles — skip if reduced motion
    if (reducedMotion) { syn.particles.length = 0; continue; }
    _drawParticles(c, syn, a, b, mx, my, srcPal);
  }
}

function _drawParticles(c, syn, a, b, mx, my, srcPal) {
  const headBuckets = [null, null, null, null, null];
  for (let i = syn.particles.length - 1; i >= 0; i--) {
    const p = syn.particles[i];
    p.t += p.speed * BRAIN_CONFIG.PARTICLE_DT;
    if (p.t >= 1) { syn.particles.splice(i, 1); continue; }
    const u = 1 - p.t;
    const px = u * u * a.x + 2 * u * p.t * mx + p.t * p.t * b.x;
    const py = u * u * a.y + 2 * u * p.t * my + p.t * p.t * b.y;

    const ti = p.trailIdx;
    p.trail[ti] = { x: px, y: py };
    p.trailIdx = (ti + 1) % BRAIN_CONFIG.PARTICLE_TRAIL_MAX;
    if (p.trailLen < BRAIN_CONFIG.PARTICLE_TRAIL_MAX) p.trailLen++;

    if (p.trailLen > 1) {
      c.save();
      const tmax = BRAIN_CONFIG.PARTICLE_TRAIL_MAX;
      for (let t = 0; t < p.trailLen - 1; t++) {
        const ci = (p.trailIdx - p.trailLen + t + tmax * 2) % tmax;
        const ni = (ci + 1) % tmax;
        const pt0 = p.trail[ci], pt1 = p.trail[ni];
        if (!pt0 || !pt1) continue;
        const trailAlpha = (t / p.trailLen) * BRAIN_CONFIG.PARTICLE_TRAIL_ALPHA;
        c.beginPath(); c.moveTo(pt0.x, pt0.y); c.lineTo(pt1.x, pt1.y);
        c.strokeStyle = cachedColor(srcPal.glow, trailAlpha);
        c.lineWidth = p.size * (t / p.trailLen) * BRAIN_CONFIG.PARTICLE_TRAIL_WIDTH;
        c.stroke();
      }
      c.restore();
    }

    const alpha = p.t > BRAIN_CONFIG.PARTICLE_FADE_START
      ? (1 - p.t) / BRAIN_CONFIG.PARTICLE_FADE_RANGE
      : Math.min(1, p.t / BRAIN_CONFIG.PARTICLE_FADE_IN_RANGE);
    const bucket = Math.min(4, Math.max(0, (alpha * 5) | 0));
    if (!headBuckets[bucket]) headBuckets[bucket] = [];
    headBuckets[bucket].push({ x: px, y: py, size: p.size });
  }

  for (let bkt = 0; bkt < headBuckets.length; bkt++) {
    const items = headBuckets[bkt];
    if (!items || items.length === 0) continue;
    c.save(); c.globalAlpha = (bkt + 0.5) / 5; c.fillStyle = srcPal.core;
    if (BRAIN_CONFIG.SHADOWS_ENABLED) { c.shadowBlur = BRAIN_CONFIG.PULSE_SHADOW_BLUR; c.shadowColor = srcPal.core; }
    c.beginPath();
    for (let j = 0; j < items.length; j++) {
      const h = items[j]; c.moveTo(h.x + h.size, h.y); c.arc(h.x, h.y, h.size, 0, PI2);
    }
    c.fill(); c.restore();
  }
}
