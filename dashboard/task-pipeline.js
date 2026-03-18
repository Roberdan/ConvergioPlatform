// Uses Maranello DS badges for status rendering.
function statusToBadge(status) {
  const map = {
    done: "success",
    in_progress: "info",
    submitted: "warning",
    blocked: "danger",
    pending: "neutral",
    cancelled: "neutral",
  };
  return map[status] || "neutral";
}
function statusBadge(status) {
  const normalized = status === "doing" ? "in_progress" : status || "pending";
  return `<span class="mn-badge mn-badge--${statusToBadge(normalized)}">${esc(String(normalized))}</span>`;
}
function _taskRow(t) {
  const title = t.title || "\u2014",
    truncated = title.length > 40,
    tokenText =
      t.tokens === null || t.tokens === undefined ? "\u2014" : fmt(t.tokens),
    statusCls =
      t.status === "done"
        ? "task-done mn-row-done"
        : t.status === "in_progress"
          ? "task-running mn-row-in-progress"
        : t.status === "submitted"
          ? "task-submitted mn-row-pending"
        : t.status === "blocked"
              ? "task-blocked mn-row-blocked mn-highlight-pulse"
              : "task-pending mn-row-pending",
    substatusBadges = {
      waiting_ci: { color: "#00d4ff", text: Icons.clock(11) + " CI" },
      waiting_review: { color: "#e6a117", text: Icons.eye(11) + " Review" },
      waiting_merge: { color: "#a855f7", text: Icons.gitMerge(11) + " Merge" },
      waiting_thor: { color: "#ff9500", text: Icons.shield(11) + " Thor" },
      agent_running: { color: "#0066ff", text: Icons.cpu(11) + " Agent" },
    },
    substatus = substatusBadges[t.substatus];
  return `<tr class="${statusCls} mn-hover-lift" onclick="toggleTaskDetail(this)" data-task-id="${esc(t.task_id || "")}"><td style="color:var(--cyan);font-weight:600">${esc(t.task_id || "")}</td><td style="overflow:hidden;text-overflow:ellipsis;white-space:nowrap" ${truncated ? `title="${esc(title)}"` : ""}>${esc(title.substring(0, 40))}${truncated ? "\u2026" : ""}</td><td>${statusBadge(t.status)} ${substatus ? `<span class="mn-badge mn-badge--info" title="${esc(t.substatus)}">${substatus.text}</span>` : ""} ${thorIcon(t.validated_at)}</td><td style="color:var(--text-dim);overflow:hidden;text-overflow:ellipsis;white-space:nowrap">${esc((t.executor_agent || "\u2014").substring(0, 12))}</td><td style="color:var(--gold)">${tokenText}</td></tr>`;
}

window.filterTasks = (planId) => {
  window.DashboardState.filteredPlanId = planId;
  renderTaskPipeline();
};
function renderTaskPipeline() {
  try {
  var table = document.querySelector("#task-table");
  if (table) { table.style.display = ''; }
  var mnTable = document.querySelector('#mn-task-table');
  if (mnTable) mnTable.remove();
  const st = window.DashboardState,
    tbody = document.querySelector("#task-table tbody");
  if (!tbody) {
    console.warn('[TaskPipeline] tbody not found');
    var w = document.getElementById('task-pipeline-widget');
    if (w) { var b = w.querySelector('.mn-widget__body'); if (b) b.innerHTML = '<div style="color:red;padding:12px">ERROR: task-table tbody not found</div>'; }
    return;
  }
  if (!st || !st.allMissionPlans || !st.allMissionPlans.length) {
    console.warn('[TaskPipeline] no plans:', st && st.allMissionPlans);
    tbody.innerHTML = '<tr><td colspan="5" style="color:var(--text-dim);padding:12px;text-align:center">No tasks loaded</td></tr>';
    return;
  }
  const plans = st.filteredPlanId
      ? st.allMissionPlans.filter(
          (m) => m.plan && m.plan.id === st.filteredPlanId,
        )
      : st.allMissionPlans,
    label = document.querySelector("#task-filter-label"),
    btn = document.querySelector("#task-filter-clear");
  if (label)
    label.textContent = st.filteredPlanId
      ? `#${st.filteredPlanId}`
      : `${st.allMissionPlans.length} plans`;
  if (btn) btn.style.display = st.filteredPlanId ? "" : "none";
  let rows = "";
  plans.forEach((m) => {
    const p = m.plan,
      waves = m.waves || [],
      tasks = m.tasks || [];
    if (!st.filteredPlanId && st.allMissionPlans.length > 1)
      rows += `<tr class="task-group-header mn-anim-fadeIn mn-hover-lift" onclick="filterTasks(${p.id})"><td colspan="5"><span style="color:var(--cyan);font-weight:600">#${p.id}</span> ${esc((p.name || "").substring(0, 30))}</td></tr>`;
    if (waves.length) {
      waves.forEach((w) => {
        const waveTasks = tasks.filter((t) => String(t.wave_id) === String(w.wave_id));
        if (!waveTasks.length) return;
        const wp =
          w.tasks_total > 0
            ? Math.round((100 * w.tasks_done) / w.tasks_total)
            : 0,
          wPct = wp >= 100 && !w.validated_at ? 95 : wp,
          wName = (w.name || "").substring(0, 25);
        rows += `<tr class="task-wave-header mn-hover-lift"><td colspan="5">${statusBadge(w.status)} <span style="color:var(--text)">${esc(String(w.wave_id))}</span> <span style="color:var(--text-dim);font-size:10px">${esc(wName)}</span> <span style="color:${wPct >= 75 ? 'var(--green)' : wPct >= 50 ? 'var(--gold)' : 'var(--red)'};font-size:10px">${wPct}%</span> ${thorIcon(w.validated_at)}</td></tr>`;
        waveTasks.forEach((t) => (rows += _taskRow(t)));
      });
      tasks
        .filter((t) => !waves.some((w) => String(w.wave_id) === String(t.wave_id)))
        .forEach((t) => (rows += _taskRow(t)));
    } else tasks.forEach((t) => (rows += _taskRow(t)));
  });
  tbody.innerHTML = rows || '<tr><td colspan="5" style="color:var(--text-dim);padding:12px;text-align:center">No tasks to display</td></tr>';
  } catch(e) {
    console.error('[TaskPipeline] ERROR:', e);
    var d = document.getElementById('dbg-errors');
    if (d) { d.style.display = 'block'; d.innerHTML += '<br>[TaskPipeline] ' + e.message; }
  }
}
window.toggleTaskDetail = function (tr) {
  const n = tr.nextElementSibling;
  if (n && n.classList.contains("task-detail-row")) {
    n.remove();
    tr.classList.remove("expanded");
    return;
  }
  document.querySelectorAll(".task-detail-row").forEach((r) => r.remove());
  document
    .querySelectorAll(".expanded")
    .forEach((r) => r.classList.remove("expanded"));
  let t = null;
  for (const m of window.DashboardState.allMissionPlans) {
    t = (m.tasks || []).find((x) => x.task_id === tr.dataset.taskId);
    if (t) break;
  }
  if (!t) return;
  tr.classList.add("expanded");
  const row = document.createElement("tr");
  row.className = "task-detail-row";
  const substatusBadges = { waiting_ci: Icons.clock(11) + " CI", waiting_review: Icons.eye(11) + " Review", waiting_merge: Icons.gitMerge(11) + " Merge", waiting_thor: Icons.shield(11) + " Thor", agent_running: Icons.cpu(11) + " Agent" };
  row.innerHTML = `<td colspan="5"><div class="task-detail"><strong style="color:var(--cyan)">${esc(t.task_id)}</strong> \u2014 ${esc(t.title || "")}<br>Status: <span style="color:${statusColor(t.status)}">${t.status}</span>${t.substatus ? ` · Substatus: <span class="mn-badge mn-badge--info">${esc(substatusBadges[t.substatus] || t.substatus)}</span>` : ""} \u00b7 Agent: ${esc(t.executor_agent || "\u2014")} \u00b7 Host: ${esc(t.executor_host || "\u2014")} \u00b7 Tokens: ${fmt(t.tokens)} ${t.validated_at ? ` \u00b7 ${thorIcon(true)} Validated` : ""}</div></td>`;
  tr.after(row);
};

window.renderTaskPipeline = renderTaskPipeline;
