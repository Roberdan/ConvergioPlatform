/**
 * Shared utility functions for the dashboard web UI.
 * Loaded via <script> before app.js and mesh-actions.js.
 */

/** HTML-escape utility for safe innerHTML usage. */
window.esc = (s) =>
  (window.Maranello?.escapeHtml ||
    ((v) =>
      String(v).replace(/[&<>"']/g, (c) =>
        ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c],
      )))(s);

function esc(s) {
  return window.esc(s);
}

/** Debounce a function — returns a wrapper that delays execution until ms have passed */
function debounce(fn, ms = 1000) {
  let timer;
  return function (...args) {
    const ctx = this;
    clearTimeout(timer);
    timer = setTimeout(() => fn.apply(ctx, args), ms);
  };
}
