// activity-feed.js — Unified activity feed (GitHub + System events)

(function() {
  const feedContainer = document.getElementById('activity-feed-content');
  if (!feedContainer) return;

  // Store merged events
  window._activityFeedItems = [];

  function _relativeTime(ts) {
    const diff = Date.now() - new Date(ts).getTime();
    const mins = Math.floor(diff / 60000);
    if (mins < 1) return 'just now';
    if (mins < 60) return mins + 'm ago';
    const hrs = Math.floor(mins / 60);
    if (hrs < 24) return hrs + 'h ago';
    const days = Math.floor(hrs / 24);
    return days + 'd ago';
  }

  function _eventIcon(type) {
    const icons = {
      push: '↑', pull_request: '⇆', issues: '●', review: '✎',
      workflow_run: '⚙', release: '◆', create: '+', delete: '−',
      nightly: '☾', backup: '💾', deploy: '🚀', cron: '⏱'
    };
    return icons[type] || '•';
  }

  function renderActivityFeed(items) {
    window._activityFeedItems = items;
    if (!feedContainer) return;

    const activeFilter = document.querySelector('.activity-filter-btn.active');
    const filter = activeFilter ? activeFilter.dataset.filter : 'all';

    const filtered = filter === 'all' ? items : items.filter(i => i.source === filter);

    if (!filtered.length) {
      feedContainer.innerHTML = '<div class="mn-loading-placeholder" style="padding:16px;text-align:center;color:var(--text-dim)">No activity</div>';
      return;
    }

    feedContainer.innerHTML = filtered.slice(0, 30).map(item => {
      const failed = item.status === 'failed' || item.status === 'failure';
      const statusCls = failed ? 'feed-item-failed' : '';
      const badgeClass = failed
        ? 'mn-badge mn-badge--danger'
        : item.source === 'github'
          ? 'mn-badge mn-badge--info'
          : 'mn-badge mn-badge--warning';
      return `<div class="feed-item feed-${item.source} ${statusCls}" data-source="${item.source}">
        <span class="feed-icon">${_eventIcon(item.type)}</span>
        <span class="${badgeClass}">${item.source === 'github' ? 'GH' : 'SYS'}</span>
        <span class="feed-text">${item.title}</span>
        <span class="feed-time">${_relativeTime(item.timestamp)}</span>
        ${failed ? '<span class="feed-alert">⚠</span>' : ''}
      </div>`;
    }).join('');
  }

  window.renderActivityFeed = renderActivityFeed;

  // Merge function called from existing renderers
  window.mergeActivityEvents = function(githubEvents, systemEvents) {
    const merged = [];
    if (githubEvents) {
      githubEvents.forEach(e => merged.push({
        source: 'github',
        type: e.type || 'push',
        title: e.title || e.message || e.repo + ' ' + (e.type || ''),
        timestamp: e.created_at || e.timestamp || new Date().toISOString(),
        status: e.status || 'ok'
      }));
    }
    if (systemEvents) {
      systemEvents.forEach(e => merged.push({
        source: 'system',
        type: e.type || 'nightly',
        title: e.title || e.name || e.description || 'System event',
        timestamp: e.timestamp || e.scheduled_at || e.last_run || new Date().toISOString(),
        status: e.status || 'ok'
      }));
    }
    merged.sort((a, b) => new Date(b.timestamp) - new Date(a.timestamp));
    renderActivityFeed(merged);
  };
})();
