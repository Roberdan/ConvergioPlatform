const $ = (s) => document.querySelector(s);
const state = (window.DashboardState = window.DashboardState || {
  hostToPeer: {},
  localPeerName: "local",
  lastMissionData: null,
  lastMeshData: null,
  allMissionPlans: [],
  filteredPlanId: null,
  pullInProgress: false,
  notifLastId: 0,
  refreshTimer: null,
  refreshIdx: 2,
  _currentPullES: null,
  currentZoom: parseInt(localStorage.getItem("dashZoom") || "100", 10),
  firstDataLoadDone: false,
});

Object.defineProperty(window, "lastMissionData", {
  get: () => state.lastMissionData,
  set: (v) => (state.lastMissionData = v),
  configurable: true,
});
Object.defineProperty(window, "lastMeshData", {
  get: () => state.lastMeshData,
  set: (v) => (state.lastMeshData = v),
  configurable: true,
});

async function fetchJson(url) {
  try {
    const resp = await fetch(url);
    if (!resp.ok) {
      DashLog.error(`API ${resp.status}`, url, `${resp.statusText}`);
      return null;
    }
    return await resp.json();
  } catch (e) {
    DashLog.error('API fetch failed', url, e.message);
    return null;
  }
}

function _resolveHost(host) {
  if (!host) return "unknown";
  if (state.hostToPeer[host]) return state.hostToPeer[host];
  // Fuzzy: strip suffixes and special chars
  const clean = host.toLowerCase().replace(/[-_]/g, "").replace(/\.(lan|local|tailnet)$/i, "");
  for (const [k, n] of Object.entries(state.hostToPeer)) {
    const cleanKey = k.toLowerCase().replace(/[-_]/g, "").replace(/\.(lan|local|tailnet)$/i, "");
    if (cleanKey === clean) return n;
  }
  return host;
}

function _pullRemoteDb() {
  if (state.pullInProgress) return;
  state.pullInProgress = true;
  try {
    if (state._currentPullES) {
      state._currentPullES.close();
      state._currentPullES = null;
    }
    const es = new EventSource('/api/mesh/pull-db');
    state._currentPullES = es;
    const badge = document.getElementById('sync-badge');

    // Generic message handler (server may send unnamed 'message' events)
    es.addEventListener('message', (e) => {
      try {
        const data = JSON.parse(e.data);
        if (data && Array.isArray(data.synced) && typeof applyMeshSyncBadges === 'function') {
          applyMeshSyncBadges(data.synced);
        }
      } catch (err) {
        // ignore malformed event data
      }
    });

    // Progress events sent during sync
    es.addEventListener('progress', (e) => {
      try {
        const data = JSON.parse(e.data);
        if (data && Array.isArray(data.synced)) {
          const ok = data.synced.filter((s) => s.ok).length;
          if (ok > 0 && badge) {
            badge.textContent = `↓ ${ok} synced`;
            badge.style.display = 'inline';
            setTimeout(() => (badge.style.display = 'none'), 3000);
          }
        }
      } catch (err) {}
    });

    // Final event: close the EventSource and clear state
    es.addEventListener('done', (e) => {
      try {
        const data = e.data ? JSON.parse(e.data) : null;
        if (data && Array.isArray(data.synced)) {
          const ok = data.synced.filter((s) => s.ok).length;
          if (ok > 0 && badge) {
            badge.textContent = `↓ ${ok} synced`;
            badge.style.display = 'inline';
            setTimeout(() => (badge.style.display = 'none'), 3000);
          }
        }
      } catch (err) {}
      try { es.close(); } catch (e) {}
      if (state._currentPullES === es) state._currentPullES = null;
      state.pullInProgress = false;
    });

    es.addEventListener('error', (e) => {
      try { es.close(); } catch (err) {}
      if (state._currentPullES === es) state._currentPullES = null;
      console.error('[Dashboard] pull-db SSE error', e);
      state.pullInProgress = false;
    });
  } catch (err) {
    state._currentPullES = null;
    state.pullInProgress = false;
  }
}

async function refreshAll() {
  const _safe = (label, fn) => { try { fn(); } catch (e) { DashLog.error(`Render: ${label}`, '', e.message || e); console.error(`[Dashboard] ${label} render error:`, e); } };
  const t0 = performance.now();
  const activityFeedState = {
    githubLoaded: false,
    nightlyLoaded: false,
    githubData: [],
    nightlyData: [],
  };
  const maybeMergeActivity = () => {
    if (!activityFeedState.githubLoaded || !activityFeedState.nightlyLoaded) return;
    _safe("activityFeedMerge", () => {
      if (typeof mergeActivityEvents === "function") {
        mergeActivityEvents(activityFeedState.githubData, activityFeedState.nightlyData);
      }
    });
  };

  // Each widget fetches and renders independently — no waiting for others
  const widgetLoaders = [
    // KPI + Overview (slowest — git log calls)
    fetchJson("/api/overview").then(ov => {
      if (!ov) return;
      // mesh counts patched after mesh loads
      if (state._lastMeshPeers) {
        ov.mesh_online = state._lastMeshPeers.filter(p => p.is_online).length;
        ov.mesh_total = state._lastMeshPeers.length;
      }
      _safe("kpi", () => { if (typeof renderKpi === "function") renderKpi(ov); });
      state._lastOverview = ov;
    }),

    // Mission widget
    fetchJson("/api/mission").then(mission => {
      _safe("mission", () => { if (mission && typeof renderMission === "function") renderMission(mission); });
    }),

    // Token charts
    fetchJson("/api/tokens/daily").then(daily => {
      _safe("tokenChart", () => { if (daily && typeof renderTokenChart === "function") renderTokenChart(daily); });
    }),

    fetchJson("/api/tokens/models").then(models => {
      _safe("modelChart", () => { if (models && typeof renderModelChart === "function") renderModelChart(models); });
    }),

    // Mesh strip
    fetchJson("/api/mesh").then(mesh => {
      let meshPeers = [];
      if (Array.isArray(mesh)) {
        meshPeers = mesh;
      } else if (mesh && Array.isArray(mesh.peers)) {
        meshPeers = mesh.peers;
        if (mesh.daemon_ws) state.daemonWsUrl = mesh.daemon_ws;
        if (mesh.local_node) state.localNodeName = mesh.local_node;
      }
      state._lastMeshPeers = meshPeers;
      if (Array.isArray(meshPeers)) {
        state.localPeerName = meshPeers.find(p => p.is_local)?.peer_name || "local";
        state.hostToPeer = {};
        meshPeers.forEach(p => {
          const name = p.peer_name || p.name;
          if (name) {
            state.hostToPeer[name] = name;
            if (p.dns_name) state.hostToPeer[p.dns_name] = name;
            if (p.ssh_alias) state.hostToPeer[p.ssh_alias] = name;
            if (p.tailscale_ip) state.hostToPeer[p.tailscale_ip] = name;
            if (p.is_local) state.hostToPeer.local = name;
            if (p.is_local && Array.isArray(p.hostname_aliases)) {
              p.hostname_aliases.forEach(alias => { if (alias) state.hostToPeer[alias] = name; });
            }
          }
        });
      }
      _safe("meshStrip", () => {
        if (typeof renderMeshStrip === "function") renderMeshStrip(meshPeers);
        lastMeshData = meshPeers;
        if (typeof renderGitHubActivity === "function") renderGitHubActivity();
        else if (typeof renderEventFeed === "function") renderEventFeed();
      });
      // Update KPI mesh counts if overview already loaded
      if (state._lastOverview) {
        state._lastOverview.mesh_online = meshPeers.filter(p => p.is_online).length;
        state._lastOverview.mesh_total = meshPeers.length;
        _safe("kpi-mesh", () => { if (typeof renderKpi === "function") renderKpi(state._lastOverview); });
      }
      fetch("/api/mesh/sync-status").then(r => r.json())
        .then(items => typeof applyMeshSyncBadges === "function" && applyMeshSyncBadges(items))
        .catch(() => null);
    }),

    // History
    fetchJson("/api/history").then(history => {
      _safe("history", () => { if (history && typeof renderHistory === "function") renderHistory(history); });
    }),

    // Recent missions
    fetchJson("/api/missions/recent").then(recent => {
      _safe("recentMissions", () => { if (recent && typeof renderLastMissions === "function") renderLastMissions(recent); });
    }),

    // Task distribution
    fetchJson("/api/tasks/distribution").then(dist => {
      _safe("dist", () => { if (dist && typeof renderDist === "function") renderDist(dist); });
    }),

    // GitHub events (for merged activity feed)
    (() => {
      const plans = Array.isArray(state.allMissionPlans) ? state.allMissionPlans : [];
      const firstPlan = plans.find((row) => row?.plan?.project_id) || plans[0];
      const projectId = firstPlan?.plan?.project_id || state.lastMissionData?.plan?.project_id || "";
      if (!projectId) {
        activityFeedState.githubData = [];
        activityFeedState.githubLoaded = true;
        maybeMergeActivity();
        return Promise.resolve();
      }
      return fetchJson(`/api/github/events/${encodeURIComponent(projectId)}`)
        .then(githubPayload => {
          const githubData = [];
          if (Array.isArray(githubPayload)) githubData.push(...githubPayload);
          if (Array.isArray(githubPayload?.remote_events)) githubData.push(...githubPayload.remote_events);
          if (Array.isArray(githubPayload?.local_events)) {
            githubData.push(...githubPayload.local_events.map(e => ({
              ...e,
              type: e.type || e.event_type,
              created_at: e.created_at || e.event_at,
            })));
          }
          activityFeedState.githubData = githubData;
        })
        .finally(() => {
          activityFeedState.githubLoaded = true;
          maybeMergeActivity();
        });
    })(),

    // Nightly jobs
    fetchJson("/api/nightly/jobs").then(nightly => {
      _safe("nightlyJobs", () => { if (typeof renderNightlyJobs === "function") renderNightlyJobs(nightly); });
      const nightlyData = [];
      if (Array.isArray(nightly)) nightlyData.push(...nightly);
      if (nightly?.latest) nightlyData.push(nightly.latest);
      if (Array.isArray(nightly?.history)) nightlyData.push(...nightly.history);
      activityFeedState.nightlyData = nightlyData;
      activityFeedState.nightlyLoaded = true;
      maybeMergeActivity();
    }).catch(() => {
      activityFeedState.nightlyData = [];
      activityFeedState.nightlyLoaded = true;
      maybeMergeActivity();
    }),
    // Plan timeline
    (typeof window.renderTimeline === 'function') ? window.renderTimeline() : Promise.resolve(),

    // Mesh latency speedometers
    (typeof window.renderMeshLatency === 'function') ? window.renderMeshLatency() : Promise.resolve(),
  ];

  _safe("ideaJarWidget", () => { if (typeof renderIdeaJarWidget === "function") renderIdeaJarWidget(); });

  // Wait for all to settle (don't fail if one widget fails)
  await Promise.allSettled(widgetLoaders);

  // Kanban needs allMissionPlans populated by mission fetch above
  _safe("kanban", () => { if (typeof renderKanban === "function") renderKanban(); });

  // IPC widgets (render into dashboard grid containers if present)
  _safe("ipcBudget", () => { const c = document.getElementById('ipc-budget-container'); if (c && typeof renderIpcBudget === "function") renderIpcBudget(c); });
  _safe("ipcRouter", () => { const c = document.getElementById('ipc-router-container'); if (c && typeof renderIpcRouter === "function") renderIpcRouter(c); });
  _safe("ipcSkills", () => { const c = document.getElementById('ipc-skills-container'); if (c && typeof renderIpcSkills === "function") renderIpcSkills(c); });
  _safe("ipcModels", () => { const c = document.getElementById('ipc-models-container'); if (c && typeof renderIpcModels === "function") renderIpcModels(c); });

  const lu = $("#last-update");
  if (lu) lu.textContent = `Updated: ${new Date().toLocaleTimeString()}`;
  DashLog.info('refreshAll', '', `${Math.round(performance.now() - t0)}ms`);
  _pullRemoteDb();
  if (!state.firstDataLoadDone) {
    state.firstDataLoadDone = true;
    document.querySelectorAll(".mn-widget.mn-widget--loading").forEach(el => el.classList.remove("mn-widget--loading"));
  }
}

function updateClock() {
  const el = $("#clock");
  if (!el) return;
  el.textContent = new Date().toLocaleString("en-GB", {
    day: "2-digit",
    month: "short",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

const ZOOM_STEP = 10;
const ZOOM_MIN = 60;
const ZOOM_MAX = 160;
function applyZoom(z) {
  state.currentZoom = Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, z));
  document.body.style.zoom = state.currentZoom / 100;
  const label = document.getElementById("zoom-level");
  if (label) label.textContent = `${state.currentZoom}%`;
  localStorage.setItem("dashZoom", String(state.currentZoom));
}
window.dashZoom = (dir) => (dir === 0 ? applyZoom(100) : applyZoom(state.currentZoom + dir * ZOOM_STEP));

const REFRESH_STEPS = [0, 1, 2, 5, 10, 15, 30, 60, 120]; // 0 = manual
state.refreshIdx = REFRESH_STEPS.indexOf(parseInt(localStorage.getItem("dashRefresh") || "0", 10));
if (state.refreshIdx === -1) state.refreshIdx = 0; // default to manual
function applyRefresh() {
  const sec = REFRESH_STEPS[state.refreshIdx];
  localStorage.setItem("dashRefresh", String(sec));
  const label = document.getElementById("refresh-label");
  if (state.refreshTimer) { clearInterval(state.refreshTimer); state.refreshTimer = null; }
  if (sec === 0) {
    if (label) { label.textContent = "Manual"; label.className = "refresh-label-manual"; }
    if (window.PollScheduler) window.PollScheduler.unregister("dashboard.refreshAll");
  } else {
    if (label) { label.textContent = sec < 60 ? `${sec}s` : `${sec / 60}m`; label.className = "refresh-label-auto"; }
    if (window.PollScheduler) {
      window.PollScheduler.register("dashboard.refreshAll", refreshAll, sec * 1000, ["overview", "admin", "brain", "ideajar"]);
      window.PollScheduler.setInterval("dashboard.refreshAll", sec * 1000);
    } else {
      state.refreshTimer = setInterval(refreshAll, sec * 1000);
    }
  }
}
window.changeRefresh = (dir) => {
  state.refreshIdx = Math.max(0, Math.min(REFRESH_STEPS.length - 1, state.refreshIdx + dir));
  applyRefresh();
};

window.openAllTerminals = function () {
  if (typeof termMgr === "undefined") return;
  const online = (state.lastMeshData || []).filter((p) => p.is_online);
  if (!online.length) return typeof showOutputModal === "function" && showOutputModal("Terminals", "No online mesh nodes");
  online.forEach((p) => termMgr.open(p.peer_name, p.peer_name, "Convergio"));
  termMgr.setMode(online.length > 1 ? "grid" : "dock");
};

function handleHashRoute() {
  const m = location.hash.match(/^#plan\/(\d+)/);
  if (!m) return;
  const id = parseInt(m[1], 10);
  if (typeof filterTasks === "function") filterTasks(id);
  const card = document.querySelector(`.mission-plan[onclick*="${id}"]`);
  if (!card) return;
  card.scrollIntoView({ behavior: "smooth", block: "center" });
  card.classList.add("highlight-pulse");
  setTimeout(() => card.classList.remove("highlight-pulse"), 3000);
}

const DASH_SECTIONS = ["dashboard-main-section", "dashboard-admin-section", "dashboard-chat-section", "dashboard-brain-section", "dashboard-ideajar-section", "dashboard-ipc-section", "dashboard-intelligence-section"];
const SECTION_MAP = {
  "dashboard-main-section": "overview",
  "dashboard-admin-section": "admin",
  "dashboard-chat-section": "chat",
  "dashboard-brain-section": "brain",
  "dashboard-ideajar-section": "ideajar",
  "dashboard-ipc-section": "ipc",
  "dashboard-intelligence-section": "Intelligence",
};
function showDashboardSection(sectionId) {
  const prev = DASH_SECTIONS.find(id => { const s = document.getElementById(id); return s && !s.hidden && s.style.display !== 'none'; });
  if (prev === 'dashboard-ideajar-section' && sectionId !== 'dashboard-ideajar-section') {
    if (window.JarCanvas) JarCanvas.destroyJarCanvas('idea-jar-canvas');
  }
  const target = DASH_SECTIONS.includes(sectionId) ? sectionId : "dashboard-main-section";
  DASH_SECTIONS.forEach((id) => {
    const section = document.getElementById(id);
    if (section) { section.hidden = id !== target; section.style.display = id !== target ? 'none' : ''; }
  });
  if (target === 'dashboard-brain-section') {
    const src = document.getElementById('brain-canvas-container');
    const dst = document.getElementById('brain-canvas-fullscreen');
    if (src && dst && !dst.hasChildNodes()) {
      dst.appendChild(src);
      src.style.height = '100%';
      if (typeof window.resizeBrainCanvas === 'function') window.resizeBrainCanvas();
    }
  }
  if (prev === 'dashboard-brain-section' && target !== 'dashboard-brain-section') {
    const src = document.getElementById('brain-canvas-container');
    const widget = document.getElementById('brain-widget');
    if (src && widget) {
      const body = widget.querySelector('.mn-widget__body') || widget;
      body.appendChild(src);
      src.style.height = '480px';
      if (typeof window.resizeBrainCanvas === 'function') window.resizeBrainCanvas();
    }
  }
  if (target === 'dashboard-ideajar-section' && typeof renderIdeaJarTab === 'function') {
    renderIdeaJarTab();
  }
  if (target === 'dashboard-intelligence-section') {
    // Lazy-load ipc-budget.js, ipc-router.js, ipc-skills.js, ipc-models.js
    ['ipc-budget','ipc-router','ipc-skills','ipc-models'].forEach(mod => {
      if (!document.querySelector(`script[src="${mod}.js"]`)) {
        const s = document.createElement('script'); s.src = `${mod}.js`; document.head.appendChild(s);
      }
    });
    setTimeout(() => {
      if (typeof renderIpcBudget === 'function') renderIpcBudget(document.getElementById('ipc-budget-panel'));
      if (typeof renderIpcRouter === 'function') renderIpcRouter(document.getElementById('ipc-router-panel'));
      if (typeof renderIpcSkills === 'function') renderIpcSkills(document.getElementById('ipc-skills-panel'));
      if (typeof renderIpcModels === 'function') renderIpcModels(document.getElementById('ipc-models-panel'));
    }, 200);
  }
  if (target === 'dashboard-chat-section') {
    renderProjectList();
  }
  if (typeof window.setAdminActive === "function") {
    window.setAdminActive(target === "dashboard-admin-section");
  }
  if (target === 'dashboard-ipc-section' && typeof window.startIpcRefresh === 'function') {
    window.startIpcRefresh(10000);
  } else if (prev === 'dashboard-ipc-section' && typeof window.stopIpcRefresh === 'function') {
    window.stopIpcRefresh();
  }
  document.querySelectorAll("#dashboard-nav [data-section]").forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.section === target);
  });
  localStorage.setItem("dashboardSection", target);
  if (window.PollScheduler) window.PollScheduler.setSection(SECTION_MAP[target] || "overview");
}

async function renderProjectList() {
  const el = document.getElementById('project-list-content');
  if (!el) return;
  const data = await fetchJson('/api/projects');
  const projects = Array.isArray(data) ? data : (data?.projects || []);
  if (!projects.length) {
    el.innerHTML = '<div style="color:var(--text-dim);font-size:12px;padding:8px">No projects yet.</div>';
    return;
  }
  el.innerHTML = projects.map(p => `<div class="project-list-item" onclick="selectProject('${esc(p.id || p.name)}')" title="${esc(p.description || '')}">
    <div style="font-weight:600;font-size:12px;color:var(--text)">${esc(p.name)}</div>
    ${p.description ? `<div style="font-size:10px;color:var(--text-dim);margin-top:2px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">${esc(p.description.slice(0, 60))}</div>` : ''}
  </div>`).join('');
}

function selectProject(projectId) {
  document.querySelectorAll('.project-list-item').forEach(el => el.classList.remove('selected'));
  const clicked = event?.target?.closest('.project-list-item');
  if (clicked) clicked.classList.add('selected');
}

async function openNewProjectModal() {
  const existing = document.getElementById('new-project-overlay');
  if (existing) existing.remove();
  const overlay = document.createElement('div');
  overlay.id = 'new-project-overlay';
  overlay.className = 'modal-overlay';
  const _f = (label, content) => `<label class="modal-field"><span class="modal-field-label">${label}</span>${content}</label>`;
  overlay.innerHTML = `<div class="mn-widget" style="width:420px;max-width:95vw;box-shadow:0 0 60px rgba(0,229,255,0.1)">
    <div class="mn-widget__header"><span class="mn-widget__title">New Project</span><span style="cursor:pointer;color:var(--red);font-size:16px" onclick="document.getElementById('new-project-overlay').remove()">✕</span></div>
    <div class="mn-widget__body">
    <form id="new-project-form" style="display:flex;flex-direction:column;gap:10px">
      ${_f('Name *', '<input name="name" required class="modal-input">')}
      ${_f('Description', '<textarea name="description" rows="3" class="modal-input"></textarea>')}
      ${_f('Repository', '<input name="repo" placeholder="owner/repo" class="modal-input">')}
      <div style="display:flex;justify-content:flex-end;gap:6px;padding-top:8px;border-top:1px solid var(--border)">
        <button type="button" class="mn-widget__action" onclick="document.getElementById('new-project-overlay').remove()">Cancel</button>
        <button type="submit" class="mn-widget__action" style="background:rgba(0,229,255,0.15)">Create</button>
      </div>
    </form></div></div>`;
  document.body.appendChild(overlay);
  overlay.addEventListener('click', e => { if (e.target === overlay) overlay.remove(); });
  overlay.querySelector('#new-project-form').addEventListener('submit', async e => {
    e.preventDefault();
    const fd = new FormData(e.target);
    const body = Object.fromEntries(fd.entries());
    try {
      await fetch('/api/projects', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) });
      overlay.remove();
      renderProjectList();
      if (typeof showToast === 'function') showToast('Project created', '', document.body, 'success');
    } catch (err) {
      if (typeof showToast === 'function') showToast('Error', err.message, document.body, 'error');
    }
  });
}

function initDashboardNavigation() {
  const nav = document.getElementById("dashboard-nav");
  if (!nav) return;
  nav.querySelectorAll("[data-section]").forEach((button) => {
    button.addEventListener("click", () => showDashboardSection(button.dataset.section));
  });
  const saved = localStorage.getItem("dashboardSection");
  showDashboardSection(saved || "dashboard-main-section");
}

function initWidgetStates() {
  document
    .querySelectorAll(".mn-widget")
    .forEach((el) => el.classList.add("mn-widget--loading"));

  document.querySelectorAll(".mn-widget").forEach((widget) => {
    const header = widget.querySelector(".mn-widget__header");
    if (!header || header.querySelector(".mn-widget__action--collapse")) return;
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "mn-widget__action mn-widget__action--collapse";
    btn.dataset.widgetToggle = "collapse";
    btn.title = "Collapse widget";
    btn.textContent = "▾";
    header.appendChild(btn);
  });

  document.addEventListener("click", (event) => {
    const action = event.target.closest(".mn-widget__action");
    if (!action || action.dataset.widgetToggle !== "collapse") return;
    const widget = action.closest(".mn-widget");
    if (!widget) return;
    event.preventDefault();
    widget.classList.toggle("mn-widget--collapsed");
    action.textContent = widget.classList.contains("mn-widget--collapsed") ? "▸" : "▾";
    action.title = widget.classList.contains("mn-widget--collapsed")
      ? "Expand widget"
      : "Collapse widget";
  });
}

function initGridTemplate() {
  const grid = document.querySelector(".dash-columns");
  if (!grid) return;
  grid.classList.add("mn-grid-template");
  if (window.Maranello && typeof window.Maranello.gridLayout === "function") {
    try {
      window.Maranello.gridLayout(grid, { template: "sidebar-main" });
    } catch (_) {}
  }
}

function initHeaderIcons() {
  const api = window.Maranello;
  const ALIAS = { bulb: 'lightbulb', sun: 'sparkle', chat: 'mail' };
  document.querySelectorAll("[data-icon]").forEach((el) => {
    const raw = el.getAttribute("data-icon");
    if (!raw) return;
    const name = ALIAS[raw] || raw;
    if (api && typeof api.renderIcon === "function") {
      try {
        api.renderIcon(el, name, { size: 'sm' });
        if (el.innerHTML && el.innerHTML !== '') return;
      } catch (_) {}
    }
    if (api && api.icons && typeof api.icons[name] === "function") {
      el.innerHTML = '<span class="mn-icon mn-icon--sm">' + api.icons[name]() + '</span>';
      return;
    }
    el.textContent = "◦";
  });
}

window.$ = $;
window.fetchJson = fetchJson;
window.refreshAll = refreshAll;
window._resolveHost = _resolveHost;
window.showDashboardSection = showDashboardSection;
window.renderProjectList = renderProjectList;
window.openNewProjectModal = openNewProjectModal;
window.selectProject = selectProject;

window.addEventListener("hashchange", handleHashRoute);

document.addEventListener("DOMContentLoaded", () => {
  if (window.PollScheduler) window.PollScheduler.start();
  if (typeof window.initAdminPanel === "function") window.initAdminPanel();
  if (window.Maranello && typeof window.Maranello.autoBind === "function") {
    try { window.Maranello.autoBind(); } catch (_) {}
  }
  initHeaderIcons();
  initWidgetStates();
  initGridTemplate();
  initDashboardNavigation();
  applyZoom(state.currentZoom);
  updateClock();
  const clockInterval = setInterval(updateClock, 1000);
  window.addEventListener("beforeunload", () => clearInterval(clockInterval));
  // First-load only — NOT in periodic refreshAll
  fetch("/api/mesh/init", { method: "POST" })
    .then((r) => r.json())
    .then((data) => {
      if (data.daemons_restarted && data.daemons_restarted.length > 0) {
        showToast(`Restarted: ${data.daemons_restarted.join(", ")}`, "info");
      }
      if (data.hosts_needing_normalization > 0) {
        showToast(`${data.hosts_needing_normalization} plans need host normalization`, "warn");
      }
    })
    .catch(() => {});
  refreshAll();
  applyRefresh();
  requestAnimationFrame(() => setTimeout(handleHashRoute, 100));
  if (typeof initDashboardWebSocket === "function") initDashboardWebSocket();
});
