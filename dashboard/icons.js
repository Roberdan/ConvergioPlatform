/**
 * Centralized icon library — delegates to Maranello DS icons (stroke-width 1.5).
 * Falls back to inline SVG only for icons not in the DS catalog.
 */
const _mn = (name) => {
  const api = window.Maranello;
  if (api && api.icons && typeof api.icons[name] === 'function') return api.icons[name]();
  return null;
};
const _wrap = (size, svg) =>
  '<span class="mn-icon" style="width:' + size + 'px;height:' + size + 'px;display:inline-flex;vertical-align:-2px">' +
  svg.replace(/<svg /, '<svg width="' + size + '" height="' + size + '" ') + '</span>';

const _ic = (size, name, fallbackPath) => {
  const mnSvg = _mn(name);
  if (mnSvg) return _wrap(size, mnSvg);
  return '<svg width="' + size + '" height="' + size + '" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:-2px;display:inline-block">' + fallbackPath + '</svg>';
};

const Icons = {
  brain: (s = 14) => _ic(s, 'brain', '<path d="M9 5a3 3 0 0 0-5 2v1a2.5 2.5 0 0 0 1.5 4.5V14a3 3 0 0 0 3 3h1.2M15 5a3 3 0 0 1 5 2v1a2.5 2.5 0 0 1-1.5 4.5V14a3 3 0 0 1-3 3h-1.2"/>'),
  clock: (s = 14) => _ic(s, 'clock', '<circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/>'),
  eye: (s = 14) => _ic(s, 'eye', '<path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/>'),
  gitMerge: (s = 14) => _ic(s, 'gitMerge', '<circle cx="6" cy="5" r="2"/><circle cx="6" cy="19" r="2"/><circle cx="18" cy="12" r="2"/><path d="M8 5h2a6 6 0 0 1 6 6M8 19h2a6 6 0 0 0 6-6"/>'),
  gitPull: (s = 14) => _ic(s, 'gitPull', '<circle cx="6" cy="6" r="3"/><circle cx="18" cy="18" r="3"/><path d="M6 9v12M18 15V6h-6"/>'),
  shield: (s = 14) => _ic(s, 'shield', '<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><path d="M9 12l2 2 4-4"/>'),
  cpu: (s = 14) => _ic(s, 'cpu', '<rect x="7" y="7" width="10" height="10" rx="1.5"/><rect x="10" y="10" width="4" height="4" rx=".6"/><path d="M9 2v3M15 2v3M9 19v3M15 19v3M2 9h3M2 15h3M19 9h3M19 15h3"/>'),
  check: (s = 14) => _ic(s, 'check', '<path d="M20 6L9 17l-5-5"/>'),
  x: (s = 14) => _ic(s, 'close', '<path d="M18 6L6 18M6 6l12 12"/>'),
  checkCircle: (s = 14) => _ic(s, 'checkCircle', '<path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><path d="M22 4L12 14.01l-3-3"/>'),
  xCircle: (s = 14) => _ic(s, 'xCircle', '<circle cx="12" cy="12" r="10"/><path d="M15 9l-6 6M9 9l6 6"/>'),
  alertTriangle: (s = 14) => _ic(s, 'alertTriangle', '<path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><path d="M12 9v4M12 17h.01"/>'),
  alertCircle: (s = 14) => _ic(s, 'alertCircle', '<circle cx="12" cy="12" r="10"/><path d="M12 8v4M12 16h.01"/>'),
  search: (s = 14) => _ic(s, 'search', '<circle cx="11" cy="11" r="8"/><path d="M21 21l-4.35-4.35"/>'),
  zap: (s = 14) => _ic(s, 'zap', '<polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/>'),
  globe: (s = 14) => _ic(s, 'globe', '<circle cx="12" cy="12" r="10"/><path d="M2 12h20"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>'),
  monitor: (s = 14) => _ic(s, 'monitor', '<rect x="3" y="4" width="18" height="12" rx="2"/><path d="M8 20h8M12 16v4"/>'),
  waveComplete: (s = 14) => _ic(s, 'deploy', '<path d="M12 3l3 6 6 3-6 3-3 6-3-6-6-3 6-3z"/><path d="M12 9v6"/>'),
  dot: (s = 14) => _ic(s, 'dot', '<circle cx="12" cy="12" r="4"/>'),
  calendar: (s = 14) => _ic(s, 'calendar', '<rect x="3" y="4" width="18" height="18" rx="2"/><path d="M16 2v4M8 2v4M3 10h18"/>'),
  nightAgent: (s = 14) => _ic(s, 'nightAgent', '<path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9z"/><circle cx="12" cy="15" r="1"/><path d="M10 12h4"/>'),
  moonClock: (s = 14) => _ic(s, 'moonClock', '<path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9z"/><circle cx="12" cy="12" r="3"/><path d="M12 10v2l1.5 1"/>'),
  sync: (s = 14) => _ic(s, 'sync', '<path d="M21 2v6h-6"/><path d="M3 12a9 9 0 0 1 15-6.7L21 8"/><path d="M3 22v-6h6"/><path d="M21 12a9 9 0 0 1-15 6.7L3 16"/>'),
  refresh: (s = 14) => _ic(s, 'refresh', '<path d="M21 2v6h-6"/><path d="M3 12a9 9 0 0 1 15-6.7L21 8"/><path d="M3 22v-6h6"/><path d="M21 12a9 9 0 0 1-15 6.7L3 16"/>'),
  start: (s = 14) => _ic(s, 'start', '<polygon points="5 3 19 12 5 21 5 3"/>'),
  pause: (s = 14) => _ic(s, 'pause', '<rect x="6" y="4" width="4" height="16"/><rect x="14" y="4" width="4" height="16"/>'),
  stop: (s = 14) => _ic(s, 'stop', '<rect x="4" y="4" width="16" height="16" rx="2"/>'),
  fixOn: (s = 14) => _ic(s, 'fixOn', '<path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/>'),
  project: (s = 14) => _ic(s, 'project', '<path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>'),
  runNow: (s = 14) => _ic(s, 'runNow', '<polygon points="5 3 19 12 5 21 5 3"/><path d="M12 12h.01"/>'),
  timer: (s = 14) => _ic(s, 'clock', '<circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/>'),
};

window.Icons = Icons;
