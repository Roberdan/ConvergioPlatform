// Brain strip initialization and resize logic.
// Manages collapse/expand state and drag-to-resize.

const BRAIN_STRIP_MIN_HEIGHT = 48;

/**
 * Initialize brain strip: toggle, resize, persisted state.
 * WHY separate module: keeps app.js under 250 lines.
 */
export function initBrainStrip() {
  const strip = document.getElementById('brain-strip');
  const handle = document.getElementById('brain-strip-handle');
  const toggle = document.getElementById('brain-strip-toggle');
  if (!strip || !handle || !toggle) {
    console.warn('[brain-strip] Missing DOM elements for brain strip');
    return;
  }

  restoreState(strip, toggle);
  bindToggle(strip, toggle);
  bindResize(strip, handle);
}

function restoreState(strip, toggle) {
  const saved = localStorage.getItem('cr-brain-strip-state');
  if (saved === 'collapsed') {
    strip.dataset.brainStrip = 'collapsed';
    toggle.setAttribute('aria-expanded', 'false');
  }

  const savedHeight = parseInt(localStorage.getItem('cr-brain-strip-height'), 10);
  if (savedHeight > BRAIN_STRIP_MIN_HEIGHT) {
    strip.style.height = `${savedHeight}px`;
  }
}

function bindToggle(strip, toggle) {
  toggle.addEventListener('click', (e) => {
    e.stopPropagation();
    const isExpanded = strip.dataset.brainStrip === 'expanded';
    strip.dataset.brainStrip = isExpanded ? 'collapsed' : 'expanded';
    toggle.setAttribute('aria-expanded', String(!isExpanded));
    localStorage.setItem('cr-brain-strip-state', strip.dataset.brainStrip);
  });
}

function bindResize(strip, handle) {
  let startY = 0;
  let startHeight = 0;

  function onPointerDown(e) {
    if (e.target.closest('.cr-brain-strip__toggle')) return;
    if (strip.dataset.brainStrip === 'collapsed') return;
    startY = e.clientY;
    startHeight = strip.getBoundingClientRect().height;
    document.addEventListener('pointermove', onPointerMove);
    document.addEventListener('pointerup', onPointerUp);
    document.body.style.cursor = 'row-resize';
    e.preventDefault();
  }

  function onPointerMove(e) {
    const delta = startY - e.clientY;
    const next = Math.max(
      BRAIN_STRIP_MIN_HEIGHT,
      Math.min(startHeight + delta, window.innerHeight * 0.5)
    );
    strip.style.height = `${next}px`;
  }

  function onPointerUp() {
    document.removeEventListener('pointermove', onPointerMove);
    document.removeEventListener('pointerup', onPointerUp);
    document.body.style.cursor = '';
    const h = strip.getBoundingClientRect().height;
    localStorage.setItem('cr-brain-strip-height', String(Math.round(h)));
  }

  handle.addEventListener('pointerdown', onPointerDown);
}
