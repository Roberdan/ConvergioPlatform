/* brain/interact.js — Mouse/touch event handlers: hover, click, drag, zoom */
'use strict';

import { BRAIN_CONFIG, BRAIN_IMMERSIVE, meshColor, cachedColor, buildPAL } from './config.js';

let PAL = null;
function pal() { if (!PAL) PAL = buildPAL(); return PAL; }
function _cssVar(name, fallback) { return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback; }

function esc(s) { return (s || '').replace(/</g, '&lt;').replace(/>/g, '&gt;'); }

/**
 * Set up all canvas interaction handlers.
 * @param {HTMLCanvasElement} canvas
 * @param {object} S — shared state: {w, h, neurons, synapses, sessions, brainData, hover, mouse, container, canvas, getBrainMode}
 * @returns {Function} teardown callback
 */
export function setupInteraction(canvas, S) {
  function canvasXY(e) {
    const zoom = parseFloat(document.body.style.zoom) || 1;
    if (e.touches) {
      const rect = canvas.getBoundingClientRect();
      const cx = (e.touches[0].clientX - rect.left) / zoom;
      const cy = (e.touches[0].clientY - rect.top) / zoom;
      return { x: cx * S.w / (rect.width / zoom), y: cy * S.h / (rect.height / zoom) };
    }
    return { x: e.offsetX / zoom, y: e.offsetY / zoom };
  }

  function hitTest(x, y) {
    const mode = S.getBrainMode();
    const radiusScale = mode.nodeRadius / BRAIN_IMMERSIVE.nodeRadius;
    for (const [id, n] of S.neurons) {
      if (n.dying) continue;
      const dx = x - n.x, dy = y - n.y;
      const hitRadius = (n.type === 'session' ? n.radius : n.radius * radiusScale) + BRAIN_CONFIG.HIT_PADDING;
      if (dx * dx + dy * dy < hitRadius * hitRadius) return id;
    }
    return null;
  }

  function fireSynapsesFor(id) {
    for (const syn of S.synapses) {
      if (syn.from === id || syn.to === id) syn.fire(S.getBrainMode);
    }
  }

  function onMouseMove(e) {
    const p = canvasXY(e);
    S.mouse.x = p.x; S.mouse.y = p.y;
    S.hover = hitTest(p.x, p.y);
    canvas.style.cursor = S.hover ? 'pointer' : 'default';
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

  canvas.addEventListener('mousemove', onMouseMove);
  canvas.addEventListener('mouseleave', onMouseLeave);
  canvas.addEventListener('click', onClick);
  canvas.addEventListener('touchstart', onTouchStart, { passive: false });
  canvas.addEventListener('touchend', onTouchEnd);

  return function teardown() {
    canvas.removeEventListener('mousemove', onMouseMove);
    canvas.removeEventListener('mouseleave', onMouseLeave);
    canvas.removeEventListener('click', onClick);
    canvas.removeEventListener('touchstart', onTouchStart);
    canvas.removeEventListener('touchend', onTouchEnd);
  };
}

/** Draw hover tooltip on canvas */
export function drawTooltip(c, S) {
  if (!S.hover || !S.getBrainMode().labelVisible) return;
  const n = S.neurons.get(S.hover);
  if (!n) return;
  const m = n.meta || {};
  const lines = _buildTooltipLines(n, m, S);

  const lh = BRAIN_CONFIG.TOOLTIP_LINE_HEIGHT, pad = BRAIN_CONFIG.TOOLTIP_PADDING;
  const maxW = Math.max(...lines.map(l => c.measureText ? BRAIN_CONFIG.TOOLTIP_TEXT_WIDTH_FACTOR * l.length : BRAIN_CONFIG.TOOLTIP_FALLBACK_WIDTH));
  const tw = Math.min(BRAIN_CONFIG.TOOLTIP_MAX_WIDTH, maxW + pad * 2);
  const th = lines.length * lh + pad * 2;
  let tx = n.x + n.radius + BRAIN_CONFIG.TOOLTIP_OFFSET, ty = n.y - th / 2;
  if (tx + tw > S.w - BRAIN_CONFIG.TOOLTIP_EDGE_MARGIN) tx = n.x - n.radius - tw - BRAIN_CONFIG.TOOLTIP_OFFSET;
  if (ty < BRAIN_CONFIG.TOOLTIP_EDGE_MARGIN) ty = BRAIN_CONFIG.TOOLTIP_EDGE_MARGIN;
  if (ty + th > S.h - BRAIN_CONFIG.TOOLTIP_EDGE_MARGIN) ty = S.h - th - BRAIN_CONFIG.TOOLTIP_EDGE_MARGIN;

  const P = pal();
  c.save();
  c.fillStyle = 'rgba(10,10,10,0.88)';
  c.strokeStyle = `${n.pal.glow}0.3)`;
  c.lineWidth = 1; c.shadowBlur = 4; c.shadowColor = `${n.pal.glow}0.15)`;
  c.beginPath();
  if (c.roundRect) c.roundRect(tx, ty, tw, th, 6);
  else c.rect(tx, ty, tw, th);
  c.fill(); c.stroke(); c.shadowBlur = 0;
  c.font = 'calc(13px * var(--mn-a11y-font-scale, 1)) "Barlow Condensed","JetBrains Mono",monospace';
  c.textAlign = 'left';
  lines.forEach((line, i) => {
    c.fillStyle = i === 0 ? _cssVar('--mn-text', '#fff') : (line.startsWith('Cmd:') ? P.sub.core : _cssVar('--mn-text-muted', '#8899bb'));
    c.fillText(line, tx + pad, ty + pad + (i + 1) * lh - 3);
  });
  c.restore();
}

function _buildTooltipLines(n, m, S) {
  const lines = [];
  if (n.type === 'session') {
    lines.push(n.label);
    if (m.tty) lines.push(`TTY ${m.tty} · PID ${m.pid || '?'}`);
    if (m.cpu != null) lines.push(`CPU ${m.cpu}% · MEM ${m.mem || 0}%`);
    if (m.duration_s) lines.push(`Duration: ${_fmtDur(m.duration_s)}`);
    if (m.tokens_total) lines.push(`Tokens: ${_fmtTok(m.tokens_total)}`);
    if (m.model && !m.model.endsWith('-cli') && m.model !== n.meta?.agent_type) lines.push(`Model: ${m.model}`);
    if (m.description && m.description.trim().length > 2 && !m.description.includes('/bin/'))
      lines.push(`Task: ${m.description.trim().substring(0, 80)}`);
    if (m.cwd && m.cwd !== 'unknown') { const proj = m.cwd.split('/').pop(); if (proj) lines.push(`Dir: ${proj}`); }
  } else if (n.type === 'plan') {
    lines.push(m.name || n.label);
    lines.push(`Progress: ${m.tasks_done || 0}/${m.tasks_total || 0} (${m.progress || 0}%)`);
    if (m.host) lines.push(`Node: ${m.host}`);
  } else if (n.type === 'task') {
    lines.push(m.title || n.label);
    lines.push(`Status: ${m.status || '?'}${m.priority ? ' · ' + m.priority : ''}`);
    if (m.executor_host) lines.push(`Node: ${m.executor_host}`);
    if (m.model) lines.push(`Model: ${m.model}`);
    if (m.tokens) lines.push(`Tokens: ${_fmtTok(m.tokens)}`);
    if (m.lines_added) lines.push(`Lines: +${m.lines_added}`);
    if (m.wave_name) lines.push(`Wave: ${m.wave_name}`);
    if (m.plan_name) lines.push(`Plan: ${m.plan_name.substring(0, 50)}`);
  } else {
    lines.push(n.label);
    if (m.model && !m.model.endsWith('-cli')) lines.push(`Model: ${m.model}`);
    if (m.duration_s) lines.push(`Duration: ${_fmtDur(m.duration_s)}`);
    if (m.tokens_total) lines.push(`Tokens: ${_fmtTok(m.tokens_total)}`);
    if (m.cost_usd) lines.push(`Cost: $${Number(m.cost_usd).toFixed(4)}`);
    if (m.description && m.description !== n.label) lines.push(m.description.substring(0, 80));
  }
  return lines;
}

/** Show click detail panel */
export function showDetailPanel(id, S) {
  const n = S.neurons.get(id);
  if (!n) return;
  let panel = document.getElementById('brain-detail');
  if (!panel) {
    panel = document.createElement('div'); panel.id = 'brain-detail';
    panel.style.cssText = 'position:absolute;right:12px;top:12px;width:380px;max-height:560px;overflow-y:auto;background:var(--bg-card, #111);border:1px solid var(--border, #2a2a2a);border-radius:var(--radius-md, 12px);padding:16px;z-index:10;font:13px var(--font-mono, "Barlow Condensed", monospace);color:var(--text, #9e9e9e);backdrop-filter:blur(12px);';
    S.container.appendChild(panel);
  }
  panel.innerHTML = _buildDetailHTML(n, S);
}

function _buildDetailHTML(n, S) {
  const m = n.meta || {};
  const row = (k, v) => v ? `<div style="display:flex;justify-content:space-between;padding:2px 0"><span style="color:var(--mn-text-dim, #5a6080)">${k}</span><span style="color:var(--mn-text, #e0e4f0)">${v}</span></div>` : '';
  let html = `<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:10px"><span style="font:bold 14px 'JetBrains Mono',monospace;color:${n.pal.core}">${n.label}</span><span style="cursor:pointer;color:var(--mn-text-dim, #5a6080);font-size:16px" onclick="this.parentElement.parentElement.remove()">&#x2715;</span></div>`;

  if (n.type === 'session') {
    html += row('PID', m.pid) + row('TTY', m.tty);
    html += row('CPU', m.cpu != null ? m.cpu + '%' : '') + row('MEM', m.mem != null ? m.mem + '%' : '');
    html += row('Duration', _fmtDur(m.duration_s)) + row('Model', m.model);
    html += row('Tokens In', _fmtTok(m.tokens_in)) + row('Tokens Out', _fmtTok(m.tokens_out));
    html += row('Total Tok', _fmtTok(m.tokens_total)) + row('Cost', m.cost_usd ? '$' + Number(m.cost_usd).toFixed(4) : '');
    html += row('Started', m.started_at || '');
  } else if (n.type === 'plan') {
    html += row('Status', m.status) + row('Node', m.host || m.executor_host);
    html += row('Progress', `${m.tasks_done || 0}/${m.tasks_total || 0}`);
    html += `<div style="margin:6px 0;height:4px;background:var(--mn-bg-subtle, #1a2040);border-radius:2px"><div style="height:100%;width:${m.progress || 0}%;background:linear-gradient(90deg,${meshColor(m.host || m.executor_host).core},${pal().green});border-radius:2px"></div></div>`;
  } else if (n.type === 'task') {
    html += row('Status', m.status) + row('Node', m.executor_host);
    html += row('Model', m.model) + row('Priority', m.priority) + row('Type', m.type);
    html += row('Tokens', _fmtTok(m.tokens)) + row('Lines', m.lines_added ? `+${m.lines_added}` : '');
    html += row('Wave', m.wave_name) + row('Plan', m.plan_name);
    if (m.title) html += `<div style="margin-top:6px;color:var(--mn-text, #e0e4f0)">${m.title}</div>`;
  } else {
    html += row('Model', m.model) + row('Duration', _fmtDur(m.duration_s));
    html += row('Tokens', _fmtTok(m.tokens_total));
    if (m.description) html += `<div style="margin-top:6px;color:${pal().sub.core};word-break:break-all">${esc(m.description.substring(0, 120))}</div>`;
  }
  return html;
}

// Formatting helpers
function _fmtDur(s) { if (!s || s < 0) return ''; if (s < BRAIN_CONFIG.MINUTE_SECONDS) return `${Math.round(s)}s`; if (s < BRAIN_CONFIG.HOUR_SECONDS) return `${Math.round(s / BRAIN_CONFIG.MINUTE_SECONDS)}m`; return `${(s / BRAIN_CONFIG.HOUR_SECONDS).toFixed(1)}h`; }
function _fmtTok(n) { if (!n) return '0'; if (n > BRAIN_CONFIG.TOKENS_MEGA) return `${(n / BRAIN_CONFIG.TOKENS_MEGA).toFixed(1)}M`; if (n > BRAIN_CONFIG.TOKENS_KILO) return `${(n / BRAIN_CONFIG.TOKENS_KILO).toFixed(1)}k`; return String(n); }
