// Core API client for ConvergioPlatform daemon REST API.
// All endpoints use relative paths (no hardcoded host/port).
// Returns parsed JSON on success, Error instance on failure.

/**
 * Generic fetch wrapper. Returns parsed JSON or Error.
 * @param {string} path - API path (e.g. '/api/overview')
 * @param {RequestInit} [options] - fetch options
 * @returns {Promise<object|Error>}
 */
export async function apiFetch(path, options = {}) {
  try {
    const res = await fetch(path, {
      headers: { 'Accept': 'application/json', ...options.headers },
      ...options,
    });
    if (!res.ok) {
      return new Error(`${res.status} ${res.statusText}`);
    }
    // Handle 204 No Content
    if (res.status === 204) return null;
    return await res.json();
  } catch (err) {
    return new Error(err.message);
  }
}

export async function fetchOverview() {
  return apiFetch('/api/overview');
}

export async function fetchMission() {
  return apiFetch('/api/mission');
}

export async function fetchTokensDaily() {
  return apiFetch('/api/tokens/daily');
}

export async function fetchTokensModels() {
  return apiFetch('/api/tokens/models');
}

export async function fetchMesh() {
  return apiFetch('/api/mesh');
}

export async function fetchMeshMetrics() {
  return apiFetch('/api/mesh/metrics');
}

export async function fetchMeshSyncStats() {
  return apiFetch('/api/mesh/sync-stats');
}

export async function fetchTasksDistribution() {
  return apiFetch('/api/tasks/distribution');
}

export async function fetchTasksBlocked() {
  return apiFetch('/api/tasks/blocked');
}

export async function fetchPlanList() {
  return apiFetch('/api/plan-db/list');
}

export async function fetchPlanTree(id) {
  return apiFetch(`/api/plan-db/execution-tree/${encodeURIComponent(id)}`);
}

/**
 * Fetch ideas with optional filters.
 * @param {object} [params] - Query params: status, priority, project_id, tag
 */
export async function fetchIdeas(params) {
  let path = '/api/ideas';
  if (params && typeof params === 'object') {
    const qs = new URLSearchParams();
    for (const [key, value] of Object.entries(params)) {
      if (value !== undefined && value !== null) {
        qs.set(key, String(value));
      }
    }
    const str = qs.toString();
    if (str) path += `?${str}`;
  }
  return apiFetch(path);
}

/**
 * Fetch paginated nightly jobs.
 * @param {number} [page]
 * @param {number} [perPage]
 */
export async function fetchNightlyJobs(page, perPage) {
  const qs = new URLSearchParams();
  if (page !== undefined) qs.set('page', String(page));
  if (perPage !== undefined) qs.set('per_page', String(perPage));
  const str = qs.toString();
  return apiFetch(`/api/nightly/jobs${str ? `?${str}` : ''}`);
}

export async function fetchNightlyJobDetail(id) {
  return apiFetch(`/api/nightly/jobs/${encodeURIComponent(id)}`);
}

export async function fetchHistory() {
  return apiFetch('/api/history');
}

export async function fetchEvents() {
  return apiFetch('/api/events');
}

export async function fetchCoordinatorStatus() {
  return apiFetch('/api/coordinator/status');
}
