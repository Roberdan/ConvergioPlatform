function _substatusBadge(substatus) {
  const badges = {
    waiting_ci: { icon: Icons.clock(11), label: 'CI', color: 'var(--info, #4EA8DE)' },
    waiting_review: { icon: Icons.eye(11), label: 'Review', color: 'var(--accent, #FFC72C)' },
    waiting_merge: { icon: Icons.gitMerge(11), label: 'Merge', color: '#8B5CF6' },
    waiting_thor: { icon: Icons.shield(11), label: 'Thor', color: 'var(--warning, #D4622B)' },
    agent_running: { icon: Icons.cpu(11), label: 'Agent', color: 'var(--info, #4EA8DE)' },
  };
  const badge = badges[substatus];
  return badge
    ? `<span class="substatus-badge" style="color:${badge.color}" title="${esc(substatus)}">${badge.icon} ${badge.label}</span>`
    : '';
}
function _healthIcon(code) {
  const icons = {
    blocked: Icons.xCircle(14),
    stale: Icons.clock(14),
    stuck_deploy: Icons.alertTriangle(14),
    manual_required: Icons.alertTriangle(14),
    thor_stuck: Icons.zap(14),
    near_complete_stuck: Icons.shield(14),
    preflight_missing:
      '<svg width="14" height="14" viewBox="0 0 24 24" fill="#ffb700" style="vertical-align:-2px"><path d="M19 3H5c-1.1 0-2 .9-2 2v14l4-4h12c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm-6 9h-2V6h2v6zm0 4h-2v-2h2v2z"/></svg>',
    preflight_stale:
      '<svg width="14" height="14" viewBox="0 0 24 24" fill="#ffb700" style="vertical-align:-2px"><path d="M13 3a9 9 0 1 0 8.95 10h-2.02A7 7 0 1 1 13 5v4l5-5-5-5v4z"/></svg>',
    preflight_context:
      '<svg width="14" height="14" viewBox="0 0 24 24" fill="#ffb700" style="vertical-align:-2px"><path d="M12 3L1 9l11 6 9-4.91V17h2V9L12 3zm0 13L3.74 11.5 12 7l8.26 4.5L12 16z"/></svg>',
    preflight_blocked:
      '<svg width="14" height="14" viewBox="0 0 24 24" fill="#ee3344" style="vertical-align:-2px"><path d="M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2zm5 11H7v-2h10z"/></svg>',
  };
  return icons[code] || icons.blocked;
}
function _renderHealthAlerts(health, planId, peer) {
  if (!health || !health.length) return '';
  const hasCritical = health.some((h) => h.severity === 'critical');
  let html = `<div class="plan-health-bar ${hasCritical ? 'health-critical' : 'health-warning'}" onclick="event.stopPropagation()">`;
  html += `<div class="plan-health-alerts">`;
  health.forEach((h) => {
    html += `<div class="plan-health-item plan-health-${h.severity}">${_healthIcon(h.code)} <span>${esc(h.message)}</span></div>`;
  });
  const actionable = health.some(
    (h) => h.severity === 'critical' || h.code === 'thor_stuck' || h.code === 'preflight_blocked',
  );
  if (!actionable) {
    html += `</div></div>`;
    return html;
  }
  html += `</div><div class="plan-health-actions">`;
  const hasThor = health.some((h) => h.code === 'thor_stuck');
  if (hasThor) {
    html += `<button class="plan-health-btn plan-health-btn-thor mn-btn mn-btn--sm" onclick="event.stopPropagation();runThorValidation(${planId})" title="Run Thor validation on submitted tasks"><svg width="16" height="16" viewBox="0 0 24 24" fill="var(--gold)"><path d="M12 1L8 5v3H5l-2 4h4l-3 11h2l7-9H9l3-5h5l3-4h-4l1-4h-5z"/></svg> Run Thor</button>`;
  }
  html += `<button class="plan-health-btn plan-health-btn-resume mn-btn mn-btn--sm" onclick="event.stopPropagation();resumePlanExecution(${planId},'${esc(peer || 'local')}')" title="Resume/fix plan execution"><svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><polygon points="5,3 19,12 5,21"/></svg> Resume</button>`;
  html += `<button class="plan-health-btn plan-health-btn-term mn-btn mn-btn--sm" onclick="event.stopPropagation();openPlanTerminal(${planId},'${esc(peer || 'local')}')" title="Open terminal on plan"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="4 17 10 11 4 5"></polyline><line x1="12" y1="19" x2="20" y2="19"></line></svg> Debug</button>`;
  html += `</div></div>`;
  return html;
}
window.openPlanTerminal = function (planId, peer) {
  if (typeof termMgr !== 'undefined') {
    const session = 'plan-' + planId;
    termMgr.open(
      peer === 'local' ||
        peer === ((window.DashboardState && window.DashboardState.localPeerName) || 'local')
        ? 'local'
        : peer,
      'Plan #' + planId,
      session,
    );
  }
};
window.resumePlanExecution = function (planId, peer) {
  const meshArr = Array.isArray(window.DashboardState?.lastMeshData) ? window.DashboardState.lastMeshData : [];
  const planData = meshArr
    .flatMap((n) => (n.plans || []).map((p) => ({ ...p, node: n.peer_name })))
    .find((p) => p.id === planId);
  const assignedHost = planData ? planData.node : peer || 'local';
  const target =
    assignedHost === 'local' ||
    assignedHost === ((window.DashboardState && window.DashboardState.localPeerName) || 'local')
      ? 'local'
      : assignedHost;

  if (typeof termMgr === 'undefined') return;

  const session = 'plan-' + planId;
  const tabId = termMgr.open(target, 'Resume #' + planId, session);

  const tab = termMgr.tabs.find((t) => t.id === tabId);
  if (!tab) return;

  const sendResume = () => {
    if (tab.ws && tab.ws.readyState === WebSocket.OPEN) {
      const cmd = 'cd ~/.claude && claude --model sonnet -p "/execute ' + planId + '"\n';
      tab.ws.send(new TextEncoder().encode(cmd));
    }
  };

  let attempts = 0;
  const waitForOpen = setInterval(() => {
    attempts++;
    if (tab.ws && tab.ws.readyState === WebSocket.OPEN) {
      clearInterval(waitForOpen);
      setTimeout(sendResume, 800);
    } else if (attempts > 50) {
      clearInterval(waitForOpen);
    }
  }, 100);
};
function _elapsedTime(ts) {
  if (!ts) return '';
  const d = new Date(ts.endsWith('Z') || ts.includes('+') ? ts : ts + 'Z');
  if (Number.isNaN(d.getTime())) return '';
  const ms = Date.now() - d.getTime();
  if (ms < 0) return '';
  const s = Math.floor(ms / 1000), m = Math.floor(s / 60), h = Math.floor(m / 60), days = Math.floor(h / 24);
  if (days > 0) return `${days}d ${h % 24}h`;
  if (h > 0) return `${h}h ${m % 60}m`;
  if (m > 0) return `${m}m`;
  return `${s}s`;
}
function _elapsedBetween(start, end) {
  if (!start || !end) return '';
  const d1 = new Date(start.endsWith('Z') || start.includes('+') ? start : start + 'Z');
  const d2 = new Date(end.endsWith('Z') || end.includes('+') ? end : end + 'Z');
  if (Number.isNaN(d1.getTime()) || Number.isNaN(d2.getTime())) return '';
  const ms = d2.getTime() - d1.getTime();
  if (ms < 0) return '';
  const s = Math.floor(ms / 1000), m = Math.floor(s / 60), h = Math.floor(m / 60);
  if (h > 0) return `${h}h ${m % 60}m`;
  if (m > 0) return `${m}m ${s % 60}s`;
  return `${s}s`;
}
function _statusDotWithDs(status) {
  const normalizedStatus = _normalizePlanStatus(status);
  const ledClass =
    normalizedStatus === 'done'
      ? 'mn-led--green'
      : normalizedStatus === 'in_progress'
        ? 'mn-led--amber'
        : normalizedStatus === 'blocked'
          ? 'mn-led--red'
          : normalizedStatus === 'submitted'
            ? 'mn-led--amber'
            : 'mn-led--off';
  return `<span class="mn-led ${ledClass}"><span class="mn-led__housing"><span class="mn-led__bulb"></span></span></span>`;
}
function _normalizePlanStatus(status) {
  if (status === 'doing') return 'in_progress';
  if (status === 'todo') return 'pending';
  return status || 'pending';
}
function _statusVisual(status) {
  switch (_normalizePlanStatus(status)) {
    case 'in_progress':
      return { color: 'var(--giallo-ferrari, #FFC72C)', badge: 'background:rgba(255,199,44,0.15);color:#FFC72C' };
    case 'done':
      return { color: 'var(--verde-racing, #00A651)', badge: 'background:rgba(0,166,81,0.15);color:#00A651' };
    case 'blocked':
      return { color: 'var(--rosso-corsa, #DC0000)', badge: 'background:rgba(220,0,0,0.15);color:#DC0000' };
    case 'submitted':
      return { color: '#4EA8DE', badge: 'background:rgba(78,168,222,0.15);color:#4EA8DE' };
    default:
      return { color: 'var(--grigio-medio, #616161)', badge: '' };
  }
}
function _progressFillClass(pct) {
  if (pct >= 75) return 'mn-progress__fill--green';
  if (pct >= 50) return 'mn-progress__fill--yellow';
  return 'mn-progress__fill--red';
}

function _renderOnePlan(m) {
  const p = m.plan,
    health = m.health || [],
    rawPct = p.tasks_total > 0 ? Math.round((100 * p.tasks_done) / p.tasks_total) : 0,
    allValidated =
      m.waves && m.waves.length && m.waves.every((w) => w.validated_at || w.status === 'pending'),
    pct = rawPct >= 100 && !allValidated ? 95 : rawPct,
    planStatus = _normalizePlanStatus(p.status),
    ringColor = _statusVisual(planStatus).color,
    hostName = p.execution_peer || _resolveHost(p.execution_host),
    isRemote =
      hostName &&
      hostName !== 'local' &&
      hostName !== ((window.DashboardState && window.DashboardState.localPeerName) || 'local'),
    blocked = (m.tasks || []).filter((t) => t.status === 'blocked').length,
    running = (m.tasks || []).filter((t) => t.status === 'in_progress').length,
    submitted = (m.tasks || []).filter((t) => t.status === 'submitted').length,
    hasCritical = health.some((h) => h.severity === 'critical'),
    cardStyleParts = [`border-left:3px solid ${_statusVisual(planStatus).color}`],
    nodeLabel = isRemote
      ? `<span class="mn-badge mn-badge--neutral host-badge-prominent">${esc(hostName)}</span>`
      : hostName && hostName !== 'local'
        ? `<span class="mn-badge mn-badge--neutral host-badge-local">${esc(hostName)}</span>`
        : '';
  if (planStatus === 'in_progress') cardStyleParts.push('background:rgba(255,199,44,0.03)');
  if (planStatus === 'blocked') cardStyleParts.push('background:rgba(220,0,0,0.03)');
  if (hasCritical) cardStyleParts.push('box-shadow: 0 0 12px rgba(220,0,0,0.15)');

  // --- HEADER: ID + Name + Status ---
  let html = `<div class="mn-mission-card mission-plan mn-card mn-card-dark mn-hover-lift mn-anim-fadeInUp${hasCritical ? ' mission-plan-critical' : health.length ? ' mission-plan-warning' : ''}" style="${cardStyleParts.join(';')}" onclick="filterTasks(${p.id})">`;
  html += `<div class="mission-header">`;
  html += `<span class="mission-id">#${p.id}</span>`;
  html += `<span class="mission-name">${esc(p.name)}</span>`;
  html += _statusDotWithDs(p.status === 'doing' ? 'in_progress' : p.status);
  if (health.length) {
    html += `<span class="health-badge health-badge-${hasCritical ? 'critical' : 'warning'} ${hasCritical ? 'mn-badge mn-badge--danger' : 'mn-badge mn-badge--warning'}" title="${health.map((h) => h.message).join('; ')}">${hasCritical ? 'ALERT' : 'WARN'}</span>`;
  }
  html += `</div>`;

  // --- META ROW: project, host, elapsed, lines ---
  html += `<div class="mission-meta">`;
  if (p.project_name) html += `<span class="badge badge-project mn-badge mn-badge--info">${esc(p.project_name)}</span>`;
  html += nodeLabel;
  const elapsed = p.status === 'doing' ? _elapsedTime(p.started_at) : '';
  if (elapsed) html += `<span class="mission-elapsed" title="Started ${esc(p.started_at || '')}">${typeof Icons !== 'undefined' ? Icons.clock(12) : ''} ${elapsed}</span>`;
  if (p.lines_added || p.lines_removed) {
    html += `<span class="mission-lines">`;
    if (p.lines_added) html += `<span class="lines-added">+${p.lines_added}</span>`;
    if (p.lines_removed) html += `<span class="lines-removed"> −${p.lines_removed}</span>`;
    html += `</span>`;
  }
  html += `</div>`;

  // --- HEALTH ALERTS ---
  html += _renderHealthAlerts(health, p.id, hostName);

  // --- SUMMARY ---
  if (p.human_summary) html += `<div class="mission-summary">${esc(p.human_summary)}</div>`;

  // --- PROGRESS: ring + bar + status badges ---
  html += `<div class="mission-progress">`;
  html += _progressRing(pct, 56, ringColor);
  html += `<div class="mission-progress-bars">`;
  html += `<div class="mission-progress-label"><span>Tasks ${p.tasks_done || 0}/${p.tasks_total || 0}</span><span style="color:${ringColor}">${pct}%</span></div>`;
  html += `<div class="mn-progress mission-progress-track"><div class="mn-progress__fill ${_progressFillClass(pct)}" style="width:${pct}%"></div></div>`;
  html += `<div class="mission-status-tags">`;
  if (running > 0) html += `<span class="mn-mission-status mn-mission-status--running" style="${_statusVisual('in_progress').badge}">${running} running</span>`;
  if (blocked > 0) html += `<span class="mn-mission-status mn-mission-status--blocked" style="${_statusVisual('blocked').badge}">${blocked} blocked</span>`;
  if (submitted > 0) html += `<span class="mn-mission-status mn-mission-status--submitted" style="${_statusVisual('submitted').badge}">${submitted} submitted</span>`;
  html += `</div></div></div>`;

  // --- WAVES ---
  html += typeof renderWaveGantt === 'function' ? renderWaveGantt(m.waves, p) : '';

  // --- TASK FLOW ---
  html += typeof renderTaskFlow === 'function' ? renderTaskFlow(m.tasks, p) : '';

  // --- ACTION BUTTONS (bottom) ---
  html += `<div class="mission-actions">`;
  html += `<button class="mission-delegate-btn mn-btn mn-btn--sm" onclick="event.stopPropagation();showDelegatePlanDialog(${p.id},'${esc(p.name)}')" title="Delegate"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 2L11 13"/><path d="M22 2L15 22L11 13L2 9L22 2Z"/></svg> Delegate</button>`;
  if (p.status === 'todo') {
    html += `<button class="mission-start-btn mn-btn mn-btn--sm" onclick="event.stopPropagation();showStartPlanDialog(${p.id},'${esc(p.name)}')" title="Start"><svg width="13" height="13" viewBox="0 0 24 24" fill="currentColor"><polygon points="5,3 19,12 5,21"/></svg> Start</button>`;
  }
  if (p.status === 'doing') {
    html += `<button class="mission-stop-btn mn-btn mn-btn--sm" onclick="event.stopPropagation();stopPlan(${p.id})" title="Stop"><svg width="13" height="13" viewBox="0 0 24 24" fill="currentColor"><rect x="4" y="4" width="16" height="16" rx="2"/></svg> Stop</button>`;
  }
  if (p.status !== 'done') {
    html += `<button class="mission-reset-btn mn-btn mn-btn--sm" onclick="event.stopPropagation();resetPlan(${p.id})" title="Reset"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M1 4v6h6"/><path d="M3.51 15a9 9 0 105.64-11.36L3 10"/></svg> Reset</button>`;
    html += `<button class="mission-cancel-btn mn-btn mn-btn--sm" onclick="event.stopPropagation();cancelPlan(${p.id})" title="Cancel"><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 6h18M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/></svg> Cancel</button>`;
  }
  html += `</div>`;

  return html + '</div>';
}
function _formatLastMissionTs(ts) {
  if (!ts) return '';
  const d = new Date(ts);
  if (Number.isNaN(d.getTime())) return esc(String(ts).replace('T', ' ').slice(0, 16));
  return d.toLocaleString('en-GB', {
    day: '2-digit',
    month: 'short',
    hour: '2-digit',
    minute: '2-digit',
  });
}
function _renderLastMission(m) {
  const p = m.plan || {},
    pct = p.tasks_total > 0 ? Math.round((100 * p.tasks_done) / p.tasks_total) : 0,
    planStatus = _normalizePlanStatus(p.status),
    statusVisual = _statusVisual(planStatus),
    hostName = p.execution_peer || _resolveHost(p.execution_host),
    isRemote =
      hostName &&
      hostName !== 'local' &&
      hostName !== ((window.DashboardState && window.DashboardState.localPeerName) || 'local'),
    nodeLabel = isRemote
      ? `<span class="mn-badge mn-badge--neutral host-badge-prominent">${esc(hostName)}</span>`
      : hostName && hostName !== 'local'
        ? `<span class="mn-badge mn-badge--neutral host-badge-local">${esc(hostName)}</span>`
        : '',
    finishedAt = p.finished_at || p.completed_at || p.cancelled_at || '',
    statusCls = planStatus === 'done' ? 'mn-mission-status mn-mission-status--done' : planStatus === 'blocked' ? 'mn-mission-status mn-mission-status--blocked' : planStatus === 'in_progress' ? 'mn-mission-status mn-mission-status--running' : 'mn-mission-status mn-mission-status--submitted',
    cardStyleParts = [`border-left:3px solid ${statusVisual.color}`];
  if (planStatus === 'in_progress') cardStyleParts.push('background:rgba(255,199,44,0.03)');
  if (planStatus === 'blocked') cardStyleParts.push('background:rgba(220,0,0,0.03)');
  let html = `<div class="mn-mission-card mission-plan mn-card mn-card-dark mn-hover-lift mn-anim-fadeInUp" style="${cardStyleParts.join(';')}" onclick="openPlanSidebar(${p.id})"><div style="margin-bottom:6px"><span class="mission-id">#${p.id}</span><span class="mission-name">&nbsp;${esc(p.name || '')}</span>${_statusDotWithDs(p.status)}${nodeLabel}${p.project_name ? `<span class="badge badge-project mn-badge mn-badge--info">${esc(p.project_name)}</span>` : ''}<span class="${statusCls}" style="${statusVisual.badge}">${esc((p.status || '').replace(/_/g, ' '))}</span></div>${p.human_summary ? `<div class="mission-summary">${esc(p.human_summary)}</div>` : ''}<div class="mission-progress">${_progressRing(pct, 56, statusVisual.color)}<div class="mission-progress-bars"><div class="mission-progress-label"><span>Overall status</span><span style="color:${statusVisual.color}">${esc((p.status || 'done').toUpperCase())}</span></div><div class="mn-progress mission-progress-track"><div class="mn-progress__fill ${_progressFillClass(pct)}" style="width:${pct}%"></div></div><div style="display:flex;gap:8px;font-size:9px;color:var(--text-dim);margin-top:4px;flex-wrap:wrap"><span>${finishedAt ? `Finished ${_formatLastMissionTs(finishedAt)}` : ''}</span><span>${p.tasks_total ? `${p.tasks_done || 0}/${p.tasks_total || 0} tasks` : ''}</span></div></div></div>`;
  return html + '</div>';
}
function renderMission(data) {
  const st = window.DashboardState;
  st.lastMissionData = data;
  st.allMissionPlans = data && data.plans ? data.plans : data && data.plan ? [data] : [];
  window._dashboardPlans = st.allMissionPlans;
  const activePlans = st.allMissionPlans.filter(m => m.plan && m.plan.status !== 'cancelled' && m.plan.status !== 'done');
  const cancelledPlans = st.allMissionPlans.filter(m => m.plan && (m.plan.status === 'cancelled' || m.plan.status === 'done'));
  if (!activePlans.length && !cancelledPlans.length) {
    $('#mission-content').innerHTML = '<span style="color:#5a6080">No active mission</span>';
    $('#task-table tbody').innerHTML = '';
    return;
  }
  let html = activePlans.map(_renderOnePlan).join('');
  if (cancelledPlans.length) {
    html += `<details class="cancelled-parking-lot"><summary style="color:var(--text-dim);font-size:11px;cursor:pointer;margin-top:10px">Cancelled (${cancelledPlans.length})</summary><div style="opacity:0.5">${cancelledPlans.map(_renderOnePlan).join('')}</div></details>`;
  }
  $('#mission-content').innerHTML = html;
  renderTaskPipeline();
}
function renderLastMissions(data) {
  const el = $('#last-missions-content');
  const plans = data && data.plans ? data.plans : [];
  if (!el) return;
  if (!plans.length) {
    el.innerHTML = '<span style="color:#5a6080">No missions completed in the last 24 hours</span>';
    return;
  }
  el.innerHTML = plans.map(_renderLastMission).join('');
}

window.runThorValidation = async function (planId) {
  try {
    const r = await fetch(`/api/plans/${planId}/validate`, { method: 'POST' });
    const d = await r.json();
    if (d.ok) {
      showToast('Thor', `Validated ${d.validated || 0} tasks`, null, 'info');
    } else {
      showToast('Thor', d.error || 'Validation failed', null, 'error');
    }
    if (typeof refreshAll === 'function') refreshAll();
  } catch (e) {
    showToast('Thor', 'Request failed: ' + e.message, null, 'error');
  }
};

window.renderMission = renderMission;
window.renderLastMissions = renderLastMissions;
