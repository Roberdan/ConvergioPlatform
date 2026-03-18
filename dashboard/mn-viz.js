;(function () {
  'use strict';

  const ns = (window.MaranelloEnhancer = window.MaranelloEnhancer || {});

  ns._meshNetworkInstance = null;
  ns._meshNetworkTimer = null;
  ns._brainVizInstance = null;
  ns._meshResizeObserver = null;
  ns._brainResizeObserver = null;

  const canInit = (el) => !!el && el.offsetWidth > 0 && el.offsetHeight > 0;

  // Mesh network visualization removed — card strip from websocket.js is sufficient
  ns.enhanceMeshNetwork = function enhanceMeshNetwork() {};

  ns.cleanupMeshNetwork = function cleanupMeshNetwork() {
    if (ns._meshNetworkTimer) clearInterval(ns._meshNetworkTimer);
    ns._meshNetworkTimer = null;
    ns._meshNetworkInstance = null;
    ns._meshResizeObserver?.disconnect?.();
    ns._meshResizeObserver = null;
    document.querySelectorAll('.mn-mesh-viz').forEach((el) => el.remove());
  };

  ns.enhanceBrainViz = function enhanceBrainViz() {
    const api = ns.M?.();
    if (!api?.neuralNodes) return;
    const primary = document.getElementById('brain-canvas-container');
    const fullscreen = document.getElementById('brain-canvas-fullscreen');
    const host = fullscreen && primary && fullscreen.contains(primary) ? fullscreen : primary;
    if (!host || host.querySelector('.mn-brain-viz')) return;

    const vizEl = document.createElement('div');
    vizEl.className = 'mn-brain-viz';
    vizEl.style.cssText = 'width:100%;height:100%;position:absolute;top:0;left:0;z-index:1;border-radius:var(--radius-md, 12px);overflow:hidden;background:radial-gradient(circle at center, color-mix(in srgb, var(--info, #4EA8DE) 8%, transparent), var(--bg-deep, #0a0a0a) 70%);';
    host.style.position = 'relative';
    host.appendChild(vizEl);
    ns.addEl?.(vizEl);

    const initBrain = () => {
      if (!canInit(vizEl) || ns._brainVizInstance) return;
      try {
        // Start in data-driven mode with labels and force layout
        ns._brainVizInstance = api.neuralNodes(vizEl, {
          nodeCount: 30,
          particleCount: 2,
          pulseSpeed: 1.1,
          interactive: true,
          labels: true,
          forceLayout: true,
        });
        if (!ns._brainVizInstance) return;
        ns._brainVizInstance.setActivity(0.55);
        setTimeout(() => { try { ns._brainVizInstance.pulse?.(0); } catch (_) {} }, 500);

        const slider = document.getElementById('brain-activity-slider');
        if (slider) {
          slider.addEventListener('input', (e) => {
            const level = Number(e.target.value || 0) / 100;
            ns._brainVizInstance?.setActivity?.(level);
          });
        }
      } catch (e) {
        console.warn('[Maranello] neuralNodes init failed:', e);
      }
    };

    ns.observeBrainVisibility?.(host, initBrain);
    if (typeof ResizeObserver === 'function') {
      ns._brainResizeObserver?.disconnect?.();
      ns._brainResizeObserver = new ResizeObserver(() => initBrain());
      ns._brainResizeObserver.observe(host);
    }
  };

  /**
   * Sync real session/agent data into neuralNodes (v4.5.0 data-driven API).
   * Called from brain-canvas.js pollData().
   */
  ns.syncBrainData = function syncBrainData(sessions, agents) {
    const mn = ns._brainVizInstance;
    if (!mn || typeof mn.setNodes !== 'function') return;

    const SESSION_COL = { claude: '#FFC72C', copilot: '#4EA8DE', opencode: '#00A651' };
    const toolOf = (id, type) => {
      const t = (type || id || '').toLowerCase();
      if (t.includes('copilot')) return 'copilot';
      if (t.includes('opencode')) return 'opencode';
      if (t.includes('claude')) return 'claude';
      return 'unknown';
    };
    const toolName = (k) => ({ claude: 'Claude', copilot: 'Copilot', opencode: 'OpenCode' })[k] || k;

    const nodes = [];
    const conns = [];

    (sessions || []).forEach((s) => {
      const tool = toolOf(s.session_id || s.agent_id, s.type);
      const label = toolName(tool);
      const desc = (s.description || s.command || '').substring(0, 25);
      nodes.push({
        id: s.session_id || s.agent_id,
        label: label,
        sublabel: desc || tool,
        color: SESSION_COL[tool] || '#FFC72C',
        size: s.status === 'running' ? 1.8 : 1.2,
        group: tool,
        energy: s.status === 'running' ? 0.8 : 0.2,
      });

      (s.children || []).forEach((c) => {
        const childTool = toolOf(c.agent_id, c.type);
        nodes.push({
          id: c.agent_id,
          label: toolName(childTool),
          sublabel: (c.description || c.model || '').substring(0, 25),
          color: SESSION_COL[childTool] || SESSION_COL[tool] || '#4EA8DE',
          size: 0.7,
          group: tool,
          energy: c.status === 'running' ? 0.6 : 0.1,
        });
        conns.push({
          from: s.session_id || s.agent_id,
          to: c.agent_id,
          strength: c.status === 'running' ? 0.8 : 0.3,
        });
      });
    });

    // Cross-session connections for shared plans
    const planSessions = new Map();
    (sessions || []).forEach((s) => {
      const planId = s.plan_id || s.metadata?.plan_id;
      if (planId) {
        if (!planSessions.has(planId)) planSessions.set(planId, []);
        planSessions.get(planId).push(s.session_id || s.agent_id);
      }
    });
    planSessions.forEach((ids) => {
      for (let i = 0; i < ids.length - 1; i++) {
        conns.push({ from: ids[i], to: ids[i + 1], strength: 0.4 });
      }
    });

    if (nodes.length > 0) {
      mn.setNodes(nodes);
      mn.setConnections(conns);
      const active = nodes.filter((n) => n.energy > 0.5).length;
      mn.setActivity(Math.min(1, 0.2 + active * 0.15));
    }
  };

  ns.cleanupBrainViz = function cleanupBrainViz() {
    ns._brainVizInstance = null;
    ns._brainResizeObserver?.disconnect?.();
    ns._brainResizeObserver = null;
    document.querySelectorAll('.mn-brain-viz').forEach((el) => el.remove());
  };
})();
