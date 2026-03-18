/* dashboard-help.js — Onboarding tooltips + help panel */
(function () {
  'use strict';

  const ONBOARD_KEY = 'dashOnboardV1';
  const HELP_ID = 'dashboard-help-panel';

  const GUIDE = [
    { icon: '🔴🟡🟢', title: 'Traffic Lights', text: 'Every widget has macOS-style buttons. <b>Red</b> = remove widget, <b>Yellow</b> = minimize, <b>Green</b> = maximize full-width.' },
    { icon: '↕️↔️', title: 'Drag & Resize', text: 'Drag any widget by its <b>header bar</b> to move it. Grab <b>edges/corners</b> to resize. Layout auto-saves.' },
    { icon: '➕', title: 'Widget Catalog', text: 'Click the <b>+</b> button (top-right) to open the catalog. Add or remove widgets to build your dashboard.' },
    { icon: '🔄', title: 'Reset Layout', text: 'Click the <b>grid icon</b> (top-right) to reset all widgets to default positions and restore removed widgets.' },
    { icon: '🔍', title: 'Zoom', text: 'Use <b>+ / − / R</b> buttons to zoom the entire dashboard in or out.' },
    { icon: '🎨', title: 'Themes', text: 'Click the <b>sun icon</b> to switch between dark/light and Ferrari-inspired themes.' },
    { icon: '⟳', title: 'Refresh', text: 'Use the refresh stepper to set auto-refresh interval or manual mode. Click <b>⟳</b> for instant refresh.' },
  ];

  // --- Help Panel ---
  function buildHelpPanel() {
    let panel = document.getElementById(HELP_ID);
    if (panel) return panel;

    panel = document.createElement('div');
    panel.id = HELP_ID;
    panel.className = 'help-panel';
    panel.hidden = true;

    panel.innerHTML =
      '<div class="help-panel__header">' +
        '<span class="help-panel__title">Dashboard Guide</span>' +
        '<button class="help-panel__close mn-btn mn-btn--ghost" onclick="toggleDashboardHelp()">✕</button>' +
      '</div>' +
      '<div class="help-panel__body">' +
        GUIDE.map(g =>
          '<div class="help-item">' +
            '<span class="help-item__icon">' + g.icon + '</span>' +
            '<div class="help-item__content">' +
              '<div class="help-item__title">' + g.title + '</div>' +
              '<div class="help-item__text">' + g.text + '</div>' +
            '</div>' +
          '</div>'
        ).join('') +
        '<div class="help-footer">' +
          '<button class="mn-btn mn-btn--accent mn-btn--sm" onclick="toggleDashboardHelp()">Got it</button>' +
        '</div>' +
      '</div>';

    document.body.appendChild(panel);
    return panel;
  }

  window.toggleDashboardHelp = function () {
    const panel = buildHelpPanel();
    panel.hidden = !panel.hidden;
  };

  // --- First-visit onboarding toast ---
  function showOnboarding() {
    if (localStorage.getItem(ONBOARD_KEY)) return;
    localStorage.setItem(ONBOARD_KEY, '1');

    const toast = document.createElement('div');
    toast.className = 'onboard-toast';
    toast.innerHTML =
      '<div class="onboard-toast__content">' +
        '<span class="onboard-toast__icon">🎛️</span>' +
        '<div>' +
          '<div class="onboard-toast__title">Welcome to Convergio</div>' +
          '<div class="onboard-toast__text">' +
            'Drag widgets by header, resize from edges, use <b>🔴🟡🟢</b> to remove/minimize/maximize. ' +
            'Click <b>+</b> to add widgets from the catalog. Click <b>?</b> for full guide.' +
          '</div>' +
        '</div>' +
        '<button class="onboard-toast__close" onclick="this.closest(\'.onboard-toast\').remove()">✕</button>' +
      '</div>';

    document.body.appendChild(toast);
    setTimeout(() => toast.classList.add('onboard-toast--visible'), 100);
    setTimeout(() => { if (toast.parentNode) toast.remove(); }, 15000);
  }

  setTimeout(showOnboarding, 2000);
})();
