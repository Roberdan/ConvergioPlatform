// lib/embed.js — WKWebView embedded mode: query param parsing + layout control
// Enables menu bar WebView to load specific views without chrome.
'use strict';

const EMBED_CLASS = 'mode-embedded';

/** Parse URL query params relevant to embedded mode */
export function getQueryParams() {
  const params = new URLSearchParams(window.location.search);
  return {
    mode: params.get('mode'),
    tab: params.get('tab'),
    brainMode: params.get('brain_mode'),
  };
}

/** Add mode-embedded class to <html>, set window flag for brain/canvas.js */
export function applyEmbeddedMode() {
  document.documentElement.classList.add(EMBED_CLASS);
  window.__convergioEmbedded = true;
}

/** Check whether embedded mode is active */
export function isEmbedded() {
  return document.documentElement.classList.contains(EMBED_CLASS);
}
