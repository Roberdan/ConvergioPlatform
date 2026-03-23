// brain-physics-bridge.js — WASM physics bridge for Brain visualization
(function () {
  'use strict';

  let wasmEngine = null;
  let wasmReady = false;

  async function initWasmPhysics() {
    try {
      const module = await import('./brain-physics-pkg/brain_physics_wasm.js');
      await module.default();
      wasmEngine = new module.PhysicsEngine(800, 600);
      wasmReady = true;
      console.log('[brain] WASM physics engine loaded');
    } catch (e) {
      console.warn('[brain] WASM physics unavailable, using JS fallback:', e?.message || e);
      wasmReady = false;
    }
  }

  window.brainWasmStep = function (neurons, synapses, width, height, config) {
    if (!wasmReady || !wasmEngine) return null;

    wasmEngine.set_bounds(width, height);

    const nodeData = [];
    const nodeKeys = [];
    const indexByKey = new Map();

    for (const [key, n] of neurons) {
      if (n?.dying) continue;
      const idx = nodeKeys.length;
      nodeKeys.push(key);
      indexByKey.set(key, idx);
      nodeData.push(
        n.x || 0,
        n.y || 0,
        n.vx || 0,
        n.vy || 0,
        n.radius || 8,
        n.mass || n.radius || 8,
        n.pinned ? 1 : 0,
        n.type === 'session' ? 1 : 0
      );
    }

    wasmEngine.set_nodes(new Float32Array(nodeData), nodeKeys.length);

    const synIndices = [];
    const synProps = [];

    for (const syn of synapses) {
      const fromIdx = indexByKey.get(syn.from);
      const toIdx = indexByKey.get(syn.to);
      if (fromIdx == null || toIdx == null) continue;

      synIndices.push(fromIdx, toIdx);
      const a = neurons.get(syn.from);
      const b = neurons.get(syn.to);
      const springLength = syn.springLength || ((a?.radius || 8) + (b?.radius || 8) + 40);
      const isSessionLink = a?.type === 'session' || b?.type === 'session' ? 1 : 0;
      synProps.push((syn.strength ?? 1), springLength, isSessionLink);
    }

    wasmEngine.set_synapses(
      new Uint32Array(synIndices),
      new Float32Array(synProps),
      Math.floor(synIndices.length / 2)
    );

    const budget = Number(config?.FRAME_BUDGET_MS) || 8;
    const result = wasmEngine.step_budget(budget);

    let i = 0;
    for (const key of nodeKeys) {
      const n = neurons.get(key);
      if (n && !n.pinned) {
        n.x = result[i];
        n.y = result[i + 1];
        n.vx = result[i + 2];
        n.vy = result[i + 3];
      }
      i += 4;
    }

    return true;
  };

  window.brainWasmReady = function () {
    return wasmReady;
  };

  initWasmPhysics();
})();
