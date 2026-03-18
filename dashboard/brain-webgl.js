/* brain-webgl.js — PixiJS WebGL renderer for brain graph */
(() => {
  'use strict';

  const PI2 = Math.PI * 2;

  function parseColor(input, fallback = 0x00e5ff) {
    if (!input || typeof input !== 'string') return fallback;
    const v = input.trim();
    if (v.startsWith('#')) {
      const hex = v.slice(1);
      const full = hex.length === 3 ? hex.split('').map(ch => ch + ch).join('') : hex.slice(0, 6);
      const n = Number.parseInt(full, 16);
      return Number.isFinite(n) ? n : fallback;
    }
    const rgb = v.match(/rgba?\((\d+)\s*,\s*(\d+)\s*,\s*(\d+)/i);
    if (rgb) {
      const r = Math.max(0, Math.min(255, Number(rgb[1])));
      const g = Math.max(0, Math.min(255, Number(rgb[2])));
      const b = Math.max(0, Math.min(255, Number(rgb[3])));
      return (r << 16) | (g << 8) | b;
    }
    const hsl = v.match(/hsl\(([-\d.]+)\s*,\s*([-\d.]+)%\s*,\s*([-\d.]+)%\)/i);
    if (hsl) {
      const h = ((Number(hsl[1]) % 360) + 360) % 360;
      const s = Math.max(0, Math.min(1, Number(hsl[2]) / 100));
      const l = Math.max(0, Math.min(1, Number(hsl[3]) / 100));
      const c = (1 - Math.abs(2 * l - 1)) * s;
      const hp = h / 60;
      const x = c * (1 - Math.abs((hp % 2) - 1));
      let r1 = 0, g1 = 0, b1 = 0;
      if (hp < 1) [r1, g1, b1] = [c, x, 0];
      else if (hp < 2) [r1, g1, b1] = [x, c, 0];
      else if (hp < 3) [r1, g1, b1] = [0, c, x];
      else if (hp < 4) [r1, g1, b1] = [0, x, c];
      else if (hp < 5) [r1, g1, b1] = [x, 0, c];
      else [r1, g1, b1] = [c, 0, x];
      const m = l - c / 2;
      const r = Math.round((r1 + m) * 255);
      const g = Math.round((g1 + m) * 255);
      const b = Math.round((b1 + m) * 255);
      return (r << 16) | (g << 8) | b;
    }
    return fallback;
  }

  function modeForContainer(container) {
    return container && container.offsetWidth > 500
      ? { nodeRadius: 14, synapseWidth: 2, particleCountScale: 1, labelVisible: true }
      : { nodeRadius: 6, synapseWidth: 0.8, particleCountScale: 0.3, labelVisible: false };
  }

  class BrainWebGLRenderer {
    constructor(container) {
      if (typeof PIXI === 'undefined' || !container) throw new Error('PixiJS unavailable');
      const probe = document.createElement('canvas');
      if (!probe.getContext('webgl2') && !probe.getContext('webgl')) throw new Error('WebGL unavailable');
      this.container = container;
      this.nodeCache = new Map();
      this.synGlowCache = new Map();
      this.labelCache = new Map();
      this.nodeTextures = new Map();
      this.particleSprites = [];
      this.w = Math.max(10, container.clientWidth || 10);
      this.h = Math.max(10, container.clientHeight || 10);

      const appOptions = {
        width: this.w,
        height: this.h,
        antialias: true,
        preference: 'webgl',
        backgroundAlpha: 0,
        autoDensity: true,
        resolution: Math.min(2, window.devicePixelRatio || 1),
        resizeTo: container
      };
      this.app = new PIXI.Application(appOptions);
      this.canvas = this.app.canvas || this.app.view;
      if (!this.canvas && typeof this.app.init === 'function') {
        throw new Error('PixiJS renderer initialization failed');
      }
      this.canvas.style.cssText = 'display:block;width:100%;height:100%;border-radius:var(--radius-md, 12px);position:absolute;inset:0;z-index:1;pointer-events:none;';
      container.style.position = container.style.position || 'relative';
      container.insertBefore(this.canvas, container.firstChild);

      this.grid = new PIXI.Graphics();
      this.synapsesGfx = new PIXI.Graphics();
      this.trailsGfx = new PIXI.Graphics();
      this.nodeRingGfx = new PIXI.Graphics();
      this.glowGfx = new PIXI.Graphics();
      this.nodeContainer = new PIXI.Container();
      this.labelContainer = new PIXI.Container();
      this.particleContainer = new PIXI.ParticleContainer(4000, {
        position: true,
        scale: true,
        alpha: true,
        tint: true
      });

      this.app.stage.addChild(this.grid);
      this.app.stage.addChild(this.synapsesGfx);
      this.app.stage.addChild(this.trailsGfx);
      this.app.stage.addChild(this.glowGfx);
      this.app.stage.addChild(this.nodeContainer);
      this.app.stage.addChild(this.nodeRingGfx);
      this.app.stage.addChild(this.particleContainer);
      this.app.stage.addChild(this.labelContainer);

      this.particleTexture = this._createCircleTexture(12, 0xffffff, 1);
      this._rebuildGrid();
    }

    _createCircleTexture(size, color, alpha) {
      const g = new PIXI.Graphics();
      g.circle(size, size, size).fill({ color, alpha });
      const tex = this.app.renderer.generateTexture(g, {
        resolution: 1,
        region: new PIXI.Rectangle(0, 0, size * 2, size * 2)
      });
      g.destroy();
      return tex;
    }

    _nodeTexture(colorHex) {
      if (this.nodeTextures.has(colorHex)) return this.nodeTextures.get(colorHex);
      const size = 64;
      const g = new PIXI.Graphics();
      g.circle(size / 2, size / 2, size / 2).fill({ color: colorHex, alpha: 0.92 });
      g.circle(size * 0.42, size * 0.42, size * 0.22).fill({ color: 0xffffff, alpha: 0.2 });
      const tex = this.app.renderer.generateTexture(g, {
        resolution: 1,
        region: new PIXI.Rectangle(0, 0, size, size)
      });
      g.destroy();
      this.nodeTextures.set(colorHex, tex);
      return tex;
    }

    _labelFor(n) {
      let item = this.labelCache.get(n.id);
      if (!item) {
        const t = new PIXI.Text({
          text: n.label || '',
          style: {
            fontFamily: '"JetBrains Mono", monospace',
            fontSize: 11,
            fill: 0xc8d0e8,
            align: 'center'
          }
        });
        t.anchor.set(0.5, 0);
        this.labelContainer.addChild(t);
        item = t;
        this.labelCache.set(n.id, t);
      }
      return item;
    }

    _spriteForNode(n) {
      let sprite = this.nodeCache.get(n.id);
      if (!sprite) {
        sprite = new PIXI.Sprite(this._nodeTexture(parseColor(n.pal?.core, 0x00e5ff)));
        sprite.anchor.set(0.5);
        this.nodeContainer.addChild(sprite);
        this.nodeCache.set(n.id, sprite);
      }
      return sprite;
    }

    _synGlowSprite(syn, colorHex) {
      const key = `${syn.from}->${syn.to}`;
      let s = this.synGlowCache.get(key);
      if (!s) {
        s = new PIXI.Sprite(this._createCircleTexture(8, colorHex, 0.8));
        s.anchor.set(0.5);
        this.app.stage.addChild(s);
        this.synGlowCache.set(key, s);
      }
      return s;
    }

    _syncParticleCount(required) {
      while (this.particleSprites.length < required) {
        const sprite = new PIXI.Sprite(this.particleTexture);
        sprite.anchor.set(0.5);
        this.particleContainer.addChild(sprite);
        this.particleSprites.push(sprite);
      }
      for (let i = required; i < this.particleSprites.length; i++) {
        this.particleSprites[i].alpha = 0;
      }
    }

    _rebuildGrid() {
      const size = 50;
      this.grid.clear();
      this.grid.setStrokeStyle({ width: 1, color: 0x00e5ff, alpha: 0.03 });
      for (let x = 0; x < this.w; x += size) this.grid.moveTo(x, 0).lineTo(x, this.h);
      for (let y = 0; y < this.h; y += size) this.grid.moveTo(0, y).lineTo(this.w, y);
      this.grid.stroke();
    }

    render(neurons, synapses, ts, config) {
      if (!this.app?.renderer || !neurons) return;
      const mode = modeForContainer(this.container);
      this.synapsesGfx.clear();
      this.trailsGfx.clear();
      this.nodeRingGfx.clear();
      this.glowGfx.clear();

      let particleCount = 0;

      for (const syn of synapses) {
        const a = neurons.get(syn.from);
        const b = neurons.get(syn.to);
        if (!a || !b || a.scale < 0.1 || b.scale < 0.1) continue;
        const aActive = a.meta?.status === 'in_progress' || a.meta?.status === 'submitted' || a.meta?.status === 'running';
        const bActive = b.meta?.status === 'in_progress' || b.meta?.status === 'submitted' || b.meta?.status === 'running';
        const bothDone = a.meta?.status === 'done' && b.meta?.status === 'done';
        const anyActive = aActive || bActive;

        const age = ts - syn.lastFire;
        const fireGlow = age < config.FIRE_GLOW_DURATION_MS ? 1 - age / config.FIRE_GLOW_DURATION_MS : 0;
        const baseAlpha = anyActive
          ? config.SYNAPSE_ACTIVE_BASE_ALPHA + fireGlow * config.SYNAPSE_ACTIVE_GLOW_ALPHA
          : bothDone
            ? config.SYNAPSE_DONE_ALPHA
            : config.SYNAPSE_IDLE_BASE_ALPHA + syn.strength * config.SYNAPSE_IDLE_STRENGTH_ALPHA + fireGlow * config.SYNAPSE_IDLE_GLOW_ALPHA;
        const curvature = config.SYNAPSE_CURVATURE_BASE + (anyActive ? config.SYNAPSE_CURVATURE_WAVE * Math.sin(ts * config.SYNAPSE_CURVATURE_SPEED) : 0);
        const mx = (a.x + b.x) / 2 + (a.y - b.y) * curvature;
        const my = (a.y + b.y) / 2 - (a.x - b.x) * curvature;
        const w = (anyActive ? config.SYNAPSE_WIDTH_ACTIVE + fireGlow * config.SYNAPSE_WIDTH_ACTIVE_GLOW : bothDone ? config.SYNAPSE_WIDTH_DONE : config.SYNAPSE_WIDTH_IDLE)
          * (mode.synapseWidth / 2);

        const colorHex = parseColor(a.pal?.core || a.pal?.glow || '#00e5ff');
        this.synapsesGfx.setStrokeStyle({ width: w, color: colorHex, alpha: Math.max(0.02, Math.min(1, baseAlpha)) });
        this.synapsesGfx.moveTo(a.x, a.y).quadraticCurveTo(mx, my, b.x, b.y).stroke();

        if (anyActive && Math.sin(ts * config.PULSE_CHECK_SPEED + syn.strength * 10) > config.PULSE_THRESHOLD) {
          const pulseR = config.PULSE_RADIUS_BASE + Math.sin(ts * config.PULSE_WAVE_SPEED) * config.PULSE_RADIUS_WAVE;
          const glow = this._synGlowSprite(syn, colorHex);
          glow.x = mx;
          glow.y = my;
          glow.scale.set(Math.max(0.6, pulseR / 4));
          glow.alpha = Math.max(0.1, config.PULSE_ALPHA_BASE + fireGlow * config.PULSE_ALPHA_GLOW);
        } else {
          const key = `${syn.from}->${syn.to}`;
          const glow = this.synGlowCache.get(key);
          if (glow) glow.alpha = 0;
        }

        for (let i = syn.particles.length - 1; i >= 0; i--) {
          const p = syn.particles[i];
          p.t += p.speed * config.PARTICLE_DT;
          if (p.t >= 1) {
            syn.particles.splice(i, 1);
            continue;
          }
          const u = 1 - p.t;
          const px = u * u * a.x + 2 * u * p.t * mx + p.t * p.t * b.x;
          const py = u * u * a.y + 2 * u * p.t * my + p.t * p.t * b.y;
          const ti = p.trailIdx || 0;
          p.trail[ti] = { x: px, y: py };
          p.trailIdx = (ti + 1) % config.PARTICLE_TRAIL_MAX;
          if (p.trailLen < config.PARTICLE_TRAIL_MAX) p.trailLen++;

          if (p.trailLen > 1) {
            const tmax = config.PARTICLE_TRAIL_MAX;
            for (let t = 0; t < p.trailLen - 1; t++) {
              const ci = (p.trailIdx - p.trailLen + t + tmax * 2) % tmax;
              const ni = (ci + 1) % tmax;
              const pt0 = p.trail[ci];
              const pt1 = p.trail[ni];
              if (!pt0 || !pt1) continue;
              const trailAlpha = (t / p.trailLen) * config.PARTICLE_TRAIL_ALPHA;
              this.trailsGfx.setStrokeStyle({
                width: Math.max(0.2, p.size * (t / p.trailLen) * config.PARTICLE_TRAIL_WIDTH),
                color: colorHex,
                alpha: trailAlpha
              });
              this.trailsGfx.moveTo(pt0.x, pt0.y).lineTo(pt1.x, pt1.y).stroke();
            }
          }

          const sp = this.particleSprites[particleCount++];
          if (!sp) continue;
          const alpha = p.t > config.PARTICLE_FADE_START
            ? (1 - p.t) / config.PARTICLE_FADE_RANGE
            : Math.min(1, p.t / config.PARTICLE_FADE_IN_RANGE);
          sp.tint = colorHex;
          sp.alpha = Math.max(0, alpha);
          sp.x = px;
          sp.y = py;
          const s = Math.max(0.05, p.size / 6);
          sp.scale.set(s);
        }
      }

      this._syncParticleCount(particleCount);

      const activeNodes = new Set();
      const showLabels = mode.labelVisible;
      for (const [, n] of neurons) {
        n.scale = n.dying ? Math.max(0, n.scale - config.NODE_SCALE_OUT_SPEED) : n.scale + (n.targetScale - n.scale) * config.NODE_SCALE_LERP;
        if (n.scale <= 0) continue;
        n.phase += config.NODE_PHASE_STEP;
        const pulse = 1 + config.NODE_PULSE_SCALE * Math.sin(n.phase * config.NODE_PULSE_SPEED);
        const modeRadiusScale = n.type === 'session' ? 1 : (mode.nodeRadius / 14);
        const r = n.radius * n.scale * pulse * modeRadiusScale;
        if (r < config.MIN_VISIBLE_RADIUS) continue;
        activeNodes.add(n.id);

        const fireAge = ts - n.fireT;
        const fireGlow = fireAge < config.NODE_FIRE_GLOW_DURATION_MS ? 1 - fireAge / config.NODE_FIRE_GLOW_DURATION_MS : 0;
        const pal = n.pal || {};
        const coreHex = parseColor(pal.core || '#00e5ff');
        const sprite = this._spriteForNode(n);
        sprite.texture = this._nodeTexture(coreHex);
        sprite.x = n.x;
        sprite.y = n.y;
        sprite.scale.set(Math.max(0.06, (r * 2) / 64));
        sprite.alpha = 0.82 + fireGlow * 0.18;

        const ringA = config.NODE_RING_ALPHA_BASE + fireGlow * config.NODE_RING_ALPHA_GLOW + config.NODE_RING_ALPHA_WAVE * Math.sin(n.phase * config.NODE_RING_WAVE_SPEED);
        this.nodeRingGfx.circle(n.x, n.y, r + config.SESSION_RING_WIDTH).stroke({
          width: n.type === 'session' ? config.SESSION_RING_WIDTH : config.DEFAULT_RING_WIDTH,
          color: parseColor(pal.glow || pal.core || '#00e5ff'),
          alpha: Math.max(0.05, Math.min(1, ringA))
        });

        if (fireGlow > 0.02) {
          this.glowGfx.circle(n.x, n.y, r * (1 + 0.15 * fireGlow)).fill({ color: coreHex, alpha: fireGlow * 0.22 });
        }

        const lbl = this._labelFor(n);
        lbl.text = n.label || '';
        lbl.visible = showLabels && (n.type === 'session' || n.type === 'plan' || n.type === 'task');
        if (lbl.visible) {
          lbl.x = n.x;
          lbl.y = n.y + r + (n.type === 'session' ? config.SESSION_LABEL_OFFSET : config.PLAN_LABEL_OFFSET);
          lbl.style.fontSize = n.type === 'task' ? config.TASK_FONT_SIZE : (n.type === 'plan' ? config.PLAN_FONT_SIZE : config.FONT_SIZE_FACTOR);
        }
      }

      for (const [id, sprite] of this.nodeCache) {
        if (!activeNodes.has(id)) {
          sprite.visible = false;
          const lbl = this.labelCache.get(id);
          if (lbl) lbl.visible = false;
        } else {
          sprite.visible = true;
        }
      }
    }

    resize(w, h) {
      this.w = Math.max(10, w || this.container?.clientWidth || 10);
      this.h = Math.max(10, h || this.container?.clientHeight || 10);
      if (this.app?.renderer) this.app.renderer.resize(this.w, this.h);
      this._rebuildGrid();
    }

    destroy() {
      for (const t of this.nodeTextures.values()) t.destroy(true);
      this.nodeTextures.clear();
      for (const s of this.synGlowCache.values()) s.destroy();
      this.synGlowCache.clear();
      for (const t of this.labelCache.values()) t.destroy();
      this.labelCache.clear();
      this.nodeCache.clear();
      if (this.particleTexture) this.particleTexture.destroy(true);
      if (this.app) {
        this.app.destroy(true, { children: true, texture: true, textureSource: true });
        this.app = null;
      }
    }
  }

  window.BrainWebGLRenderer = BrainWebGLRenderer;
  if (typeof module !== 'undefined' && module.exports) module.exports = { BrainWebGLRenderer };
})();
