// IPC-specific API functions for ConvergioPlatform dashboard.
// Uses apiFetch from api-core.js for consistent error handling.
// Returns parsed JSON on success, Error instance on failure.

import { apiFetch } from './api-core.js';

// --- IPC GET endpoints ---

export async function fetchIpcAgents() {
  return apiFetch('/api/ipc/agents');
}

export async function fetchIpcBudget() {
  return apiFetch('/api/ipc/budget');
}

export async function fetchIpcModels() {
  return apiFetch('/api/ipc/models');
}

export async function fetchIpcSkills() {
  return apiFetch('/api/ipc/skills');
}

export async function fetchIpcLocks() {
  return apiFetch('/api/ipc/locks');
}

export async function fetchIpcConflicts() {
  return apiFetch('/api/ipc/conflicts');
}

export async function fetchIpcStatus() {
  return apiFetch('/api/ipc/status');
}

export async function fetchIpcAuthStatus() {
  return apiFetch('/api/ipc/auth-status');
}

export async function fetchIpcRouteHistory() {
  return apiFetch('/api/ipc/route-history');
}

export async function fetchIpcMetrics() {
  return apiFetch('/api/ipc/metrics');
}

// --- Mutation helpers (POST/PUT/DELETE) ---

/**
 * Create a new idea.
 * @param {object} data - Idea payload
 */
export async function postIdea(data) {
  return apiFetch('/api/ideas', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  });
}

/**
 * Update an existing idea.
 * @param {number|string} id - Idea ID
 * @param {object} data - Fields to update
 */
export async function updateIdea(id, data) {
  return apiFetch(`/api/ideas/${encodeURIComponent(id)}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  });
}

/**
 * Delete an idea.
 * @param {number|string} id - Idea ID
 */
export async function deleteIdea(id) {
  return apiFetch(`/api/ideas/${encodeURIComponent(id)}`, {
    method: 'DELETE',
  });
}

/**
 * Send a chat message to a session.
 * @param {string} sid - Session ID
 * @param {string} message - Message text
 */
export async function postChatMessage(sid, message) {
  return apiFetch(`/api/chat/send/${encodeURIComponent(sid)}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ message }),
  });
}
