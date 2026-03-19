/* brain/canvas.js — Main renderer: animation loop, state, data sync */
'use strict';
import { BRAIN_CONFIG, BRAIN_EMBEDDED, BRAIN_IMMERSIVE, PI2, gradientCache } from './config.js';
import { updatePhysics } from './physics.js';
import { Neuron, renderNodes } from './nodes.js';
import { Synapse, drawGrid, renderEffects } from './effects.js';
import { setupInteraction, drawTooltip } from './interact.js';

const S = { container: null, canvas: null, ctx: null, w: 0, h: 0, dpr: 1, raf: 0, running: true, lastTs: 0, _prevTs: 0, _skipNext: false, webglRenderer: null, ro: null, pollT: 0, ws: null, wsRetry: 0, wsT: 0, neurons: new Map(), synapses: [], sessions: [], agents: [], brainData: null, forceTick: 0, frameCount: 0, sweepFrame: { v: 0 }, hover: null, mouse: { x: -1, y: -1 }, interactionTeardown: null, getBrainMode: null };

function getBrainMode() { const c = document.getElementById('brain-canvas-container') || S.container; return (!c || c.offsetWidth <= 500) ? BRAIN_EMBEDDED : BRAIN_IMMERSIVE; }
S.getBrainMode = getBrainMode;
function applyBrainMode(mode) {
  if (!S.container) return;
  S.container.classList.toggle('brain-immersive', mode === BRAIN_IMMERSIVE);
  S.container.classList.toggle('brain-embedded', mode === BRAIN_EMBEDDED);
  S.container.style.height = mode === BRAIN_EMBEDDED ? '300px' : '100%';
  const ctrl = document.getElementById('brain-controls'); if (ctrl) ctrl.style.display = mode.controlsVisible ? '' : 'none';
}
function parseMeta(s) { try { return typeof s === 'string' ? JSON.parse(s) : (s || {}); } catch { return {}; } }
function toolOf(id, type) { const t = (type || id || '').toLowerCase(); if (t.includes('copilot')) return 'copilot'; if (t.includes('opencode')) return 'opencode'; if (t.includes('claude')) return 'claude'; const m = t.match(/^([a-z]+)(?:-cli|-agent)?/); return m ? m[1] : 'unknown'; }
const _tn = { claude: 'Claude', copilot: 'Copilot', opencode: 'OpenCode' };
function toolDisplayName(k) { return _tn[k] || (k.charAt(0).toUpperCase() + k.slice(1)); }
function shortModel(m) { if (!m) return ''; const lo = m.toLowerCase(); if (lo === 'claude-cli' || lo === 'copilot-cli' || lo === 'opencode') return ''; for (const [p, r] of [['opus','opus'],['sonnet','sonnet'],['haiku','haiku'],['codex','codex']]) { if (lo.includes(p)) return r; } if (/gpt-5/.test(lo)) return 'gpt5'; if (/gpt-4/.test(lo)) return 'gpt4'; if (/\bo[34]/.test(lo)) return lo.match(/o[34][^ ]*/)?.[0] || 'o'; for (const p of ['deepseek','gemini','llama','mistral','qwen']) { if (lo.includes(p)) return lo.replace(new RegExp('.*' + p + '-?'), p.slice(0,4) + '-').substring(0, 8); } return m.replace(/^(claude-|gpt-|copilot-|opencode-)/i, '').substring(0, 12); }
function richLabel(tool, tty, model, desc) { const name = toolDisplayName(tool), sm = shortModel(model), d = (desc || '').substring(0, BRAIN_CONFIG.SESSION_DESC_MAX).trim(); if (d && sm) return `${name} ${sm} · ${d}`; if (d) return `${name} · ${d}`; if (sm && sm !== 'cli') return `${name} ${sm}`; if (tty) return `${name} ${tty}`; return name; }
function childLabel(model, desc, type) { const d = (desc || '').substring(0, BRAIN_CONFIG.CHILD_DESC_MAX).trim(), sm = shortModel(model); if (d && sm) return `${sm}: ${d}`; if (d) return d; if (sm && sm !== 'cli') return sm; return type || 'agent'; }
function fireSynapsesFor(id) { for (const syn of S.synapses) { if (syn.from === id || syn.to === id) syn.fire(getBrainMode); } }

// --- Data sync ---
function syncGraph() {
  const now = performance.now(), active = S.sessions.filter(s => s.status === 'running'), ids = new Set();
  // Sessions
  for (let i = 0; i < active.length; i++) {
    const sess = active[i], meta = parseMeta(sess.metadata), tool = toolOf(sess.session_id, sess.type);
    const label = richLabel(tool, meta.tty || '', sess.model, sess.description);
    ids.add(sess.session_id);
    Object.assign(meta, { status: sess.status, duration_s: sess.duration_s, tokens_total: sess.tokens_total, tokens_in: sess.tokens_in, tokens_out: sess.tokens_out, cost_usd: sess.cost_usd, model: sess.model, description: sess.description, started_at: sess.started_at });
    if (!S.neurons.has(sess.session_id)) {
      const n = new Neuron(sess.session_id, 'session', label, meta);
      n.tool = tool; const angle = (i / Math.max(active.length, 1)) * PI2 - Math.PI / 2;
      const rad = Math.min(S.w, S.h) * BRAIN_CONFIG.SESSION_ORBIT_RADIUS;
      n.x = S.w / 2 + Math.cos(angle) * rad; n.y = S.h / 2 + Math.sin(angle) * rad;
      S.neurons.set(sess.session_id, n);
      for (const [oid, other] of S.neurons) { if (oid !== sess.session_id && other.type === 'session' && !other.dying && other.tool === tool) S.synapses.push(new Synapse(sess.session_id, oid)); }
      setTimeout(() => { const nn = S.neurons.get(sess.session_id); if (nn) { nn.fire(); fireSynapsesFor(sess.session_id); } }, BRAIN_CONFIG.SESSION_FIRE_DELAY_MS);
    } else { const n = S.neurons.get(sess.session_id); n.label = label; n.meta = meta; n.active = true; }
    for (const ch of (sess.children || [])) {
      if (ch.status !== 'running') continue; ids.add(ch.agent_id);
      if (!S.neurons.has(ch.agent_id)) {
        const cn = new Neuron(ch.agent_id, 'sub', childLabel(ch.model, ch.description, ch.type), { status: ch.status, model: ch.model, description: ch.description, duration_s: ch.duration_s, tokens_total: ch.tokens_total, cost_usd: ch.cost_usd });
        cn.tool = tool; const par = S.neurons.get(sess.session_id);
        cn.x = (par?.x || S.w / 2) + (Math.random() - 0.5) * BRAIN_CONFIG.SESSION_SPAWN_JITTER;
        cn.y = (par?.y || S.h / 2) + (Math.random() - 0.5) * BRAIN_CONFIG.SESSION_SPAWN_JITTER;
        S.neurons.set(ch.agent_id, cn); S.synapses.push(new Synapse(sess.session_id, ch.agent_id));
        setTimeout(() => fireSynapsesFor(ch.agent_id), BRAIN_CONFIG.CHILD_FIRE_DELAY_MS);
      } else { const ex = S.neurons.get(ch.agent_id); ex.label = childLabel(ch.model, ch.description, ch.type); ex.meta = { status: ch.status, model: ch.model, description: ch.description, duration_s: ch.duration_s, tokens_total: ch.tokens_total }; }
    }
  }
  // Plans
  for (const plan of (S.brainData?.plans || [])) {
    const pid = `plan-${plan.id}`; ids.add(pid); const tc = plan.tasks_total || 1;
    const pr = Math.max(BRAIN_CONFIG.PLAN_RADIUS_MIN, Math.min(BRAIN_CONFIG.PLAN_RADIUS_MAX, BRAIN_CONFIG.PLAN_RADIUS_BASE + Math.sqrt(tc) * BRAIN_CONFIG.PLAN_RADIUS_SCALE));
    const meta = { name: plan.name, status: plan.status, progress: plan.progress_pct, tasks_done: plan.tasks_done, tasks_total: plan.tasks_total, host: plan.execution_host, executor_host: plan.execution_host };
    if (!S.neurons.has(pid)) {
      const pn = new Neuron(pid, 'plan', `#${plan.id} ${(plan.name || '').substring(0, BRAIN_CONFIG.PLAN_NAME_MAX)}`, meta);
      pn.tool = 'claude'; pn.radius = pr;
      const idx = (S.brainData?.plans || []).indexOf(plan), tot = (S.brainData?.plans || []).length;
      const a = (idx / Math.max(1, tot)) * PI2 + Math.random() * BRAIN_CONFIG.PLAN_ORBIT_JITTER_ANGLE;
      pn.x = S.w / 2 + Math.cos(a) * S.w * BRAIN_CONFIG.PLAN_ORBIT_FACTOR + (Math.random() - 0.5) * BRAIN_CONFIG.PLAN_ORBIT_JITTER_POS;
      pn.y = S.h / 2 + Math.sin(a) * S.h * BRAIN_CONFIG.PLAN_ORBIT_FACTOR + (Math.random() - 0.5) * BRAIN_CONFIG.PLAN_ORBIT_JITTER_POS;
      S.neurons.set(pid, pn);
    } else { const en = S.neurons.get(pid); en.meta = meta; en.radius = pr; }
  }
  // Tasks
  const wg = {};
  for (const task of (S.brainData?.tasks || [])) {
    const tid = `task-${task.id}`; ids.add(tid); const planNid = `plan-${task.plan_id}`;
    const wk = `${task.plan_id}-${task.wave_id || 'W0'}`; if (!wg[wk]) wg[wk] = []; wg[wk].push(tid);
    let la = 0; try { const od = typeof task.output_data === 'string' ? JSON.parse(task.output_data) : task.output_data; la = od?.lines_added || 0; } catch {}
    const dr = la > 0 ? Math.max(BRAIN_CONFIG.TASK_LINES_RADIUS_MIN, Math.min(BRAIN_CONFIG.TASK_LINES_RADIUS_MAX, BRAIN_CONFIG.TASK_RADIUS_BASE + Math.sqrt(la) * BRAIN_CONFIG.TASK_RADIUS_SCALE))
      : Math.max(BRAIN_CONFIG.TASK_LINES_RADIUS_MIN, Math.min(BRAIN_CONFIG.TASK_FALLBACK_RADIUS_MAX, BRAIN_CONFIG.TASK_RADIUS_BASE + (task.tokens || 0) / BRAIN_CONFIG.TASK_TOKEN_DIVISOR));
    const tr = task.status === 'pending' ? BRAIN_CONFIG.TASK_PENDING_RADIUS : dr;
    if (!S.neurons.has(tid)) {
      const tn = new Neuron(tid, 'task', (task.title || '').substring(0, BRAIN_CONFIG.TASK_LABEL_MAX), { title: task.title, status: task.status, priority: task.priority, type: task.task_type, plan_name: task.plan_name, wave_name: task.wave_id || task.wave_name, executor_host: task.executor_host, model: task.model, tokens: task.tokens, lines_added: la });
      tn.tool = 'claude'; tn.radius = tr; const pN = S.neurons.get(planNid); const tA = Math.random() * PI2; const tD = BRAIN_CONFIG.TASK_ORBIT_MIN + Math.random() * BRAIN_CONFIG.TASK_ORBIT_RANGE;
      tn.x = (pN?.x || S.w / 2) + Math.cos(tA) * tD; tn.y = (pN?.y || S.h / 2) + Math.sin(tA) * tD;
      S.neurons.set(tid, tn); if (S.neurons.has(planNid)) S.synapses.push(new Synapse(planNid, tid));
      if (task.executor_session_id && S.neurons.has(task.executor_session_id)) S.synapses.push(new Synapse(task.executor_session_id, tid));
    } else { const en = S.neurons.get(tid); en.meta = { ...en.meta, status: task.status, executor_host: task.executor_host, model: task.model, tokens: task.tokens, lines_added: la }; en.radius = tr; if (en.meta._prevStatus && en.meta._prevStatus !== task.status) { en.fire(); fireSynapsesFor(tid); } en.meta._prevStatus = task.status; }
  }
  for (const [, group] of Object.entries(wg)) {
    if (group.length < 2) continue;
    for (let i = 1; i < group.length; i++) { const a = group[i - 1], b = group[i]; if (!S.synapses.find(s => (s.from === a && s.to === b) || (s.from === b && s.to === a))) { const syn = new Synapse(a, b); syn.strength = BRAIN_CONFIG.WAVE_LINK_STRENGTH; S.synapses.push(syn); } }
  }
  // Cleanup dead
  for (const [id, n] of S.neurons) { if (!ids.has(id) && !n.dying) { n.dying = true; n.deathT = now; } }
  for (const [id, n] of S.neurons) { if (n.dying && n.scale <= 0) { S.neurons.delete(id); S.synapses = S.synapses.filter(s => s.from !== id && s.to !== id); } }
  _updateStats();
}

function _updateStats() { const el = document.getElementById('brain-stats'); if (!el) return; const run = S.sessions.filter(s => s.status === 'running'), tc = {}; run.forEach(s => { const t = toolOf(s.session_id, s.type); tc[t] = (tc[t] || 0) + 1; }); const ts = Object.entries(tc).map(([t, n]) => `${n}${toolDisplayName(t).charAt(0)}`).join('/'); el.textContent = `${run.length} sessions · ${ts || '0'} · ${(S.brainData?.plans || []).length} plans · ${(S.brainData?.tasks || []).length} tasks · ${S.synapses.length} synapses`; }

// --- Polling / WS ---
function pollData() {
  fetch('/api/brain').then(r => r.json()).then(data => {
    S.brainData = data; const sessions = data.sessions || [], agents = data.agents || [];
    const cm = new Map(); agents.forEach(a => { if (a.parent_session) { if (!cm.has(a.parent_session)) cm.set(a.parent_session, []); cm.get(a.parent_session).push(a); } });
    S.sessions = sessions.map(s => ({ session_id: s.agent_id, type: s.type || 'claude-cli', status: s.status, metadata: s.metadata, description: s.description, started_at: s.started_at, duration_s: s.duration_s, tokens_total: s.tokens_total, tokens_in: s.tokens_in, tokens_out: s.tokens_out, cost_usd: s.cost_usd, model: s.model, children: (cm.get(s.agent_id) || []).map(c => ({ agent_id: c.agent_id, type: c.type, model: c.model, description: c.description, status: c.status || 'running', duration_s: c.duration_s, tokens_total: c.tokens_total, cost_usd: c.cost_usd })) }));
    S.agents = agents; window._dashboardAgentData = { sessions: S.sessions, orphan_agents: [] };
    syncGraph(); scheduleFrame();
    if (window.MaranelloEnhancer?.syncBrainData) window.MaranelloEnhancer.syncBrainData(sessions, agents);
    if (S.canvas) S.canvas.style.opacity = '1';
  }).catch(() => {
    Promise.all([fetch('/api/sessions').then(r => r.json()).catch(() => []), fetch('/api/agents').then(r => r.json()).catch(() => ({ running: [] }))]).then(([raw, ad]) => {
      const run = ad.running || [], cm = new Map();
      run.forEach(a => { if (a.parent_session) { if (!cm.has(a.parent_session)) cm.set(a.parent_session, []); cm.get(a.parent_session).push(a); } });
      S.sessions = (raw || []).map(s => ({ session_id: s.agent_id, type: s.type, status: s.status, metadata: s.metadata, description: s.description, duration_s: s.duration_s, tokens_total: s.tokens_total, model: s.model, children: (cm.get(s.agent_id) || []).map(c => ({ agent_id: c.agent_id, type: c.type, model: c.model, description: c.description, status: c.status || 'running', duration_s: c.duration_s })) }));
      syncGraph(); scheduleFrame();
    });
  });
}

function scheduleFrame() { if (S.raf) return; S.raf = requestAnimationFrame(render); }

function render(ts) {
  S.raf = 0; if (!S.ctx || !S.running) return;
  try {
    S.lastTs = ts; S.frameCount++;
    if (S.frameCount > 10) { const dt = ts - S._prevTs; if (dt > 25) { S._skipNext = !S._skipNext; if (S._skipNext) { scheduleFrame(); return; } } else S._skipNext = false; }
    S._prevTs = ts;
    updatePhysics(S.neurons, S.synapses, S.w, S.h, { forceTick: S.forceTick });
    const mode = getBrainMode(); applyBrainMode(mode);
    const pool = mode.fireEffects === 'perimeter' ? S.synapses.filter(syn => { const a = S.neurons.get(syn.from), b = S.neurons.get(syn.to); return a?.type === 'session' || b?.type === 'session'; }) : S.synapses;
    if (pool.length && S.frameCount % 3 === 0 && Math.random() < BRAIN_CONFIG.AMBIENT_FIRE_CHANCE) { const syn = pool[Math.floor(Math.random() * pool.length)]; syn.fire(getBrainMode); const t = S.neurons.get(syn.to); if (t) t.fire(); }
    if (S.webglRenderer) { S.webglRenderer.render(S.neurons, S.synapses, ts, BRAIN_CONFIG); S.ctx.clearRect(0, 0, S.w, S.h); drawTooltip(S.ctx, S); }
    else { S.ctx.clearRect(0, 0, S.w, S.h); drawGrid(S.ctx, S.w, S.h); renderEffects(S.ctx, S.synapses, S.neurons, ts, getBrainMode, { frameCount: S.frameCount, sweepFrame: S.sweepFrame }); renderNodes(S.ctx, S.neurons, ts, S, getBrainMode); drawTooltip(S.ctx, S); }
  } catch (e) { console.warn('[brain] render error (loop continues):', e); }
  scheduleFrame();
}

function resize() { if (!S.container || !S.canvas) return; applyBrainMode(getBrainMode()); S.dpr = window.devicePixelRatio || 1; S.w = Math.max(BRAIN_CONFIG.CANVAS_MIN_SIZE, S.container.clientWidth); S.h = Math.max(BRAIN_CONFIG.CANVAS_MIN_SIZE, S.container.clientHeight); S.canvas.width = Math.floor(S.w * S.dpr); S.canvas.height = Math.floor(S.h * S.dpr); S.canvas.style.width = S.w + 'px'; S.canvas.style.height = S.h + 'px'; S.ctx.setTransform(S.dpr, 0, 0, S.dpr, 0, 0); if (S.webglRenderer) S.webglRenderer.resize(S.w, S.h); }

const wsUrl = () => `${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.host}/ws/brain`;
function connectWs() { try { S.ws = new WebSocket(wsUrl()); } catch { S.ws = null; } if (!S.ws) return; S.ws.onopen = () => { S.wsRetry = 0; }; S.ws.onmessage = () => pollData(); S.ws.onerror = () => S.ws?.close(); S.ws.onclose = () => { clearTimeout(S.wsT); S.wsT = setTimeout(connectWs, Math.min(BRAIN_CONFIG.WS_RETRY_MAX_MS, BRAIN_CONFIG.WS_RETRY_BASE_MS * Math.pow(BRAIN_CONFIG.WS_RETRY_EXP_BASE, S.wsRetry++))); }; }
function onVis() { S.running = !document.hidden; if (document.hidden) { if (S.pollT) { clearInterval(S.pollT); S.pollT = 0; } } else if (!S.pollT) S.pollT = setInterval(pollData, BRAIN_CONFIG.POLL_INTERVAL_MS); if (S.running) scheduleFrame(); if (!S.running && S.raf) { cancelAnimationFrame(S.raf); S.raf = 0; } }

function addHelpButton() {
  const ctrl = document.getElementById('brain-controls');
  if (!ctrl || document.getElementById('brain-help-btn')) return;
  const btn = document.createElement('button');
  btn.id = 'brain-help-btn'; btn.className = 'brain-ctrl-btn'; btn.title = 'Legend';
  btn.innerHTML = '<svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="8" cy="8" r="6.5"/><path d="M6.5 6.2a1.8 1.8 0 0 1 3.3.9c0 1.2-1.8 1-1.8 2.4M8 11.5v.01"/></svg>';
  btn.onclick = function() {
    let leg = document.getElementById('brain-legend'); if (leg) { leg.remove(); return; }
    leg = document.createElement('div'); leg.id = 'brain-legend';
    leg.style.cssText = 'position:absolute;left:12px;bottom:12px;background:var(--bg-card, #111);border:1px solid var(--border, #2a2a2a);border-radius:var(--radius-md, 12px);padding:10px 14px;z-index:15;font:calc(9px * var(--mn-a11y-font-scale, 1)) var(--font-mono, "Barlow Condensed", monospace);color:var(--text-dim, #616161);backdrop-filter:blur(10px);line-height:1.8;cursor:pointer;';
    leg.onclick = function() { leg.remove(); };
    leg.innerHTML = '<span style="color:var(--text, #9e9e9e);font-size:10px;letter-spacing:1px">NEURAL GRAPH</span><br><br><span style="color:var(--accent, #FFC72C)">&#9679;</span> Claude &nbsp; <span style="color:var(--info, #4EA8DE)">&#9679;</span> Copilot &nbsp; <span style="color:var(--success, #00A651)">&#9679;</span> OpenCode<br><span style="color:var(--success, #00A651)">&#9679;</span> Plan &nbsp; <span style="color:var(--accent, #FFC72C)">&#9679;</span> Task<br><br><span style="color:var(--text, #9e9e9e)">Brightness</span> = activity &nbsp; <span style="color:var(--text, #9e9e9e)">Size</span> = agents<br><span style="color:var(--text-dim)">Hover for details | Tap on mobile</span>';
    S.container.appendChild(leg);
  };
  ctrl.insertBefore(btn, ctrl.firstChild);
}

/** Main entry point */
export default function initBrain(containerId) {
  window.destroyBrainCanvas();
  S.container = typeof containerId === 'string' ? document.getElementById(containerId) : containerId;
  if (!S.container) return; gradientCache.clear(); S.frameCount = 0; S.sweepFrame.v = 0; S.running = true;
  S.canvas = document.createElement('canvas');
  S.canvas.style.cssText = 'display:block;width:100%;height:100%;border-radius:var(--radius-md, 12px);position:relative;z-index:2;';
  S.canvas.setAttribute('role', 'img'); S.canvas.setAttribute('aria-label', BRAIN_CONFIG.CANVAS_ARIA_LABEL);
  S.container.appendChild(S.canvas); S.ctx = S.canvas.getContext('2d', { alpha: true }); resize();
  if (window.Maranello?.autoResize) { try { window.Maranello.autoResize(S.canvas); } catch (_) {} }
  if (typeof BrainWebGLRenderer !== 'undefined') { try { S.webglRenderer = new BrainWebGLRenderer(S.container); S.canvas.style.opacity = '0'; S.canvas.style.background = 'transparent'; } catch (e) { S.webglRenderer = null; S.canvas.style.opacity = '1'; console.warn('[brain] WebGL failed:', e.message); } }
  else S.canvas.style.opacity = '1';
  S.ro = new ResizeObserver(resize); S.ro.observe(S.container);
  S.interactionTeardown = setupInteraction(S.canvas, S);
  document.addEventListener('visibilitychange', onVis);
  pollData(); connectWs(); addHelpButton(); scheduleFrame(); onVis();
}

// Window globals for backward compatibility
window.initBrainCanvas = function(id) { initBrain(id || 'brain-canvas-container'); };
window.destroyBrainCanvas = function() {
  if (S.raf) cancelAnimationFrame(S.raf); S.raf = 0;
  if (S.ro) S.ro.disconnect(); S.ro = null; if (S.ws) S.ws.close(); S.ws = null; clearTimeout(S.wsT); S.wsT = 0;
  if (S.pollT) clearInterval(S.pollT); S.pollT = 0;
  if (S.interactionTeardown) S.interactionTeardown(); S.interactionTeardown = null;
  if (S.webglRenderer) { S.webglRenderer.destroy(); S.webglRenderer = null; }
  document.removeEventListener('visibilitychange', onVis);
  if (S.container) S.container.innerHTML = ''; S.container = S.canvas = S.ctx = null;
  gradientCache.clear(); S.frameCount = 0; S.sweepFrame.v = 0; S.neurons.clear(); S.synapses = []; S.sessions = []; S.agents = [];
};
window.updateBrainData = function() { pollData(); };
window.toggleBrainFreeze = function() {
  S.running = !S.running; const btn = document.getElementById('brain-pause-btn');
  if (btn) btn.innerHTML = S.running ? (window.Icons ? Icons.pause(14) : '\u23F8') : (window.Icons ? Icons.start(14) : '\u25B6');
  if (S.running) scheduleFrame(); else if (S.raf) { cancelAnimationFrame(S.raf); S.raf = 0; }
};
window.rewindBrain = function() { S.neurons.clear(); S.synapses = []; pollData(); };
window.resizeBrainCanvas = resize;
window.brainResize = function() { resize(); scheduleFrame(); };
window.brainExpandTo = function(target) {
  const c = document.getElementById('brain-canvas-container') || S.container; if (!c) return;
  const t = typeof target === 'string' ? document.getElementById(target) || document.querySelector(target) : target;
  if (!t) return; if (c.parentElement !== t) t.appendChild(c); S.container = c;
  if (S.ro) { S.ro.disconnect(); S.ro.observe(S.container); } resize(); scheduleFrame();
};

const _boot = () => window.initBrainCanvas('brain-canvas-container');
if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', _boot);
else setTimeout(_boot, BRAIN_CONFIG.BOOT_DELAY_MS);
