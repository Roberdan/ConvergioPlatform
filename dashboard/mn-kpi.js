;(function () {
  'use strict';

  const ns = (window.MaranelloEnhancer = window.MaranelloEnhancer || {});

  ns._lastKpiHash = '';
  ns._meshPrev = null;
  ns._meshSpeedInterval = null;
  ns._kpiBound = false;
  ns._autoResizeCleanups = ns._autoResizeCleanups || [];

  ns.enhanceKpiGauges = function enhanceKpiGauges() {
    // Convergio 2.1: compact adaptive KPI strip, no gauge override
    return;
  };

  ns.renderKpiRow = function renderKpiRow(d) {
    const bar = document.getElementById('kpi-bar');
    if (!bar || !d) return;
    const hash = JSON.stringify([d.plans_active, d.plans_total, d.plans_done, d.mesh_online, d.mesh_total, d.total_tokens, d.today_tokens, d.blocked, d.today_lines_changed, d.yesterday_lines_changed, d.week_lines_changed, d.prev_week_lines_changed, d.today_cost, d.agents_today]);
    if (hash === ns._lastKpiHash) return;
    ns._lastKpiHash = hash;

    const fv = (v) => (typeof fmt === 'function' ? fmt(v) : v);
    const online = d.mesh_online || 0;
    const total = d.mesh_total || 0;
    const meshLed = online === total && total > 0 ? 'green' : online > 0 ? 'amber' : 'red';
    const blockedLed = (d.blocked || 0) > 0 ? 'red' : 'green';
    const activeLed = (d.plans_active || 0) > 0 ? 'amber' : 'off';
    const linesToday = Number(d.today_lines_changed || 0);
    const linesYesterday = d.yesterday_lines_changed != null ? Number(d.yesterday_lines_changed) : null;
    const linesWeek = Number(d.week_lines_changed || 0);
    const linesPrevWeek = d.prev_week_lines_changed != null ? Number(d.prev_week_lines_changed) : null;
    const costToday = Number(d.today_cost || 0);
    const agentsToday = Number(d.agents_today || 0);

    const left = [
      { label: 'Active Plans', value: d.plans_active || 0, unit: (d.plans_done || 0) + ' done', led: activeLed, target: 'mission-panel', key: 'kpi.plans_active' },
      { label: 'Total Plans', value: d.plans_total || 0, unit: 'executed', led: 'off', target: 'history-widget' },
      { label: 'Mesh Nodes', value: online + '/' + total, unit: 'online', led: meshLed, target: 'mesh-panel', key: 'kpi.mesh_online' },
      { label: 'Tokens Used', value: fv(d.total_tokens), unit: 'today ' + fv(d.today_tokens), led: 'off', target: 'widget-tokens' },
    ];
    const right = [
      { label: d.blocked > 0 ? 'Blocked' : 'Blocked', value: d.blocked || 0, unit: d.blocked > 0 ? 'needs attention' : 'tasks', led: blockedLed, target: 'task-pipeline-widget', key: 'kpi.blocked' },
      { label: 'Lines Today', value: fv(linesToday), unit: ns._kpiDelta(linesToday, linesYesterday, 'vs yesterday'), led: 'off', target: 'history-widget' },
      { label: 'Lines / Week', value: fv(linesWeek), unit: ns._kpiDelta(linesWeek, linesPrevWeek, 'vs prev'), led: 'off', target: 'history-widget' },
      { label: 'Cost Today', value: '$' + costToday.toFixed(0), unit: '', led: costToday > 50 ? 'red' : costToday > 20 ? 'amber' : 'green', target: 'widget-cost', key: 'kpi.cost' },
    ];

    const ledBulb = (led) => {
      const cls = { green: 'mn-led--green', amber: 'mn-led--amber', red: 'mn-led--red', off: 'mn-led--off' }[led] || 'mn-led--off';
      return '<div class="mn-strip__led"><div class="mn-led"><div class="mn-led__housing"><div class="mn-led__bulb ' + cls + '"></div></div></div></div>';
    };
    const valColor = (led) => led === 'green' ? 'var(--verde-racing, #00A651)'
      : led === 'amber' ? 'var(--giallo-ferrari, #FFC72C)'
      : led === 'red' ? 'var(--rosso-corsa, #DC0000)'
      : 'var(--mn-text, #fafafa)';

    const section = (k) =>
      '<div class="mn-strip__section mn-hover-lift" onclick="scrollToWidget(\'' + k.target + '\')" role="button" tabindex="0">' +
        ledBulb(k.led) +
        '<div class="mn-strip__value" style="color:' + valColor(k.led) + '"' + (k.key ? ' data-kpi-key="' + k.key + '"' : '') + '>' + k.value + '</div>' +
        '<div class="mn-strip__label">' + k.label + '</div>' +
        (k.unit ? '<div class="mn-strip__dim">' + k.unit + '</div>' : '') +
      '</div>';

    const divider = '<div class="mn-strip__divider"></div>';
    const leftHtml = left.map(section).join(divider);
    const rightHtml = right.map(section).join(divider);

    const agentsRunning = Number(d.agents_running || 0);
    const plansActive = Number(d.plans_active || 0);

    const speedPod = (id, w, h) =>
      '<div class="mn-strip__section--pod" style="padding:2px 6px;">' +
        '<div class="mn-strip__gauge-ring">' +
          '<canvas id="' + id + '" width="' + w + '" height="' + h + '"></canvas>' +
        '</div>' +
      '</div>';

    const gaugeHtml =
      speedPod('speedo-plans', 110, 110) +
      '<div class="mn-strip__divider"></div>' +
      speedPod('speedo-agents', 160, 160) +
      '<div class="mn-strip__divider"></div>' +
      speedPod('speedo-mesh', 110, 110);

    bar.innerHTML =
      '<div class="mn-strip">' +
        '<div class="mn-strip__inner">' +
          leftHtml + divider + gaugeHtml + divider + rightHtml +
        '</div>' +
      '</div>';

    if (ns._meshSpeedInterval) clearInterval(ns._meshSpeedInterval);
    ns._meshSpeedInterval = null;
    ns.initMeshSpeedGauge();
    // Render the 3 speedometer gauges
    ns._renderSpeedoGauges(plansActive, agentsRunning, online, total);
    ns.renderTrendSparklines(d);
    ns.bindAndEmitKpi(d);
  };

  ns.buildSignalRow = function buildSignalRow(d) {
    const online = d.mesh_online || 0;
    const total = d.mesh_total || 0;
    const blocked = d.blocked || 0;
    const active = d.plans_active || 0;
    const agentsToday = Number(d.agents_today || 0);
    const meshOk = online === total && total > 0;
    const signals = [
      { label: 'Mesh', status: meshOk ? 'ok' : online > 0 ? 'warn' : 'danger', text: online + '/' + total + ' online' },
      { label: 'Plans', status: active > 0 ? 'warn' : 'ok', text: active + ' active' },
      { label: 'Blocked', status: blocked > 0 ? 'danger' : 'ok', text: blocked > 0 ? blocked + ' stuck' : 'clear' },
      { label: 'Agents', status: agentsToday > 0 ? 'ok' : 'warn', text: agentsToday + ' today' },
    ];
    const dotColor = {
      ok: 'var(--verde-racing, #00A651)',
      warn: 'var(--giallo-ferrari, #FFC72C)',
      danger: 'var(--rosso-corsa, #DC0000)',
    };
    return '<div class="mn-strip__legend">' +
      '<div class="mn-strip__legend-inner">' +
        signals.map((s) =>
          '<div class="mn-strip__legend-item">' +
            '<span class="mn-strip__legend-dot" style="background:' + (dotColor[s.status] || 'var(--grigio-medio)') + ';box-shadow:0 0 6px ' + (dotColor[s.status] || 'transparent') + ';"></span>' +
            '<span class="mn-strip__legend-key">' + s.label + '</span>' +
            '<span style="color:' + (dotColor[s.status] || 'var(--mn-text)') + '">' + s.text + '</span>' +
          '</div>'
        ).join('') +
        '<div class="mn-cockpit-sparklines" id="cockpit-sparklines"></div>' +
      '</div>' +
    '</div>';
  };

  ns._trendHistory = ns._trendHistory || { tokens: [], lines: [], cost: [] };

  ns.renderTrendSparklines = function renderTrendSparklines(d) {
    const api = ns.M?.();
    if (!api?.sparkline) return;
    const container = document.getElementById('cockpit-sparklines');
    if (!container) return;

    ns._trendHistory.tokens.push(Number(d.today_tokens || 0));
    ns._trendHistory.lines.push(Number(d.today_lines_changed || 0));
    ns._trendHistory.cost.push(Number(d.today_cost || 0));
    // Keep last 20 data points
    ['tokens', 'lines', 'cost'].forEach((k) => {
      if (ns._trendHistory[k].length > 20) ns._trendHistory[k].shift();
    });

    if (ns._trendHistory.tokens.length < 2) return;

    const p = api.palette?.() || {};
    const sparkOpts = { width: 80, height: 20, lineWidth: 1.5 };

    container.innerHTML = '';
    const items = [
      { data: ns._trendHistory.tokens, color: p.accent || '#FFC72C', label: 'Tokens' },
      { data: ns._trendHistory.lines, color: p.signalOk || '#00A651', label: 'Lines' },
      { data: ns._trendHistory.cost, color: p.signalDanger || '#DC0000', label: 'Cost' },
    ];
    items.forEach((item) => {
      const wrap = document.createElement('div');
      wrap.className = 'mn-cockpit-spark';
      wrap.innerHTML = '<span class="mn-cockpit-spark__label">' + item.label + '</span>';
      const canvas = document.createElement('canvas');
      canvas.width = sparkOpts.width;
      canvas.height = sparkOpts.height;
      canvas.style.cssText = 'width:80px;height:20px;display:block;';
      wrap.appendChild(canvas);
      container.appendChild(wrap);
      const opts = { ...sparkOpts, color: item.color };
      try {
        if (api.autoResize) {
          ns._autoResizeCleanups.push(api.autoResize(canvas, api.sparkline, item.data));
        } else {
          api.sparkline(canvas, item.data, opts);
        }
      } catch (_) {}
    });
  };

  ns.bindAndEmitKpi = function bindAndEmitKpi(d) {
    const api = ns.M?.();
    const mesh = `${d.mesh_online || 0}/${d.mesh_total || 0}`;
    const blocked = Number(d.blocked || 0);
    const cost = '$' + Number(d.today_cost || 0).toFixed(0);
    ns.emitData?.('kpi.plans_active', d.plans_active || 0);
    ns.emitData?.('kpi.mesh_online', mesh);
    ns.emitData?.('kpi.blocked', blocked);
    ns.emitData?.('kpi.cost', cost);
    if (!api || typeof api.bind !== 'function' || ns._kpiBound) return;
    ns._kpiBound = true;
    ['kpi.plans_active', 'kpi.mesh_online', 'kpi.blocked', 'kpi.cost'].forEach((key) => {
      ns.bindData?.(key, (value) => {
        const el = document.querySelector(`[data-kpi-key="${key}"]`);
        if (el) el.textContent = String(value);
      });
    });
  };

  ns._kpiDelta = function _kpiDelta(current, previous, label) {
    if (previous == null) return label;
    const diff = current - previous;
    if (diff === 0) return '<span style="color:var(--text-dim)">= ' + label + '</span>';
    const p = ns.M?.()?.palette?.();
    const color = diff > 0 ? (p?.signalOk || '#00A651') : (p?.signalDanger || '#DC0000');
    const arrow = diff > 0 ? '▲' : '▼';
    const abs = typeof fmt === 'function' ? fmt(Math.abs(diff)) : Math.abs(diff);
    return '<span style="color:' + color + '">' + arrow + ' ' + (diff > 0 ? '+' : '') + abs + '</span> ' + label;
  };

  ns.autoTicks = function autoTicks(max) {
    if (max <= 5) return Array.from({ length: max + 1 }, (_, i) => i);
    const count = max <= 20 ? 4 : 5;
    const step = max / count;
    const nice = step >= 100 ? Math.round(step / 100) * 100 : step >= 10 ? Math.round(step / 10) * 10 : Math.round(step);
    const ticks = [];
    for (let v = 0; v <= max; v += nice || 1) ticks.push(v);
    if (ticks[ticks.length - 1] !== max) ticks.push(max);
    return ticks;
  };

  ns.initMeshSpeedGauge = function initMeshSpeedGauge() {
    if (!ns.M?.()?.speedometer || !document.getElementById('mesh-speed-gauge')) return;
    ns.updateMeshSpeed();
    ns._meshSpeedInterval = setInterval(ns.updateMeshSpeed, 4000);
  };

  ns.updateMeshSpeed = function updateMeshSpeed() {
    if (!ns._active || !ns.M?.()?.speedometer) return;
    const canvas = document.getElementById('mesh-speed-gauge');
    if (!canvas) return;
    fetch('/api/mesh/traffic').then((r) => r.json()).then((data) => {
      if (!data?.ok) return;
      const now = Date.now() / 1000;
      const syncPeers = data.sync_peers || [];
      const heartbeats = data.heartbeats || [];
      let totalOps = 0; let activePeers = 0; let latencySum = 0; let latencyCount = 0;
      syncPeers.forEach((p) => {
        totalOps += (p.total_sent || 0) + (p.total_received || 0);
        if (p.latency_ms != null && p.latency_ms > 0 && (p.active || (p.last_sync_ago_s != null && p.last_sync_ago_s < 120))) {
          activePeers += 1;
          latencySum += p.latency_ms;
          latencyCount += 1;
        }
      });
      const avgLatency = latencyCount > 0 ? Math.round(latencySum / latencyCount) : 0;
      const onlineNodes = heartbeats.filter((h) => h.last_seen_ago_s < 60).length;
      let syncRate = 0;
      if (ns._meshPrev?.time) {
        const dt = now - ns._meshPrev.time;
        if (dt > 1) syncRate = Math.max(0, totalOps - ns._meshPrev.ops) / dt;
      }
      ns._meshPrev = { ops: totalOps, time: now };
      ns.renderMeshSpeedGauge(canvas, syncRate, avgLatency, activePeers, onlineNodes);
    }).catch(() => {});
  };

  ns.renderMeshSpeedGauge = function renderMeshSpeedGauge(canvas, syncRate, avgLatency, activePeers, onlineNodes) {
    if (!canvas || !ns.M?.()?.speedometer) return;
    const ctx = canvas.getContext('2d');
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    const maxRate = syncRate < 5 ? 20 : syncRate < 20 ? 50 : syncRate < 100 ? 200 : 1000;
    const p = ns.M?.()?.palette?.();
    const ok = p?.signalOk || '#00A651';
    const warn = p?.accent || '#FFC72C';
    const danger = p?.signalDanger || '#DC0000';
    const info = p?.info || '#4ea8de';
    const arcColor = syncRate > 10 ? ok : syncRate > 2 ? warn : avgLatency > 500 ? danger : info;
    ns.M().speedometer(canvas, {
      value: Math.round(syncRate * 10) / 10,
      max: maxRate,
      unit: 'sync/s',
      size: 'fluid',
      ticks: ns.autoTicks(maxRate),
      needleColor: danger,
      arcColor,
      bar: { value: onlineNodes, max: Math.max(onlineNodes, 3) },
      subLabel: (avgLatency > 0 ? avgLatency + 'ms' : '—') + ' · ' + activePeers + ' peers',
      animate: false,
    });
  };

  ns._renderSpeedoGauges = function _renderSpeedoGauges(plans, agents, meshOnline, meshTotal) {
    if (!ns.M?.()?.speedometer) return;
    const p = ns.M?.()?.palette?.();
    const ok = p?.signalOk || '#00A651';
    const warn = p?.accent || '#FFC72C';
    const danger = p?.signalDanger || '#DC0000';
    const info = p?.info || '#4ea8de';

    // Plans speedometer (small)
    const plansCanvas = document.getElementById('speedo-plans');
    if (plansCanvas) {
      const ctx = plansCanvas.getContext('2d');
      ctx.clearRect(0, 0, plansCanvas.width, plansCanvas.height);
      ns.M().speedometer(plansCanvas, {
        value: plans,
        max: Math.max(plans, 10),
        unit: 'active',
        size: 'fluid',
        ticks: [0, 2, 5, 10],
        needleColor: danger,
        arcColor: plans > 3 ? warn : plans > 0 ? ok : info,
        subLabel: 'Plans',
        animate: false,
      });
    }

    // Agents speedometer (central, large)
    const agentsCanvas = document.getElementById('speedo-agents');
    if (agentsCanvas) {
      const ctx = agentsCanvas.getContext('2d');
      ctx.clearRect(0, 0, agentsCanvas.width, agentsCanvas.height);
      const maxAgents = Math.max(agents, 20);
      ns.M().speedometer(agentsCanvas, {
        value: agents,
        max: maxAgents,
        unit: 'running',
        size: 'fluid',
        ticks: ns.autoTicks(maxAgents),
        needleColor: danger,
        arcColor: agents > 10 ? danger : agents > 3 ? warn : agents > 0 ? ok : info,
        subLabel: 'Agents',
        animate: false,
      });
    }

    // Mesh speedometer (small) — online nodes count
    const meshCanvas = document.getElementById('speedo-mesh');
    if (meshCanvas) {
      const ctx = meshCanvas.getContext('2d');
      ctx.clearRect(0, 0, meshCanvas.width, meshCanvas.height);
      const maxMesh = Math.max(meshTotal, 5);
      ns.M().speedometer(meshCanvas, {
        value: meshOnline,
        max: maxMesh,
        unit: 'online',
        size: 'fluid',
        ticks: Array.from({length: maxMesh + 1}, (_, i) => i),
        needleColor: danger,
        arcColor: meshOnline === meshTotal && meshTotal > 0 ? ok : meshOnline > 0 ? warn : danger,
        bar: { value: meshOnline, max: maxMesh },
        subLabel: meshOnline + '/' + meshTotal + ' nodes',
        animate: false,
      });
    }
  };

  ns.removeKpiEnhancements = function removeKpiEnhancements() {
    ns._lastKpiHash = '';
    if (ns._meshSpeedInterval) clearInterval(ns._meshSpeedInterval);
    ns._meshSpeedInterval = null;
    ns._meshPrev = null;
    const d = window.__kpiOverviewData;
    if (ns._orig.renderKpi && d) ns._orig.renderKpi(d);
  };
})();
