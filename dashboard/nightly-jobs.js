(function () {
  window._njRunNowInFlight = window._njRunNowInFlight || {};
  const state = { latest: null, history: [], definitions: [], page: 1, perPage: 50, total: 0, unavailable: false };
  const byId = (id) => document.getElementById(id);
  const esc = (v) => String(v ?? "").replace(/[&<>"']/g, (m) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[m]);
  const parseTs = (v) => !v ? 0 : typeof v === "number" ? (v > 1e12 ? v : v * 1000) : (Date.parse(String(v)) || 0);
  const num = (v) => Number(v || 0);
  const _njIcon = (name, size) => (window.Icons && window.Icons[name]) ? window.Icons[name](size || 14) : '';
  const PROJECT_ICONS = { mirrorbuddy: 'brain', virtualbpm: 'zap' };
  const PROJECT_LABELS = { mirrorbuddy: "MirrorBuddy", virtualbpm: "VirtualBPM" };

  function formatDuration(sec, fallbackRow) {
    let s = Number.isFinite(Number(sec)) ? Math.max(0, Math.round(Number(sec))) : NaN;
    if (!Number.isFinite(s) && fallbackRow) s = Math.max(0, Math.round((parseTs(fallbackRow.finished_at) - parseTs(fallbackRow.started_at)) / 1000));
    if (!Number.isFinite(s)) return "\u2014";
    const h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60), r = s % 60;
    return h ? h+"h "+m+"m" : m ? m+"m "+r+"s" : r+"s";
  }
  function formatTimestamp(val) {
    const ts = parseTs(val);
    return ts ? new Date(ts).toLocaleString("en-GB", { timeZone: "Europe/Rome", day: "2-digit", month: "2-digit", year: "numeric", hour: "2-digit", minute: "2-digit", second: "2-digit", hour12: false, timeZoneName: "short" }) : "n/a";
  }
  function timeAgo(val) {
    const ts = parseTs(val);
    if (!ts) return "n/a";
    const sec = Math.max(0, Math.floor((Date.now() - ts) / 1000));
    return sec < 60 ? sec+"s ago" : sec < 3600 ? Math.floor(sec / 60)+"m ago" : sec < 86400 ? Math.floor(sec / 3600)+"h ago" : Math.floor(sec / 86400)+"d ago";
  }
  function statusBadge(status) {
    const s = String(status || "none").toLowerCase();
    const kind = s === "ok" || s === "success" || s === "completed"
      ? ["nightly-ok mn-status-done", "OK", "mn-badge--neutral"]
      : s === "running"
        ? ["nightly-running mn-status-in-progress", "RUNNING", "mn-badge--info"]
        : s === "action_required" || s === "action"
          ? ["nightly-action", "ACTION", "mn-badge--info"]
          : s === "none"
            ? ["", "\u2014", "mn-badge--neutral"]
            : ["nightly-failed mn-status-blocked", "FAILED", "mn-badge--neutral"];
    return '<span class="nightly-badge mn-badge '+kind[2]+' '+kind[0]+'">'+kind[1]+'</span>';
  }
  function cronToHuman(cron) {
    if (!cron) return "\u2014";
    const p = cron.split(/\s+/);
    if (p.length < 5) return cron;
    return p[1].padStart(2, "0")+":"+p[0].padStart(2, "0")+" CET";
  }
  function projectIdFromJob(row) {
    const name = String(row?.job_name || "");
    if (name.startsWith("virtualbpm")) return "virtualbpm";
    if (name.startsWith("mirrorbuddy")) return "mirrorbuddy";
    return name.split("-")[0] || "mirrorbuddy";
  }
  function latestForProject(pid) {
    return state.history.find((r) => projectIdFromJob(r) === pid) || null;
  }
  const toast = (title, msg, type) => window.showToast ? window.showToast(title, msg || "", null, type) : alert(title+(msg ? ": "+msg : ""));
  async function apiCall(url, method, body) {
    const res = await fetch(url, { method: method || "GET", headers: body ? { "Content-Type": "application/json" } : undefined, body: body ? JSON.stringify(body) : undefined });
    const data = await res.json().catch(() => ({}));
    if (!res.ok || data.ok === false) throw new Error(data.error || method+" "+url+" failed");
    return data;
  }
  async function refreshWidget() {
    const data = await apiCall("/api/nightly/jobs?page=1&per_page="+(state.perPage || 50));
    window.renderNightlyJobs(data);
  }
  async function loadMore() {
    const data = await apiCall("/api/nightly/jobs?page="+(state.page + 1)+"&per_page="+(state.perPage || 50));
    state.page = num(data.page) || state.page + 1;
    state.total = num(data.total) || state.total;
    state.perPage = num(data.per_page) || state.perPage;
    state.history = state.history.concat((data.history || []).filter((row) => !state.history.some((seen) => String(seen.id) === String(row.id))));
    draw();
  }
  function parseReport(raw) {
    if (!raw) return {};
    if (typeof raw === "object") return raw;
    try { return JSON.parse(raw); } catch { return {}; }
  }
  function collect(report, key, fallback) {
    const bucket = report[key] || report[key+"_issues"] || report[key+"_summary"] || {};
    const items = [bucket.issues, bucket.items, report[key+"_top_issues"], report[key+"_titles"], report[key+"_issue_titles"]].find(Array.isArray) || (Array.isArray(bucket) ? bucket : []);
    return {
      count: Number(report[key+"_count"] ?? bucket.count ?? fallback ?? items.length ?? 0),
      titles: items.map((item) => typeof item === "string" ? item : item?.title || item?.issue_title || item?.name).filter(Boolean).slice(0, 3),
    };
  }

  function projectCard(def) {
    const pid = def.project_id || "mirrorbuddy";
    const iconName = PROJECT_ICONS[pid] || 'project';
    const icon = _njIcon(iconName, 18);
    const label = PROJECT_LABELS[pid] || pid;
    const latest = latestForProject(pid);
    const report = parseReport(latest?.report_json);
    const sentry = collect(report, "sentry", latest?.sentry_unresolved);
    const isActive = !!def.enabled;
    const isPaused = !isActive;

    let statusIcon = _njIcon('clock', 14), statusText = "No runs yet", borderColor = "var(--text-dim)";
    if (isPaused) {
      statusIcon = _njIcon('pause', 14); statusText = "Paused"; borderColor = "var(--gold)";
    } else if (latest) {
      const s = String(latest.status || "").toLowerCase();
      if (s === "ok" || s === "success" || s === "completed") {
        statusIcon = _njIcon('checkCircle', 14); statusText = "OK"; borderColor = "var(--green)";
      } else if (s === "running") {
        statusIcon = _njIcon('sync', 14); statusText = "Running"; borderColor = "var(--cyan)";
      } else if (s === "failed") {
        statusIcon = _njIcon('xCircle', 14); statusText = "Failed"; borderColor = "var(--red)";
      } else if (s === "action_required" || s === "action") {
        statusIcon = _njIcon('alertTriangle', 14); statusText = "Action"; borderColor = "var(--gold)";
      }
      if (sentry.count > 0 && statusText === "OK") {
        statusIcon = _njIcon('alertCircle', 14); statusText = sentry.count+" errors"; borderColor = "var(--red)";
      }
    }

    const runDisabled = isPaused ? 'disabled style="opacity:0.4;cursor:not-allowed"' : '';

    return '<div class="nj-card mn-night-agent mn-hover-lift mn-anim-fadeIn" style="border-left:4px solid '+borderColor+';background:var(--bg);border-radius:8px;padding:14px;display:flex;flex-direction:column;gap:10px;'+(isPaused ? 'opacity:0.7;' : '')+'">'
      +'<div style="display:flex;align-items:center;justify-content:space-between">'
      +'<div style="display:flex;align-items:center;gap:8px">'
      +icon
      +'<span style="font-weight:600;font-size:15px;color:var(--text)">'+esc(label)+'</span>'
      +'</div>'
      +'<div style="display:flex;align-items:center;gap:6px">'
      +statusIcon
      +'<span style="font-size:12px;font-weight:500;color:var(--text)">'+statusText+'</span>'
      +'</div></div>'
      +'<div style="display:grid;grid-template-columns:1fr 1fr;gap:4px;font-size:11px;color:var(--text-dim)">'
      +'<span>'+_njIcon('calendar', 12)+' '+cronToHuman(def.schedule)+'</span>'
      +'<span>'+_njIcon('monitor', 12)+' '+esc(def.target_host || "local")+'</span>'
      +'<span>'+_njIcon('clock', 12)+' '+(latest ? timeAgo(latest.finished_at || latest.started_at) : "never")+'</span>'
      +'<span>'+_njIcon('timer', 12)+' '+(latest ? formatDuration(latest.duration_sec, latest) : "\u2014")+'</span>'
      +'</div>'
      +(latest && sentry.count > 0 ? '<div style="font-size:11px;padding:4px 8px;background:color-mix(in srgb,var(--red) 10%,transparent);border-radius:4px;color:var(--red)">'+_njIcon('alertTriangle', 12)+' '+sentry.count+' Sentry error'+(sentry.count > 1 ? 's' : '')+'</div>' : '')
      +(latest && latest.pr_url ? '<div style="font-size:11px"><a href="'+esc(latest.pr_url)+'" target="_blank" style="color:var(--cyan)">'+_njIcon('gitPull', 12)+' Open PR</a></div>' : '')
      +'<div style="display:flex;gap:6px;margin-top:auto;flex-wrap:wrap">'
      +'<button class="nightly-btn mn-btn mn-btn--sm" data-action="run-project" data-project="'+esc(pid)+'" '+runDisabled+'>'+_njIcon('start', 12)+' Run Now</button>'
      +'<button class="nightly-btn mn-btn mn-btn--sm '+(isPaused ? 'nightly-btn-save' : 'nightly-btn-cancel')+'" data-action="toggle-def" data-id="'+esc(def.id)+'">'+(isPaused ? _njIcon('start', 12)+' Resume' : _njIcon('pause', 12)+' Pause')+'</button>'
      +'<button class="nightly-btn mn-btn mn-btn--sm '+(def.run_fixes ? 'nightly-ok mn-status-done' : 'nightly-failed mn-status-blocked')+'" data-action="toggle-fixes-project" data-project="'+esc(pid)+'" style="font-size:10px;cursor:pointer">'+_njIcon('fixOn', 10)+' Fix: '+(def.run_fixes ? 'ON' : 'OFF')+'</button>'
      +'</div></div>';
  }

  function projectCardsSection() {
    if (!state.definitions.length) return '<div class="nightly-empty">No nightly agents configured.</div>';
    return '<div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(280px,1fr));gap:12px;margin-bottom:16px">'+state.definitions.map(projectCard).join("")+'</div>';
  }

  function historySection() {
    if (!state.history.length) return '<div class="nightly-history mn-signal-panel"><div class="nightly-empty">No run history yet.</div></div>';
    const rows = state.history.map(function(row) {
      const pid = projectIdFromJob(row);
      const icon = _njIcon(PROJECT_ICONS[pid] || 'project', 12);
      const label = PROJECT_LABELS[pid] || pid;
      const isFailed = String(row.status).toLowerCase() === "failed";
      return '<tr data-row-id="'+esc(row.id)+'" style="cursor:pointer;border-top:1px solid color-mix(in srgb,var(--blue) 20%,transparent);background:'+(isFailed ? 'color-mix(in srgb,var(--red) 8%,transparent)' : 'transparent')+'">'
        +'<td>'+statusBadge(row.status)+'</td><td>'+icon+' '+esc(label)+'</td><td>'+esc(row.host || "-")+'</td>'
        +'<td title="'+formatTimestamp(row.started_at)+'">'+timeAgo(row.started_at)+'</td>'
        +'<td>'+formatDuration(row.duration_sec, row)+'</td>'
        +'<td title="processed / fixed">'+num(row.processed_items)+' / '+num(row.fixed_items)+'</td>'
        +'<td>'+esc(row.trigger_source || "-")+'</td>'
        +'<td>'+(row.pr_url ? '<a href="'+esc(row.pr_url)+'" target="_blank" rel="noreferrer" style="color:var(--cyan)" data-action="open-pr" data-url="'+esc(row.pr_url)+'">PR</a>' : '-')+'</td>'
        +'<td><button class="nightly-btn mn-btn mn-btn--sm" data-action="retry" data-id="'+esc(row.id)+'">'+_njIcon('refresh', 12)+'</button></td></tr>';
    }).join("");
    return '<div class="nightly-history mn-signal-panel">'
      +'<div class="nightly-history-title">Run History</div>'
      +'<div style="overflow:auto">'
      +'<table style="width:100%;border-collapse:collapse;font-size:10px">'
      +'<thead><tr style="text-align:left;color:var(--text-dim)"><th>Status</th><th>Project</th><th>Host</th><th>Started</th><th>Duration</th><th>Items</th><th>Trigger</th><th>PR</th><th></th></tr></thead>'
      +'<tbody>'+rows+'</tbody></table></div>'
      +(state.total > state.history.length ? '<div style="margin-top:8px"><button class="nightly-btn mn-btn mn-btn--sm" data-action="load-more">Load more</button></div>' : '')
      +'</div>';
  }

  function draw() {
    const root = byId("nightly-jobs-content");
    if (!root) return;
    root.innerHTML = state.unavailable ? '<div class="nightly-empty">Nightly agents unavailable.</div>' :
      '<div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:10px">'
      +'<div style="font-size:13px;font-weight:600;color:var(--text)">'+_njIcon('nightAgent', 16)+' Nightly Agents</div>'
      +'<button class="nightly-btn mn-btn mn-btn--sm" data-action="refresh" style="font-size:10px">'+_njIcon('refresh', 12)+' Refresh</button></div>'
      +projectCardsSection()+historySection();
    if (!root.dataset.bound) {
      root.dataset.bound = "1";
      root.addEventListener("click", async (event) => {
        const action = event.target.closest("[data-action]"), row = action ? null : event.target.closest("[data-row-id]");
        if (row && window._njShowDetail) return void window._njShowDetail(row.dataset.rowId, row);
        if (!action) return;
        event.preventDefault();
        const id = action.dataset.id, pid = action.dataset.project;
        try {
          if (action.dataset.action === "refresh") await refreshWidget();
          if (action.dataset.action === "load-more") await loadMore();
          if (action.dataset.action === "run-project") {
            if (!pid || action.disabled) return;
            if (window._njRunNowInFlight[pid]) return;
            if (!confirm("Run "+(PROJECT_LABELS[pid] || pid)+" nightly now?")) return;
            window._njRunNowInFlight[pid] = true;
            try {
              await apiCall("/api/nightly/jobs/trigger", "POST", { project_id: pid });
              toast("Triggered", (PROJECT_LABELS[pid] || pid)+" nightly started", "success");
              await refreshWidget();
            } finally {
              setTimeout(function() { window._njRunNowInFlight[pid] = false; }, 500);
            }
          }
          if (action.dataset.action === "toggle-fixes-project") {
            const def = state.definitions.find(function(d) { return d.project_id === pid; });
            await apiCall("/api/nightly/config/"+encodeURIComponent(pid), "PUT", { run_fixes: def?.run_fixes ? 0 : 1 });
            toast("Auto-fix", (PROJECT_LABELS[pid] || pid)+": "+(def?.run_fixes ? "OFF" : "ON"), "success");
            await refreshWidget();
          }
          if (action.dataset.action === "retry") { await apiCall("/api/nightly/jobs/"+encodeURIComponent(id)+"/retry", "POST"); toast("Retry queued", "Run "+id, "success"); await refreshWidget(); }
          if (action.dataset.action === "open-pr") window.open(action.dataset.url, "_blank", "noopener");
          if (action.dataset.action === "toggle-def") { await apiCall("/api/nightly/jobs/definitions/"+encodeURIComponent(id)+"/toggle", "POST"); await refreshWidget(); }
          if (action.dataset.action === "parent" && window._njShowDetail) window._njShowDetail(id, Array.from(root.querySelectorAll("[data-row-id]")).find(function(el) { return el.dataset.rowId === id; }) || null);
        } catch (err) {
          toast("Error", err.message, "error");
        }
      });
    }
  }

  window.renderNightlyJobs = function renderNightlyJobs(payload) {
    state.unavailable = !payload || payload.ok === false;
    state.latest = payload?.latest || null;
    state.history = Array.isArray(payload?.history) ? payload.history : [];
    state.definitions = Array.isArray(payload?.definitions) ? payload.definitions : [];
    state.page = num(payload?.page) || 1;
    state.perPage = num(payload?.per_page) || 50;
    state.total = num(payload?.total) || state.history.length;
    draw();
  };
})();
