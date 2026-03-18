// Plan Kanban — drag-and-drop plan pipeline (todo → doing → done → cancelled)
function renderKanban() {
  const st = window.DashboardState;
  if (!st || !st.allMissionPlans) return;
  const cols = { todo: [], doing: [], done: [], cancelled: [] };
  const statusMap = { pending: 'todo', doing: 'doing', in_progress: 'doing', blocked: 'doing', submitted: 'doing', done: 'done', cancelled: 'cancelled', completed: 'done' };
  st.allMissionPlans.forEach((m) => {
    const p = m.plan;
    if (!p) return;
    const bucket = statusMap[p.status] || 'todo';
    cols[bucket].push(m);
  });
  ["todo", "doing", "done", "cancelled"].forEach((s) => {
    const el = document.getElementById(`kanban-${s}`);
    if (!el) return;
    el.innerHTML = cols[s].map((m) => _kanbanCard(m, s)).join("") ||
      '<div class="kanban-empty">No plans</div>';
  });
}

const _trashSvg = `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="kanban-trash">
  <path d="M3 6h18M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/>
</svg>`;

function _kanbanCard(m, col) {
  const p = m.plan,
    pct = p.tasks_total > 0 ? Math.round((100 * p.tasks_done) / p.tasks_total) : 0,
    running = (m.tasks || []).filter((t) => t.status === "in_progress").length,
    host = p.execution_peer || p.execution_host || "",
    rgb = pct >= 75 ? "0,204,106" : pct >= 50 ? "230,161,23" : "238,51,68",
    border = col === "doing" ? `border-left:3px solid rgba(${rgb},0.8)` : "",
    tint = {
      todo: "",
      doing: "background:rgba(255,199,44,0.04)",
      done: "background:rgba(0,166,81,0.04)",
      cancelled: "background:rgba(255,255,255,0.02)",
    }[col] || "";
  const fillClass =
    pct >= 75 ? "mn-progress__fill--green" : pct >= 50 ? "mn-progress__fill--yellow" : "mn-progress__fill--red";
  const trashBtn = (col === "todo" || col === "doing")
    ? `<button class="kanban-trash-btn mn-btn mn-btn--sm" onclick="cancelPlan(${p.id})" title="Cancel plan">${_trashSvg}</button>`
    : "";
  return `<div class="kanban-card mn-hover-lift" draggable="true" data-plan-id="${p.id}" data-status="${p.status}"
    ondragstart="kanbanDragStart(event)" style="${border}${border && tint ? ";" : ""}${tint}">
    ${trashBtn}
    <div class="kanban-card-top">
      <span class="kanban-plan-id">#${p.id}</span>
      <span class="kanban-plan-name">${esc((p.name || "").substring(0, 22))}</span>
    </div>
    ${col === "doing" ? `<div class="mn-progress kanban-progress"><div class="mn-progress__fill ${fillClass}" style="width:${pct}%"></div></div>` : ""}
    <div class="kanban-card-meta">
      ${p.tasks_done || 0}/${p.tasks_total || 0} tasks
      ${running > 0 ? `<span class="kanban-running mn-badge mn-badge--info">${running} ${Icons.zap(11)}</span>` : ""}
      ${host ? `<span class="kanban-host mn-badge mn-badge--neutral">${esc(host.substring(0, 10))}</span>` : ""}
    </div>
  </div>`;
}

window.kanbanDragStart = function (e) {
  const card = e.target.closest(".kanban-card");
  if (!card) return;
  e.dataTransfer.setData("text/plain", card.dataset.planId);
  e.dataTransfer.effectAllowed = "move";
  card.classList.add("dragging");
  setTimeout(() => card.classList.remove("dragging"), 0);
};

window.kanbanDrop = async function (e, targetStatus) {
  e.preventDefault();
  e.currentTarget.classList.remove("drag-over");
  const planId = e.dataTransfer.getData("text/plain");
  if (!planId) return;
  const st = window.DashboardState;
  const m = st.allMissionPlans.find((x) => x.plan && String(x.plan.id) === planId);
  if (!m) return;
  const currentStatus = m.plan.status;
  if (currentStatus === targetStatus) return;

  // Validate transition
  const valid = {
    "todo→doing": true,
    "doing→todo": true,
    "doing→done": true,
    "done→todo": true,
    "todo→cancelled": true,
    "doing→cancelled": true,
    "cancelled→todo": true,
  };
  const key = `${currentStatus}→${targetStatus}`;
  if (!valid[key]) {
    showToast(`Cannot move plan from ${currentStatus} to ${targetStatus}`, "error");
    return;
  }

  // For "doing", show start dialog instead of direct POST
  if (targetStatus === 'doing') {
    const planName = m.plan.name || '';
    showStartPlanDialog(parseInt(planId, 10), planName);
    return;
  }

  // Confirm destructive actions
  if (currentStatus === "doing" && targetStatus === "todo") {
    if (!confirm(`Stop plan #${planId} and move back to pipeline?`)) return;
  }
  if (targetStatus === "done" && currentStatus === "doing") {
    if (!confirm(`Mark plan #${planId} as complete?`)) return;
  }
  if (targetStatus === "cancelled") {
    if (!confirm(`Cancel plan #${planId}? This moves it to parking lot.`)) return;
  }
  if (currentStatus === "cancelled" && targetStatus === "todo") {
    if (!confirm(`Restore plan #${planId} from parking lot to pipeline?`)) return;
  }

  // Execute via API
  try {
    const res = await fetch("/api/plan-status", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ plan_id: parseInt(planId, 10), status: targetStatus }),
    });
    const data = await res.json();
    if (data.error) {
      showToast(data.error, "error");
      return;
    }
    showToast(`Plan #${planId}: ${currentStatus} → ${targetStatus}`, "info");
    if (typeof refreshAll === "function") refreshAll();
  } catch (err) {
    showToast("Failed to update plan status", "error");
  }
};

window.renderKanban = renderKanban;
