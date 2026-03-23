/**
 * Bottom drawer widget — VS Code-style resizable terminal panel.
 * Fixed to viewport bottom, supports multiple terminal tabs.
 * Collapsed by default on each page load (no localStorage).
 */
import { createSession } from './drawer-terminal.js';

const STYLE_ID = 'mn-drawer-bottom-style';
const MIN_HEIGHT = 100;
const MAX_HEIGHT_VH = 0.6;
const ANIM_MS = 200;
const DEFAULT_HEIGHT = 260;

function injectStyles() {
  if (document.getElementById(STYLE_ID)) return;
  const style = document.createElement('style');
  style.id = STYLE_ID;
  style.textContent = `
    .drawer-bottom {
      position: fixed; bottom: 0; left: var(--mn-sidebar-width, 240px); right: 0;
      z-index: 50; display: flex; flex-direction: column;
      background: var(--mn-surface); border-top: 1px solid var(--mn-border);
      transform: translateY(100%); transition: transform ${ANIM_MS}ms ease-out;
    }
    .drawer-bottom--open { transform: translateY(0); }
    .drawer-bottom__drag {
      height: 4px; cursor: ns-resize; flex-shrink: 0; position: relative;
    }
    .drawer-bottom__drag::after {
      content: ''; position: absolute; left: 50%; top: 50%;
      transform: translate(-50%, -50%); width: 40px; height: 3px;
      border-radius: 2px; background: var(--mn-border); transition: background 0.15s;
    }
    .drawer-bottom__drag:hover::after { background: var(--mn-accent); }
    .drawer-bottom__header {
      display: flex; align-items: center; border-bottom: 1px solid var(--mn-border);
      padding: 0 0.5rem; flex-shrink: 0; height: 2rem;
    }
    .drawer-bottom__tab,
    .drawer-bottom__add,
    .drawer-bottom__tab-close {
      cursor: pointer; border: none; background: transparent; color: var(--mn-text-muted);
    }
    .drawer-bottom__tab {
      padding: 0.25rem 0.75rem; font-size: 0.8rem;
      border-bottom: 2px solid transparent; margin-bottom: -1px;
      transition: color 0.15s, border-color 0.15s;
    }
    .drawer-bottom__tab:hover { color: var(--mn-text); }
    .drawer-bottom__tab--active {
      color: var(--mn-accent); border-bottom-color: var(--mn-accent);
    }
    .drawer-bottom__tab-close {
      margin-left: 0.25rem; opacity: 0.5; color: inherit; font-size: 0.7rem; padding: 0 2px;
    }
    .drawer-bottom__tab-close:hover { opacity: 1; }
    .drawer-bottom__add { padding: 0.25rem 0.5rem; font-size: 0.9rem; }
    .drawer-bottom__add:hover { color: var(--mn-accent); }
    .drawer-bottom__body { flex: 1; overflow: hidden; min-height: 0; }
  `;
  document.head.appendChild(style);
}

/**
 * Create and mount the bottom drawer.
 * @returns {{toggle: Function, destroy: Function, el: HTMLElement}}
 */
export function createDrawer() {
  injectStyles();

  let drawerHeight = DEFAULT_HEIGHT;
  let isOpen = false;
  const sessions = [];
  let activeIdx = -1;

  // Build DOM
  const drawer = document.createElement('div');
  drawer.className = 'drawer-bottom';
  drawer.setAttribute('role', 'region');
  drawer.setAttribute('aria-label', 'Terminal panel');
  drawer.style.height = `${drawerHeight}px`;

  const dragHandle = document.createElement('div');
  dragHandle.className = 'drawer-bottom__drag';
  dragHandle.setAttribute('role', 'separator');
  dragHandle.setAttribute('aria-orientation', 'horizontal');
  dragHandle.setAttribute('aria-label', 'Resize terminal panel');

  const header = document.createElement('div');
  header.className = 'drawer-bottom__header';

  const addBtn = document.createElement('button');
  addBtn.className = 'drawer-bottom__add';
  addBtn.textContent = '+';
  addBtn.title = 'New terminal (Ctrl+Shift+`)';

  const body = document.createElement('div');
  body.className = 'drawer-bottom__body';

  drawer.append(dragHandle, header, body);
  document.body.appendChild(drawer);

  // Tab rendering
  function renderTabs() {
    header.innerHTML = '';
    sessions.forEach((s, i) => {
      const tab = document.createElement('button');
      tab.className = 'drawer-bottom__tab';
      if (i === activeIdx) tab.classList.add('drawer-bottom__tab--active');
      tab.textContent = s.label;
      tab.addEventListener('click', () => switchTab(i));
      if (sessions.length > 1) {
        const closeBtn = document.createElement('span');
        closeBtn.className = 'drawer-bottom__tab-close';
        closeBtn.textContent = '\u00d7';
        closeBtn.addEventListener('click', (e) => {
          e.stopPropagation();
          closeTab(i);
        });
        tab.appendChild(closeBtn);
      }
      header.appendChild(tab);
    });
    header.appendChild(addBtn);
  }

  function switchTab(idx) {
    if (idx === activeIdx || idx < 0 || idx >= sessions.length) return;
    activeIdx = idx;
    body.innerHTML = '';
    const mount = document.createElement('div');
    mount.style.cssText = 'width:100%;height:100%';
    body.appendChild(mount);
    const session = sessions[idx];
    if (session.handle) {
      session.handle.term.open(mount);
    } else {
      session.handle = createSession(mount);
    }
    renderTabs();
  }

  function addTab() {
    const idx = sessions.length;
    sessions.push({ label: `Shell ${idx + 1}`, handle: null });
    switchTab(idx);
    if (!isOpen) toggle();
  }

  function closeTab(idx) {
    if (idx < 0 || idx >= sessions.length) return;
    const s = sessions[idx];
    if (s.handle) s.handle.cleanup();
    sessions.splice(idx, 1);
    if (sessions.length === 0) {
      activeIdx = -1;
      body.innerHTML = '';
      toggle();
      renderTabs();
      return;
    }
    const next = Math.min(idx, sessions.length - 1);
    activeIdx = -1;
    switchTab(next);
  }

  addBtn.addEventListener('click', addTab);

  // Toggle open/close
  function toggle() {
    isOpen = !isOpen;
    drawer.classList.toggle('drawer-bottom--open', isOpen);
    if (isOpen && sessions.length === 0) addTab();
  }

  // Drag resize
  function onDragStart(e) {
    e.preventDefault();
    const startY = e.clientY;
    const startH = drawerHeight;
    const maxH = window.innerHeight * MAX_HEIGHT_VH;
    function onMove(ev) {
      const delta = startY - ev.clientY;
      drawerHeight = Math.max(MIN_HEIGHT, Math.min(maxH, startH + delta));
      drawer.style.height = `${drawerHeight}px`;
    }
    function onUp() {
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup', onUp);
    }
    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
  }
  dragHandle.addEventListener('mousedown', onDragStart);

  // Keyboard shortcut: Ctrl+` to toggle
  function onKeydown(e) {
    if ((e.ctrlKey || e.metaKey) && e.key === '`') {
      e.preventDefault();
      toggle();
    }
  }
  document.addEventListener('keydown', onKeydown);

  function destroy() {
    document.removeEventListener('keydown', onKeydown);
    for (const s of sessions) {
      if (s.handle) s.handle.cleanup();
    }
    drawer.remove();
  }

  // Start with first tab created but drawer collapsed
  addTab();
  isOpen = true;
  toggle();

  return { toggle, destroy, el: drawer };
}
