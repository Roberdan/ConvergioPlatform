(function () {
  const esc = (value) => String(value ?? "").replace(/[&<>"']/g, (m) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
  })[m]);
  const state = { detailRunId: null, detailEl: null, modal: null };

  function parseJson(value) {
    if (!value) return null;
    if (typeof value === "object") return value;
    try { return JSON.parse(value); } catch { return null; }
  }

  function parseReportJson(reportStr) {
    const report = parseJson(reportStr) || {};
    return {
      sentry_unresolved: Number(report.sentry_unresolved ?? report.sentry?.unresolved ?? 0),
      github_open: Number(report.github_open ?? report.github_open_issues ?? report.github?.open ?? 0),
      top_sentry: Array.isArray(report.top_sentry_issues) ? report.top_sentry_issues : Array.isArray(report.top_sentry) ? report.top_sentry : [],
      top_github: Array.isArray(report.top_github_issues) ? report.top_github_issues : Array.isArray(report.top_github) ? report.top_github : [],
      deploy_status: String(report.deploy?.status || report.deploy_status || "unknown"),
    };
  }

  function formatValue(value) {
    return typeof value === "object" && value !== null ? `<pre style="margin:4px 0 0;white-space:pre-wrap">${esc(JSON.stringify(value, null, 2))}</pre>` : `<span class="nightly-meta">${esc(value)}</span>`;
  }

  function formatConfigSnapshot(configStr) {
    const config = parseJson(configStr);
    if (!config || typeof config !== "object") return '<div class="nightly-empty">No config snapshot captured.</div>';
    const entries = Object.entries(config);
    if (!entries.length) return '<div class="nightly-empty">No config snapshot captured.</div>';
    return entries.map(([key, value]) => `<div class="nightly-def-row" style="align-items:flex-start"><span class="nightly-def-name">${esc(key)}</span><div style="min-width:0">${formatValue(value)}</div></div>`).join("");
  }

  async function fetchJob(runId) {
    const response = await fetch(`/api/nightly/jobs/${encodeURIComponent(runId)}`);
    const data = await response.json().catch(() => null);
    if (!response.ok || !data) throw new Error(data?.error || "Failed to load");
    return data;
  }

  function issueLabel(item, type) {
    if (!item) return "";
    if (typeof item === "string") return item;
    if (type === "github") return `#${item.number ?? item.id ?? "?"} ${item.title || item.short || ""}`.trim();
    return [item.short || item.id, item.title].filter(Boolean).join(" - ");
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
            ? ["", "—", "mn-badge--neutral"]
            : ["nightly-failed mn-status-blocked", String(status || "FAILED"), "mn-badge--neutral"];
    return `<span class="nightly-badge mn-badge ${kind[2]} ${kind[0]}">${esc(kind[1])}</span>`;
  }

  function renderIssueList(items, type, emptyText) {
    if (!items.length) return `<div class="nightly-empty">${esc(emptyText)}</div>`;
    return `<ul class="nightly-issues">${items.slice(0, 5).map((item) => `<li>${esc(issueLabel(item, type))}</li>`).join("")}</ul>`;
  }

  function closeDetail() {
    if (state.detailEl) state.detailEl.remove();
    state.detailEl = null;
    state.detailRunId = null;
  }

  function closeModal() {
    if (!state.modal) return;
    document.removeEventListener("keydown", state.modal.onKeyDown);
    state.modal.el.remove();
    state.modal = null;
  }

  function setLogTab(tab) {
    if (!state.modal) return;
    state.modal.tab = tab;
    const text = String(state.modal.detail?.[tab === "stderr" ? "log_stderr" : "log_stdout"] || "").trim() || "No logs captured for this run";
    state.modal.logEl.textContent = state.modal.errorText || text;
    state.modal.el.querySelectorAll("[data-tab]").forEach((button) => {
      button.style.background = button.dataset.tab === tab ? "color-mix(in srgb, var(--cyan) 20%, transparent)" : "";
      button.style.borderColor = button.dataset.tab === tab ? "color-mix(in srgb, var(--cyan) 55%, transparent)" : "";
    });
  }

  async function retryRun(runId, button) {
    if (!button) return;
    const previous = button.textContent;
    button.disabled = true;
    button.textContent = "Retrying...";
    try {
      const response = await fetch(`/api/nightly/jobs/${encodeURIComponent(runId)}/retry`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: "{}",
      });
      const data = await response.json().catch(() => null);
      if (!response.ok || !data?.ok) throw new Error(data?.error || "Failed to load");
      button.textContent = "Retry queued";
      if (typeof refreshAll === "function") refreshAll();
    } catch (error) {
      button.textContent = error.message || "Failed to load";
      setTimeout(() => { button.textContent = previous; button.disabled = false; }, 1200);
      return;
    }
    setTimeout(() => { button.textContent = previous; button.disabled = false; }, 1200);
  }

  async function showLogs(runId) {
    closeModal();
    let detail = null;
    let errorText = "";
    try { detail = await fetchJob(runId); } catch (error) { errorText = error.message || "Failed to load"; }
    const pid = detail ? projectIdFromJob(detail) : "";
    const pIcon = _njIcon(PROJECT_ICONS[pid] || '', 14);
    const pLabel = PROJECT_LABELS[pid] || "";
    const projectTag = pIcon ? `<span style="display:inline-flex;align-items:center;gap:4px">${pIcon} ${esc(pLabel)}</span> · ` : "";
    const overlay = document.createElement("div");
    overlay.className = "nightly-overlay mn-modal";
    overlay.style.cssText = "position:fixed;inset:0;background:rgba(0,0,0,.72);z-index:9999;display:flex;padding:24px";
    overlay.innerHTML = `<div class="nightly-history mn-signal-panel" style="margin:auto;width:min(1100px,100%);height:100%;background:var(--bg);border:1px solid color-mix(in srgb, var(--blue) 35%, transparent);border-radius:10px;padding:14px;display:flex;flex-direction:column;gap:10px">
      <div class="nightly-head" style="justify-content:space-between">
        <div class="nightly-head"><div class="nightly-history-title" style="margin:0">${projectTag}Run logs · ${esc(detail?.run_id || runId)}</div></div>
        <div class="nightly-head"><button class="nightly-btn mn-btn mn-btn--sm" data-copy="1">Copy</button><button class="nightly-btn mn-btn mn-btn--sm" data-close="1">X</button></div>
      </div>
      <div class="nightly-head" style="gap:6px"><button class="nightly-btn mn-btn mn-btn--sm" data-tab="stdout">stdout</button><button class="nightly-btn mn-btn mn-btn--sm" data-tab="stderr">stderr</button></div>
      <pre class="nightly-summary" data-log style="margin:0;flex:1;overflow:auto;white-space:pre-wrap;background:color-mix(in srgb, var(--blue) 6%, transparent);border-radius:8px;padding:12px;font-family:ui-monospace,SFMono-Regular,Menlo,monospace"></pre>
    </div>`;
    document.body.appendChild(overlay);
    const onKeyDown = (event) => { if (event.key === "Escape") closeModal(); };
    document.addEventListener("keydown", onKeyDown);
    state.modal = { el: overlay, onKeyDown, detail, logEl: overlay.querySelector("[data-log]"), errorText, tab: "stdout" };
    overlay.addEventListener("click", (event) => { if (event.target === overlay || event.target.dataset.close) closeModal(); });
    overlay.querySelectorAll("[data-tab]").forEach((button) => button.addEventListener("click", () => setLogTab(button.dataset.tab)));
    overlay.querySelector("[data-copy]").addEventListener("click", async () => {
      const copyButton = overlay.querySelector("[data-copy]");
      try {
        if (!navigator.clipboard?.writeText) throw new Error("Copy unavailable");
        await navigator.clipboard.writeText(state.modal.logEl.textContent || "");
        copyButton.textContent = "Copied";
      } catch (error) {
        copyButton.textContent = error.message || "Copy failed";
      }
      setTimeout(() => { if (state.modal) copyButton.textContent = "Copy"; }, 1000);
    });
    setLogTab(detail?.log_stdout ? "stdout" : "stderr");
  }

  const _njIcon = (name, size) => (window.Icons && window.Icons[name]) ? window.Icons[name](size || 14) : '';
  const PROJECT_ICONS = { mirrorbuddy: 'brain', virtualbpm: 'zap' };
  const PROJECT_LABELS = { mirrorbuddy: "MirrorBuddy", virtualbpm: "VirtualBPM" };
  function projectIdFromJob(row) {
    const name = String(row?.job_name || "");
    if (name.startsWith("virtualbpm")) return "virtualbpm";
    if (name.startsWith("mirrorbuddy")) return "mirrorbuddy";
    return name.split("-")[0] || "mirrorbuddy";
  }

  function buildDetailHtml(detail) {
    const report = parseReportJson(detail.report_json);
    const pid = projectIdFromJob(detail);
    const pIcon = _njIcon(PROJECT_ICONS[pid] || 'project', 18);
    const pLabel = PROJECT_LABELS[pid] || pid;
    const sentryCount = Number(detail.sentry_unresolved ?? report.sentry_unresolved ?? 0);
    const githubCount = Number(detail.github_open_issues ?? report.github_open ?? 0);
    const runStatus = statusBadge(detail.status || "none");
    const deployStatus = statusBadge(report.deploy_status || "unknown");
    const summary = detail.summary ? `<div class="nightly-summary">${esc(detail.summary)}</div>` : '<div class="nightly-empty">No summary recorded for this run.</div>';
    const errorBox = detail.error_detail ? `<div class="nightly-summary nightly-failed mn-status-blocked" style="padding:8px;border-radius:6px;font-family:ui-monospace,SFMono-Regular,Menlo,monospace">${esc(detail.error_detail)}</div>` : "";
    const prLink = detail.pr_url ? `<a href="${esc(detail.pr_url)}" target="_blank" rel="noreferrer">${esc(detail.pr_url)}</a>` : '<span class="nightly-empty">No PR</span>';
    const branch = detail.branch_name ? `<span class="nightly-meta">${esc(detail.branch_name)}</span>` : '<span class="nightly-empty">No branch</span>';
    const parent = detail.parent_run_id ? `<div class="nightly-meta">Retry of: ${esc(detail.parent_run_id)}</div>` : "";
    return `<div class="nightly-latest" style="gap:10px">
      <div style="display:flex;align-items:center;gap:8px;margin-bottom:4px">${pIcon}<span style="font-weight:600;font-size:14px;color:var(--text)">${esc(pLabel)}</span>${runStatus}</div>
      ${summary}
      ${errorBox}
      <div class="nightly-metrics mn-stat-row"><span><b>${sentryCount}</b> Sentry unresolved</span><span><b>${githubCount}</b> GitHub open</span><span>${deployStatus} deploy</span></div>
      <div class="nightly-history mn-signal-panel">
        <div class="nightly-history-title">Report details</div>
        <div class="nightly-def-row" style="align-items:flex-start"><span class="nightly-def-name">Sentry</span><div>${renderIssueList(report.top_sentry, "sentry", "No Sentry issues listed.")}</div></div>
        <div class="nightly-def-row" style="align-items:flex-start"><span class="nightly-def-name">GitHub</span><div>${renderIssueList(report.top_github, "github", "No GitHub issues listed.")}</div></div>
        <div class="nightly-def-row"><span class="nightly-def-name">Deploy</span><span>${deployStatus}</span></div>
      </div>
      <div class="nightly-history mn-signal-panel">
        <div class="nightly-history-title">Links</div>
        <div class="nightly-def-row"><span class="nightly-def-name">PR</span><div>${prLink}</div></div>
        <div class="nightly-def-row"><span class="nightly-def-name">Branch</span><div>${branch}</div></div>
        ${parent}
      </div>
      <details class="nightly-history mn-signal-panel"><summary class="nightly-history-title" style="cursor:pointer">Config snapshot</summary><div style="margin-top:8px">${formatConfigSnapshot(detail.config_snapshot)}</div></details>
      <div class="nightly-head" style="justify-content:flex-end"><button class="nightly-btn mn-btn mn-btn--sm" data-action="logs">View Logs</button><button class="nightly-btn mn-btn mn-btn--sm" data-action="retry">Retry</button></div>
    </div>`;
  }

  async function showDetail(runId, containerElement) {
    const anchor = containerElement?.closest?.(".nightly-history-row") || containerElement;
    if (!anchor?.parentNode) return;
    if (state.detailRunId === runId) { closeDetail(); return; }
    closeDetail();
    const panel = document.createElement("div");
    panel.className = "nightly-history nightly-detail mn-signal-panel mn-anim-fadeIn";
    panel.style.cssText = "margin:6px 0 10px 14px;padding:10px;border:1px solid color-mix(in srgb, var(--blue) 28%, transparent);border-radius:8px;background:color-mix(in srgb, var(--blue) 5%, transparent)";
    panel.innerHTML = '<div class="nightly-empty">Loading...</div>';
    anchor.parentNode.insertBefore(panel, anchor.nextSibling);
    state.detailEl = panel;
    state.detailRunId = runId;
    panel.addEventListener("click", (event) => {
      const button = event.target.closest("[data-action]");
      if (!button) return;
      event.stopPropagation();
      if (button.dataset.action === "logs") showLogs(runId);
      if (button.dataset.action === "retry") retryRun(runId, button);
    });
    try {
      const detail = await fetchJob(runId);
      if (state.detailEl !== panel) return;
      panel.innerHTML = buildDetailHtml(detail);
    } catch {
      if (state.detailEl === panel) panel.innerHTML = '<div class="nightly-empty">Failed to load</div>';
    }
  }

  window._njShowLogs = showLogs;
  window._njShowDetail = showDetail;
})();
