/* plan-timeline.js — Plan Gantt timeline widget */
;(function() {
  'use strict';

  const STATUS_STATE = { done: 'completed', doing: 'in_progress', in_progress: 'in_progress', todo: 'pending', pending: 'pending', cancelled: 'withdrawn', blocked: 'errors' };
  const STATE_COLOR = { completed: '#00A651', in_progress: '#FFC72C', pending: '#4EA8DE', withdrawn: '#616161', errors: '#ff3355' };

  let _ganttCtrl = null;
  let _currentDays = 30;

  function openPlanFromGantt(task) {
    if (!task || !task.id) return;
    var id = String(task.id);
    // Extract plan ID from either plan-123 or task-456
    var planId = null;
    if (id.startsWith('plan-')) planId = id.replace('plan-', '');
    else if (id.startsWith('task-')) planId = id.replace('task-', '');
    if (planId && typeof window.openPlanSidebar === 'function') {
      window.openPlanSidebar(planId);
    }
  }

  function toISO(dt) {
    if (!dt) return null;
    var d = new Date(String(dt).replace(' ', 'T'));
    return isNaN(d.getTime()) ? null : d.toISOString().split('T')[0];
  }

  function planToGantt(plan) {
    var start = toISO(plan.started_at || plan.created_at);
    var endRaw = plan.completed_at || plan.cancelled_at;
    var end = toISO(endRaw);
    if (!start) return null;
    // Plans that start and end same day: extend end by 1 day for visibility
    if (!end || end === start) {
      var d = new Date(start + 'T12:00:00');
      d.setDate(d.getDate() + 1);
      end = d.toISOString().split('T')[0];
    }
    var total = Number(plan.tasks_total) || 1;
    var done = Number(plan.tasks_done) || 0;
    return {
      id: 'plan-' + plan.id,
      title: '#' + plan.id + ' ' + (plan.name || '').substring(0, 50),
      start: start, end: end,
      progress: Math.round((done / total) * 100),
      state: STATUS_STATE[plan.status] || 'pending',
      type: 'task',
      badges: [done + '/' + total + ' tasks', plan.execution_host || ''].filter(Boolean)
    };
  }

  function groupByProject(plans) {
    var projects = {};
    var order = [];
    plans.forEach(function(p) {
      var gt = planToGantt(p);
      if (!gt) return;
      var proj = p.project_id || 'unknown';
      if (!projects[proj]) { projects[proj] = []; order.push(proj); }
      projects[proj].push(gt);
    });
    return order.map(function(proj) {
      var items = projects[proj];
      var starts = items.map(function(i) { return i.start; }).sort();
      var ends = items.map(function(i) { return i.end; }).sort().reverse();
      return {
        id: 'proj-' + proj,
        title: proj,
        start: starts[0],
        end: ends[0],
        progress: 0,
        state: 'active',
        type: 'group',
        children: items
      };
    });
  }

  function renderFallback(container, groups) {
    var html = '<div style="padding:8px;overflow-x:auto;max-height:500px;overflow-y:auto;">';
    html += '<table style="width:100%;border-collapse:collapse;font:11px/1.8 var(--font-mono,monospace);color:var(--text,#c8d0e8);">';
    html += '<tr style="border-bottom:1px solid var(--border,#2a2a3a);position:sticky;top:0;background:var(--bg-card,#111);z-index:1;"><th style="text-align:left;padding:4px 8px;">Plan</th><th>Status</th><th>Progress</th><th>Start</th><th>End</th><th>Tasks</th></tr>';
    groups.forEach(function(g) {
      // Project header
      html += '<tr style="border-bottom:1px solid var(--border,#2a2a3a);background:var(--bg-deep,#080808);"><td colspan="6" style="padding:6px 8px;font-weight:bold;color:var(--accent,#FFC72C);letter-spacing:1px;font-size:10px;text-transform:uppercase;">' + (g.title || 'Unknown') + ' <span style="color:var(--text-dim);font-weight:normal;">(' + (g.children||[]).length + ' plans)</span></td></tr>';
      (g.children || []).forEach(function(t) {
        var col = STATE_COLOR[t.state] || '#888';
        var pct = t.progress || 0;
        var bar = '<div style="width:80px;height:8px;background:#1a1a2a;border-radius:4px;overflow:hidden;display:inline-block;vertical-align:middle;"><div style="width:' + pct + '%;height:100%;background:' + col + ';border-radius:4px;"></div></div>';
        html += '<tr style="border-bottom:1px solid #1a1a2a;cursor:pointer;" onclick="typeof openPlanSidebar===\'function\'&&openPlanSidebar(\'' + String(t.id).replace('plan-','') + '\')">';
        html += '<td style="padding:4px 8px 4px 20px;max-width:280px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">' + (t.title || '') + '</td>';
        html += '<td style="text-align:center;color:' + col + ';text-transform:uppercase;font-size:9px;letter-spacing:1px;">' + (t.state || '') + '</td>';
        html += '<td style="text-align:center;">' + bar + ' <span style="color:var(--text-dim);font-size:10px;">' + pct + '%</span></td>';
        html += '<td style="text-align:center;color:var(--text-dim);font-size:10px;">' + (t.start || '') + '</td>';
        html += '<td style="text-align:center;color:var(--text-dim);font-size:10px;">' + (t.end || '') + '</td>';
        html += '<td style="text-align:center;">' + ((t.badges||[])[0]||'') + '</td>';
        html += '</tr>';
      });
    });
    html += '</table></div>';
    container.innerHTML = html;
  }

  async function loadTimeline(days) {
    _currentDays = days || _currentDays;
    var container = document.getElementById('plan-timeline-container');
    if (!container) { if (window.DashLog) DashLog.warn('Timeline', '', 'container not found'); return; }

    try {
      var resp = await fetch('/api/plans/timeline?days=' + _currentDays);
      if (!resp.ok) { if (window.DashLog) DashLog.error('Timeline', '/api/plans/timeline', 'HTTP ' + resp.status); return; }
      var data = await resp.json();
      var plans = data.plans || [];
      var ganttTasks = groupByProject(plans);
      if (window.DashLog) DashLog.info('Timeline', '', plans.length + ' plans, ' + ganttTasks.length + ' projects');

      if (ganttTasks.length === 0) {
        container.innerHTML = '<div style="padding:24px;color:var(--text-dim);text-align:center;">No plans in the last ' + _currentDays + ' days</div>';
        return;
      }

      // Try Maranello gantt headless API
      if (window.Maranello && typeof window.Maranello.gantt === 'function') {
        try {
          if (_ganttCtrl) {
            _ganttCtrl.setTasks(ganttTasks);
          } else {
            container.innerHTML = '';
            _ganttCtrl = window.Maranello.gantt(container, ganttTasks, {
              labelWidth: 300,
              onSelect: function(task, type) {
                openPlanFromGantt(task);
              },
              onClick: function(task, type) {
                openPlanFromGantt(task);
              }
            });
          }
          setTimeout(function() { if (_ganttCtrl && _ganttCtrl.scrollToToday) _ganttCtrl.scrollToToday(); }, 300);
          return;
        } catch (ganttErr) {
          if (window.DashLog) DashLog.warn('Timeline', '', 'Gantt init failed: ' + ganttErr.message + ', using fallback');
          _ganttCtrl = null;
        }
      }

      // Fallback table
      renderFallback(container, ganttTasks);
    } catch (e) {
      if (window.DashLog) DashLog.error('Timeline', '', e.message);
    }
  }

  function renderTimelineControls() {
    var controls = document.getElementById('timeline-controls');
    if (!controls) return;
    var ranges = [
      { label: '7d', days: 7 }, { label: '14d', days: 14 },
      { label: '30d', days: 30 }, { label: '90d', days: 90 }, { label: 'All', days: 365 }
    ];
    controls.innerHTML = ranges.map(function(r) {
      return '<button class="mn-btn mn-btn--ghost mn-btn--sm timeline-range-btn' + (r.days === _currentDays ? ' active' : '') + '" onclick="window._loadTimeline(' + r.days + ')">' + r.label + '</button>';
    }).join('');
  }

  window._loadTimeline = function(days) { loadTimeline(days); renderTimelineControls(); };
  window.renderTimeline = function() { renderTimelineControls(); loadTimeline(_currentDays); };
})();
