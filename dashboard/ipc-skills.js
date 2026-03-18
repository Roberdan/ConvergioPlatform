// ipc-skills.js — Skill pool matrix panel for dashboard
// Uses Maranello DS components (mn-widget, mn-card)

async function renderIpcSkills(container) {
  if (!container) return;
  const data = await fetchJson('/api/ipc/skills');
  if (!data || !data.skills || !data.skills.length) {
    container.innerHTML = '<mn-widget title="Skill Pool"><p>No skills registered</p></mn-widget>';
    return;
  }
  // Build heatmap grid: agents × skills
  const skills = data.skills;
  const skillNames = [...new Set(skills.map(s => s.skill))].sort();
  const agents = [...new Set(skills.map(s => `${s.agent}@${s.host}`))].sort();
  const lookup = {};
  skills.forEach(s => { lookup[`${s.agent}@${s.host}:${s.skill}`] = s.confidence; });

  const headerRow = `<tr><th></th>${skillNames.map(s => `<th style="font-size:10px;transform:rotate(-45deg);white-space:nowrap">${s}</th>`).join('')}</tr>`;
  const rows = agents.map(agent => {
    const cells = skillNames.map(skill => {
      const conf = lookup[`${agent}:${skill}`];
      if (conf === undefined) return '<td style="background:#1e293b;width:28px;height:28px"></td>';
      // Heatmap: 0=gray, 0.5=yellow, 1.0=green
      const r = conf < 0.5 ? 128 : Math.round(255 - conf * 255);
      const g = Math.round(conf * 200 + 55);
      const b = conf < 0.5 ? 128 : 50;
      return `<td style="background:rgb(${r},${g},${b});width:28px;height:28px;text-align:center;font-size:10px;color:#fff" title="${agent}: ${skill} (${conf.toFixed(2)})">${conf.toFixed(1)}</td>`;
    }).join('');
    return `<tr><td style="font-size:11px;white-space:nowrap;padding-right:8px">${agent}</td>${cells}</tr>`;
  }).join('');

  const matrix = `
    <mn-widget title="Skill Pool Matrix">
      <div style="overflow-x:auto">
        <table class="mn-table" style="border-collapse:collapse">${headerRow}${rows}</table>
      </div>
      <div style="font-size:10px;color:#64748b;margin-top:8px">
        Confidence heatmap: <span style="color:#808080">gray=0</span> → <span style="color:#c8b832">yellow=0.5</span> → <span style="color:#22c832">green=1.0</span>
      </div>
    </mn-widget>`;
  container.innerHTML = matrix;
}

window.renderIpcSkills = renderIpcSkills;
