/**
 * Mesh action toolbar — moved from app.js.
 * XSS-safe: uses data-peer / data-action attributes with event delegation.
 */

let _meshActionInFlight = false;

window.parseSseJson = function (raw) {
  try {
    return JSON.parse(raw);
  } catch {
    console.warn("Malformed SSE:", raw?.substring(0, 100));
    return null;
  }
};

window.attachSseCleanup = function (es, overlay, onCleanup) {
  let cleaned = false;
  let observer = null;
  const handleBeforeUnload = () => cleanup();
  const cleanup = () => {
    if (cleaned) return;
    cleaned = true;
    window.removeEventListener("beforeunload", handleBeforeUnload);
    if (observer) observer.disconnect();
    try {
      es.close();
    } catch {}
    if (typeof onCleanup === "function") onCleanup();
  };
  window.addEventListener("beforeunload", handleBeforeUnload, { once: true });
  if (overlay && document.body) {
    observer = new MutationObserver(() => {
      if (!document.body.contains(overlay)) cleanup();
    });
    observer.observe(document.body, { childList: true, subtree: true });
  }
  return cleanup;
};

window.meshAction = async function (action, peer) {
  if (action === "edit") {
    const peers = Array.isArray(lastMeshData) ? lastMeshData : [];
    const peerData = peers.find((p) => p.peer_name === peer);
    if (peerData && typeof showPeerForm === "function")
      showPeerForm("edit", peerData);
    return;
  }
  if (action === "delete") {
    if (typeof showDeleteDialog === "function") showDeleteDialog(peer);
    return;
  }
  if (action === "terminal") {
    if (typeof termMgr !== "undefined") {
      const peers = Array.isArray(lastMeshData) ? lastMeshData : [];
      const peerData = peers.find((p) => p.peer_name === peer);
      const activePlan = peerData
        ? (peerData.plans || []).find(
            (pl) => pl.status === "doing" || pl.status === "todo",
          )
        : null;
      const tmuxSession = "Convergio";
      termMgr.open(peer, peer, tmuxSession);
    }
    return;
  }
  if (action === "movehere") {
    showMovePlanDialog(peer);
    return;
  }
  if (action === "fullsync") {
    runFullSync(peer === "__all__" ? "" : peer);
    return;
  }
  if (action === "reboot" && !confirm("Are you sure?")) return;
  // Stream all other actions via SSE
  if (_meshActionInFlight) return;
  _meshActionInFlight = true;
  try {
    streamMeshAction(action, peer, () => {
      _meshActionInFlight = false;
    });
  } catch (err) {
    _meshActionInFlight = false;
    if (typeof showOutputModal === "function") {
      showOutputModal("Mesh Action Error", err.message || "Failed to start action");
    }
  }
};

/**
 * Stream a mesh action via SSE with live output modal.
 */
window.streamMeshAction = function (action, peer, onCleanup) {
  const actionLabels = {
    sync: "Sync Config",
    fullsync: "Full Bidirectional Sync",
    heartbeat: "Heartbeat Status",
    auth: "Auth Sync",
    status: "Load Status",
    wake: "Wake-on-LAN",
    reboot: "SSH Reboot",
  };
  const target = peer === "__all__" ? "All Peers" : peer;
  const title = `${actionLabels[action] || action} — ${target}`;

  const overlay = document.createElement("div");
  overlay.className = "modal-overlay";
  overlay.innerHTML = `<div class="modal-box" style="max-width:650px">
    <div class="modal-title">${esc(title)}<span class="modal-close" onclick="this.closest('.modal-overlay').remove()">✕</span></div>
    <pre class="modal-output" id="mesh-action-output" style="min-height:150px"></pre>
  </div>`;
  document.body.appendChild(overlay);
  overlay.addEventListener("click", (e) => {
    if (e.target === overlay) overlay.remove();
  });

  const output = document.getElementById("mesh-action-output");
  const url = `/api/mesh/action/stream?action=${encodeURIComponent(action)}&peer=${encodeURIComponent(peer)}`;
  const es = new EventSource(url);
  const cleanup =
    typeof attachSseCleanup === "function"
      ? attachSseCleanup(es, overlay, onCleanup)
      : () => {
          try {
            es.close();
          } catch {}
          if (typeof onCleanup === "function") onCleanup();
        };

  es.addEventListener("log", (e) => {
    const line = e.data || "";
    const hasAnsi = /\x1b\[/.test(line);
    let html;
    if (hasAnsi) {
      html = ansiToHtml(line);
    } else if (line.startsWith("▶")) {
      html = `<span style="color:var(--cyan)">${esc(line)}</span>`;
    } else if (/^(OK|PASS|✓|MATCH|synced|pushed|done)/i.test(line)) {
      html = `<span style="color:var(--green)">${esc(line)}</span>`;
    } else if (/^(WARN|SKIP|MISMATCH)/i.test(line)) {
      html = `<span style="color:var(--gold)">${esc(line)}</span>`;
    } else if (/^(ERROR|FAIL|✗)/i.test(line)) {
      html = `<span style="color:var(--red)">${esc(line)}</span>`;
    } else if (line.startsWith("---") || line.startsWith("===")) {
      html = `<span style="color:var(--cyan);font-weight:600">${esc(line)}</span>`;
    } else {
      html = esc(line);
    }
    output.innerHTML += html + "\n";
    output.scrollTop = output.scrollHeight;
  });

  es.addEventListener("done", (e) => {
    const data = typeof parseSseJson === "function" ? parseSseJson(e.data) : null;
    cleanup();
    if (!data) {
      output.innerHTML += `\n<span style="color:var(--red)">${Icons.x(14)} Invalid response</span>\n`;
      output.scrollTop = output.scrollHeight;
      return;
    }
    if (data.ok) {
      output.innerHTML += `\n<span style="color:var(--green);font-weight:600">${Icons.checkCircle(14)} Completed successfully</span>\n`;
    } else {
      const msg = data.message || `Exit code ${data.exit_code || "?"}`;
      output.innerHTML += `\n<span style="color:var(--red);font-weight:600">${Icons.xCircle(14)} ${esc(msg)}</span>\n`;
    }
    output.scrollTop = output.scrollHeight;
    if (typeof refreshAll === "function") refreshAll();
  });

  es.onerror = () => {
    cleanup();
    output.innerHTML += `\n<span style="color:var(--red)">${Icons.x(14)} Connection lost</span>\n`;
  };
};

function ansiToHtml(raw) {
  if (!raw) return "";
  const colorMap = {
    30: "#0a0e1a",
    31: "#ff3355",
    32: "#00ff88",
    33: "#ffb700",
    34: "#00e5ff",
    35: "#ff2daa",
    36: "#00e5ff",
    37: "#c8d0e8",
    90: "#5a6080",
    91: "#ff5577",
    92: "#33ff99",
    93: "#ffd044",
    94: "#44eeff",
    95: "#ff55cc",
    96: "#44eeff",
    97: "#e0e4f0",
  };
  let html = "";
  let open = false;
  const parts = raw.split(/(\x1b\[[0-9;]*m)/);
  for (const part of parts) {
    const m = part.match(/^\x1b\[([0-9;]*)m$/);
    if (m) {
      const codes = m[1].split(";");
      if (codes.includes("0") || m[1] === "") {
        if (open) html += "</span>";
        open = false;
      } else {
        if (open) html += "</span>";
        let color = null;
        let bold = false;
        for (const c of codes) {
          if (c === "1") bold = true;
          if (colorMap[c]) color = colorMap[c];
        }
        if (color) {
          html += `<span style="color:${color}${bold ? ";font-weight:600" : ""}">`;
          open = true;
        }
      }
    } else {
      html += esc(part);
    }
  }
  if (open) html += "</span>";
  return html;
}

window.showOutputModal = function (title, text) {
  const overlay = document.createElement("div");
  overlay.className = "modal-overlay";
  const hasAnsi = /\x1b\[/.test(text);
  const body = hasAnsi ? ansiToHtml(text) : esc(text);
  overlay.innerHTML = `<div class="modal-box">
    <div class="modal-title">${esc(title)}<span class="modal-close" onclick="this.closest('.modal-overlay').remove()">\u2715</span></div>
    <pre class="modal-output">${body}</pre>
  </div>`;
  document.body.appendChild(overlay);
  overlay.addEventListener("click", (e) => {
    if (e.target === overlay) overlay.remove();
  });
};

// Event delegation for .mn-mesh-action — uses data-peer / data-action (XSS-safe)
document.addEventListener("click", (e) => {
  const btn = e.target.closest(".mn-mesh-action");
  if (btn) {
    e.stopPropagation();
    meshAction(btn.dataset.action, btn.dataset.peer);
  }
});
