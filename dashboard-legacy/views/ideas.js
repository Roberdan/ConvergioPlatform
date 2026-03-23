/**
 * Ideas view — idea capture list with create/edit via modal and side-panel.
 * Uses mn-data-table for listing, mn-modal for creation, and EntityWorkbench
 * via orchestrator for detail editing.
 */

const { StateScaffold } = window.Maranello;

const TABLE_COLUMNS = JSON.stringify([
  { key: 'title', label: 'Title' },
  { key: 'tags', label: 'Tags' },
  { key: 'priority', label: 'Priority' },
  { key: 'status', label: 'Status' },
  { key: 'project_id', label: 'Project' },
  { key: 'created_at', label: 'Created' },
]);

const STATUS_OPTIONS = ['draft', 'active', 'ready', 'promoted', 'archived'];
const PRIORITY_OPTIONS = ['P0', 'P1', 'P2', 'P3'];

/**
 * Format a raw idea row for display.
 * @param {object} idea — raw idea from API
 * @returns {object}
 */
function ideaRow(idea) {
  const tags = Array.isArray(idea.tags)
    ? idea.tags.join(', ')
    : (idea.tags || '');
  return {
    ...idea,
    tags,
    status: window.statusDot(idea.status || 'draft') + ' ' + window.esc(idea.status || 'draft'),
    created_at: idea.created_at ? new Date(idea.created_at).toLocaleDateString() : '',
  };
}

/**
 * Build a select element for filters.
 * @param {string} id — element ID
 * @param {string} label — display label
 * @param {string[]} options — option values
 * @returns {string} HTML
 */
function filterSelect(id, label, options) {
  const opts = options.map(o => `<option value="${o}">${window.esc(o)}</option>`).join('');
  return `<select id="${id}" class="mn-select mn-select--sm" style="min-width:100px">
    <option value="">${window.esc(label)}</option>
    ${opts}
  </select>`;
}

/**
 * Build the create-idea modal form HTML.
 * @returns {string}
 */
function createFormHtml() {
  const statusOpts = STATUS_OPTIONS.map(s => `<option value="${s}">${s}</option>`).join('');
  const prioOpts = PRIORITY_OPTIONS.map(p => `<option value="${p}">${p}</option>`).join('');

  return `
    <form id="idea-create-form" style="display:flex;flex-direction:column;gap:0.75rem">
      <label class="mn-label">Title
        <input type="text" name="title" class="mn-input" required>
      </label>
      <label class="mn-label">Description
        <textarea name="description" class="mn-input" rows="3"></textarea>
      </label>
      <label class="mn-label">Tags (comma-separated)
        <input type="text" name="tags" class="mn-input" placeholder="ai, dashboard">
      </label>
      <div style="display:flex;gap:0.75rem">
        <label class="mn-label" style="flex:1">Priority
          <select name="priority" class="mn-select">${prioOpts}</select>
        </label>
        <label class="mn-label" style="flex:1">Status
          <select name="status" class="mn-select">${statusOpts}</select>
        </label>
      </div>
      <div style="display:flex;justify-content:flex-end;gap:0.5rem;margin-top:0.5rem">
        <button type="button" class="mn-btn mn-btn--ghost" id="idea-cancel">Cancel</button>
        <button type="submit" class="mn-btn mn-btn--primary">Create</button>
      </div>
    </form>
  `;
}

/**
 * Ideas list view factory.
 * @param {HTMLElement} container — mount target
 * @param {{api: object, store: object}} deps
 * @returns {Function} teardown callback
 */
export default function ideas(container, { api, store }) {
  const scaffold = new StateScaffold(container, {
    state: 'loading',
    onRetry: () => refresh(),
  });

  // Toolbar: filters + create button
  const toolbar = document.createElement('div');
  toolbar.className = 'mn-toolbar';
  toolbar.style.cssText = 'display:flex;gap:0.5rem;margin-bottom:1rem;align-items:center;flex-wrap:wrap';
  toolbar.innerHTML = `
    ${filterSelect('ideas-filter-status', 'All Status', STATUS_OPTIONS)}
    ${filterSelect('ideas-filter-priority', 'All Priority', PRIORITY_OPTIONS)}
    <span style="flex:1"></span>
    <button class="mn-btn mn-btn--primary mn-btn--sm" id="ideas-create">New Idea</button>
  `;
  container.appendChild(toolbar);

  // Data table
  const table = document.createElement('mn-data-table');
  table.setAttribute('columns', TABLE_COLUMNS);
  table.setAttribute('page-size', '20');
  table.setAttribute('selectable', '');
  container.appendChild(table);

  // Filter state
  let filterStatus = '';
  let filterPriority = '';

  const statusSelect = toolbar.querySelector('#ideas-filter-status');
  const prioSelect = toolbar.querySelector('#ideas-filter-priority');

  statusSelect.onchange = () => { filterStatus = statusSelect.value; refresh(); };
  prioSelect.onchange = () => { filterPriority = prioSelect.value; refresh(); };

  // Row click opens detail in side-panel via orchestrator
  table.addEventListener('mn-row-click', (e) => {
    const idea = e.detail?.row;
    if (!idea) return;
    if (window.__convergio?.orch) {
      window.__convergio.orch.open('idea-detail', 'side-panel', idea);
    }
  });

  // Create button opens modal
  toolbar.querySelector('#ideas-create').onclick = () => openCreateModal();

  /** Open the create-idea modal. */
  function openCreateModal() {
    const modal = document.createElement('mn-modal');
    modal.setAttribute('title', 'New Idea');
    modal.innerHTML = createFormHtml();
    document.body.appendChild(modal);
    modal.open();

    const form = modal.querySelector('#idea-create-form');
    modal.querySelector('#idea-cancel').onclick = () => { modal.close(); modal.remove(); };

    form.onsubmit = async (e) => {
      e.preventDefault();
      const fd = new FormData(form);
      const payload = {
        title: fd.get('title'),
        description: fd.get('description'),
        tags: fd.get('tags'),
        priority: fd.get('priority'),
        status: fd.get('status'),
      };

      const result = await api.postIdea(payload);
      if (result instanceof Error) {
        console.warn('[ideas] postIdea failed', result);
        return;
      }

      modal.close();
      modal.remove();
      refresh();
    };
  }

  /** Fetch ideas with current filters and update table. */
  async function refresh() {
    scaffold.state = 'loading';
    const params = {};
    if (filterStatus) params.status = filterStatus;
    if (filterPriority) params.priority = filterPriority;

    const result = await api.fetchIdeas(params);

    if (result instanceof Error) {
      scaffold.state = 'error';
      console.warn('[ideas] fetchIdeas failed', result);
      return;
    }

    const rows = Array.isArray(result)
      ? result
      : (result?.ideas || []);
    table.setData(rows.map(ideaRow));
    scaffold.state = 'ready';
  }

  refresh();
  const unsub = store.subscribe('ideas', () => refresh());

  return () => {
    unsub();
    container.innerHTML = '';
  };
}
