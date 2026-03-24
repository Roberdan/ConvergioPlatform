#!/usr/bin/env node
// Convergio MCP Server — full daemon API as MCP tools.
// Every daemon capability exposed: plans, mesh, agents, IPC, workspace, etc.

import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';

const API = process.env.CONVERGIO_API_URL || 'http://localhost:8420';

async function get(path) {
  const r = await fetch(`${API}${path}`);
  return r.ok ? r.json() : { error: r.statusText, status: r.status };
}

async function post(path, body = {}) {
  const r = await fetch(`${API}${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  return r.ok ? r.json() : { error: r.statusText, status: r.status };
}

// -- Tool definitions grouped by domain --

const str = (d) => ({ type: 'string', description: d });
const num = (d) => ({ type: 'number', description: d });
const obj = (props = {}, required = []) => ({
  type: 'object', properties: props, ...(required.length ? { required } : {}),
});

const TOOLS = [
  // ── Platform ──
  { name: 'convergio_health', description: 'Daemon health, uptime, DB, peers', inputSchema: obj() },
  { name: 'convergio_overview', description: 'Dashboard overview: active plans, agents, tokens, cost, mesh', inputSchema: obj() },
  { name: 'convergio_projects', description: 'List all projects', inputSchema: obj() },
  { name: 'convergio_project_tree', description: 'Project tree (plans, waves, tasks)', inputSchema: obj({ id: str('Project ID') }, ['id']) },
  { name: 'convergio_notifications', description: 'Pending notifications', inputSchema: obj() },

  // ── Plans ──
  { name: 'convergio_plans', description: 'All plans with status, task counts', inputSchema: obj() },
  { name: 'convergio_plan_detail', description: 'Full plan: waves, tasks, progress', inputSchema: obj({ plan_id: num('Plan ID') }, ['plan_id']) },
  { name: 'convergio_plan_tree', description: 'Plan execution tree', inputSchema: obj({ plan_id: num('Plan ID') }, ['plan_id']) },
  { name: 'convergio_plan_drift', description: 'Plan drift check', inputSchema: obj({ plan_id: num('Plan ID') }, ['plan_id']) },
  { name: 'convergio_plan_readiness', description: 'Plan readiness for merge', inputSchema: obj({ plan_id: num('Plan ID') }, ['plan_id']) },
  { name: 'convergio_plan_create', description: 'Create a new plan', inputSchema: obj({ project: str('Project'), name: str('Plan name') }, ['project', 'name']) },
  { name: 'convergio_plan_start', description: 'Start a plan', inputSchema: obj({ plan_id: num('Plan ID') }, ['plan_id']) },
  { name: 'convergio_plan_complete', description: 'Complete a plan', inputSchema: obj({ plan_id: num('Plan ID') }, ['plan_id']) },
  { name: 'convergio_plan_cancel', description: 'Cancel a plan', inputSchema: obj({ plan_id: num('Plan ID'), reason: str('Reason') }, ['plan_id', 'reason']) },
  { name: 'convergio_plan_import', description: 'Import plan from YAML spec', inputSchema: obj({ plan_id: num('Plan ID'), spec: str('YAML content') }, ['plan_id', 'spec']) },
  { name: 'convergio_plan_validate', description: 'Validate plan (Thor)', inputSchema: obj({ plan_id: num('Plan ID') }, ['plan_id']) },

  // ── Tasks ──
  { name: 'convergio_task_update', description: 'Update task status', inputSchema: obj({ task_id: str('Task ID'), status: str('pending|in_progress|done|blocked'), summary: str('Summary') }, ['task_id', 'status']) },
  { name: 'convergio_tasks_blocked', description: 'List blocked tasks', inputSchema: obj() },
  { name: 'convergio_tasks_distribution', description: 'Task distribution by agent', inputSchema: obj() },

  // ── Waves ──
  { name: 'convergio_wave_create', description: 'Create a wave', inputSchema: obj({ plan_id: num('Plan ID'), wave_number: num('Wave number') }, ['plan_id']) },
  { name: 'convergio_wave_update', description: 'Update wave status', inputSchema: obj({ wave_id: num('Wave ID'), status: str('Status') }, ['wave_id', 'status']) },

  // ── Agents ──
  { name: 'convergio_agents', description: 'Running and recent agents', inputSchema: obj() },
  { name: 'convergio_agent_catalog', description: 'Full agent catalog with capabilities', inputSchema: obj() },
  { name: 'convergio_agent_start', description: 'Register agent start', inputSchema: obj({ agent_id: str('Agent ID'), type: str('Type'), host: str('Host') }, ['agent_id']) },
  { name: 'convergio_agent_complete', description: 'Register agent completion', inputSchema: obj({ agent_id: str('Agent ID') }, ['agent_id']) },
  { name: 'convergio_agent_stop', description: 'Stop a running agent', inputSchema: obj({ name: str('Agent name') }, ['name']) },

  // ── Mesh ──
  { name: 'convergio_mesh', description: 'Mesh peers: CPU, memory, status', inputSchema: obj() },
  { name: 'convergio_mesh_topology', description: 'Mesh network topology', inputSchema: obj() },
  { name: 'convergio_mesh_exec', description: 'Execute command on mesh peer', inputSchema: obj({ peer: str('Peer name'), command: str('Command') }, ['peer', 'command']) },
  { name: 'convergio_mesh_delegate', description: 'Delegate task to mesh peer', inputSchema: obj({ peer: str('Peer'), task: str('Task description') }, ['peer', 'task']) },
  { name: 'convergio_mesh_provision', description: 'Provision mesh node', inputSchema: obj() },
  { name: 'convergio_mesh_ping', description: 'Ping a mesh peer', inputSchema: obj({ peer: str('Peer name') }, ['peer']) },
  { name: 'convergio_mesh_diagnostics', description: 'Mesh diagnostics', inputSchema: obj() },
  { name: 'convergio_mesh_sync', description: 'Force CRDT sync', inputSchema: obj() },

  // ── IPC ──
  { name: 'convergio_ipc_agents', description: 'IPC registered agents', inputSchema: obj() },
  { name: 'convergio_ipc_send', description: 'Send IPC message', inputSchema: obj({ target: str('Target agent'), type: str('Message type'), message: str('Content') }, ['target', 'message']) },
  { name: 'convergio_ipc_locks', description: 'Active file locks', inputSchema: obj() },
  { name: 'convergio_ipc_status', description: 'IPC system status', inputSchema: obj() },
  { name: 'convergio_ipc_budget', description: 'Token budget status', inputSchema: obj() },
  { name: 'convergio_ipc_skills', description: 'Registered skills', inputSchema: obj() },
  { name: 'convergio_ipc_worktrees', description: 'Active worktrees', inputSchema: obj() },

  // ── Workspace ──
  { name: 'convergio_workspaces', description: 'Active workspaces', inputSchema: obj() },
  { name: 'convergio_workspace_create', description: 'Create workspace', inputSchema: obj({ plan_id: num('Plan ID'), branch: str('Branch name') }, ['branch']) },
  { name: 'convergio_workspace_events', description: 'Workspace events', inputSchema: obj({ workspace_id: str('Workspace ID'), limit: num('Limit') }) },
  { name: 'convergio_workspace_quality', description: 'Run quality gates', inputSchema: obj({ workspace_id: str('Workspace ID') }, ['workspace_id']) },

  // ── Metrics & Cost ──
  { name: 'convergio_metrics', description: 'Run count, avg duration, total cost', inputSchema: obj() },
  { name: 'convergio_cost', description: 'Cost by model/project/date', inputSchema: obj({ days: num('Days back (default 7)') }) },
  { name: 'convergio_runs', description: 'Execution runs list', inputSchema: obj() },
  { name: 'convergio_run_detail', description: 'Run detail', inputSchema: obj({ id: str('Run ID') }, ['id']) },

  // ── Knowledge Base ──
  { name: 'convergio_kb_search', description: 'Search knowledge base', inputSchema: obj({ q: str('Query'), limit: num('Max results') }, ['q']) },
  { name: 'convergio_kb_write', description: 'Write to knowledge base', inputSchema: obj({ key: str('Key'), value: str('Value') }, ['key', 'value']) },

  // ── Workers & Coordinator ──
  { name: 'convergio_workers', description: 'Worker status', inputSchema: obj() },
  { name: 'convergio_worker_launch', description: 'Launch a worker', inputSchema: obj({ task: str('Task'), peer: str('Peer') }, ['task']) },
  { name: 'convergio_coordinator_status', description: 'Coordinator status', inputSchema: obj() },

  // ── Ideas ──
  { name: 'convergio_ideas', description: 'List ideas', inputSchema: obj() },
  { name: 'convergio_idea_create', description: 'Create an idea', inputSchema: obj({ title: str('Title'), description: str('Description') }, ['title']) },
  { name: 'convergio_idea_promote', description: 'Promote idea to plan', inputSchema: obj({ id: num('Idea ID') }, ['id']) },

  // ── Checkpoints ──
  { name: 'convergio_checkpoint_save', description: 'Save plan checkpoint', inputSchema: obj({ plan_id: num('Plan ID') }, ['plan_id']) },
  { name: 'convergio_checkpoint_restore', description: 'Restore plan checkpoint', inputSchema: obj({ plan_id: num('Plan ID') }, ['plan_id']) },
];

// -- Route handlers --

const H = {
  // Platform
  convergio_health: () => get('/api/health'),
  convergio_overview: () => get('/api/overview'),
  convergio_projects: () => get('/api/projects'),
  convergio_project_tree: (a) => get(`/api/project/${a.id}/tree`),
  convergio_notifications: () => get('/api/notifications'),
  // Plans
  convergio_plans: () => get('/api/plan-db/list'),
  convergio_plan_detail: (a) => get(`/api/plan-db/json/${a.plan_id}`),
  convergio_plan_tree: (a) => get(`/api/plan-db/execution-tree/${a.plan_id}`),
  convergio_plan_drift: (a) => get(`/api/plan-db/drift-check/${a.plan_id}`),
  convergio_plan_readiness: (a) => get(`/api/plan-db/readiness/${a.plan_id}`),
  convergio_plan_create: (a) => post('/api/plan-db/create', a),
  convergio_plan_start: (a) => post(`/api/plan-db/start/${a.plan_id}`, {}),
  convergio_plan_complete: (a) => post(`/api/plan-db/complete/${a.plan_id}`, {}),
  convergio_plan_cancel: (a) => post(`/api/plan-db/cancel/${a.plan_id}`, { reason: a.reason }),
  convergio_plan_import: (a) => post('/api/plan-db/import', { plan_id: a.plan_id, spec: a.spec }),
  convergio_plan_validate: (a) => post(`/api/plans/${a.plan_id}/validate`, {}),
  // Tasks
  convergio_task_update: (a) => post('/api/plan-db/task/update', a),
  convergio_tasks_blocked: () => get('/api/tasks/blocked'),
  convergio_tasks_distribution: () => get('/api/tasks/distribution'),
  // Waves
  convergio_wave_create: (a) => post('/api/plan-db/wave/create', a),
  convergio_wave_update: (a) => post('/api/plan-db/wave/update', a),
  // Agents
  convergio_agents: () => get('/api/agents'),
  convergio_agent_catalog: () => get('/api/agents/catalog'),
  convergio_agent_start: (a) => post('/api/plan-db/agent/start', a),
  convergio_agent_complete: (a) => post('/api/plan-db/agent/complete', a),
  convergio_agent_stop: (a) => post('/api/ipc/agents/unregister', a),
  // Mesh
  convergio_mesh: () => get('/api/mesh'),
  convergio_mesh_topology: () => get('/api/mesh/topology'),
  convergio_mesh_exec: (a) => post('/api/mesh/exec', a),
  convergio_mesh_delegate: (a) => post('/api/mesh/delegate', a),
  convergio_mesh_provision: () => get('/api/mesh/provision'),
  convergio_mesh_ping: (a) => get(`/api/mesh/ping/${a.peer}`),
  convergio_mesh_diagnostics: () => get('/api/mesh/diagnostics'),
  convergio_mesh_sync: () => post('/api/crdt/force-sync', {}),
  // IPC
  convergio_ipc_agents: () => get('/api/ipc/agents'),
  convergio_ipc_send: (a) => post('/api/ipc/send', a),
  convergio_ipc_locks: () => get('/api/ipc/locks'),
  convergio_ipc_status: () => get('/api/ipc/status'),
  convergio_ipc_budget: () => get('/api/ipc/budget'),
  convergio_ipc_skills: () => get('/api/ipc/skills'),
  convergio_ipc_worktrees: () => get('/api/ipc/worktrees'),
  // Workspace
  convergio_workspaces: () => get('/api/workspace/list'),
  convergio_workspace_create: (a) => post('/api/workspace/create', a),
  convergio_workspace_events: (a) => get(`/api/workspace/events?workspace_id=${a.workspace_id || ''}&limit=${a.limit || 20}`),
  convergio_workspace_quality: (a) => post('/api/workspace/quality-gate', a),
  // Metrics
  convergio_metrics: () => get('/api/metrics/summary'),
  convergio_cost: (a) => get(`/api/metrics/cost?days=${a.days || 7}`),
  convergio_runs: () => get('/api/runs'),
  convergio_run_detail: (a) => get(`/api/runs/${a.id}`),
  // KB
  convergio_kb_search: (a) => get(`/api/plan-db/kb-search?q=${encodeURIComponent(a.q)}&limit=${a.limit || 10}`),
  convergio_kb_write: (a) => post('/api/plan-db/kb-write', a),
  // Workers
  convergio_workers: () => get('/api/workers'),
  convergio_worker_launch: (a) => post('/api/workers/launch', a),
  convergio_coordinator_status: () => get('/api/coordinator/status'),
  // Ideas
  convergio_ideas: () => get('/api/ideas'),
  convergio_idea_create: (a) => post('/api/ideas', a),
  convergio_idea_promote: (a) => post(`/api/ideas/${a.id}/promote`, {}),
  // Checkpoints
  convergio_checkpoint_save: (a) => post('/api/plan-db/checkpoint/save', a),
  convergio_checkpoint_restore: (a) => get(`/api/plan-db/checkpoint/restore?plan_id=${a.plan_id}`),
};

// -- MCP Server --

const server = new Server(
  { name: 'convergio', version: '2.0.0' },
  { capabilities: { tools: {} } },
);

server.setRequestHandler({ method: 'tools/list' }, async () => ({ tools: TOOLS }));

server.setRequestHandler({ method: 'tools/call' }, async (request) => {
  const { name, arguments: args } = request.params;
  const handler = H[name];
  if (!handler) {
    return { content: [{ type: 'text', text: `Unknown tool: ${name}` }], isError: true };
  }
  try {
    const result = await handler(args || {});
    return { content: [{ type: 'text', text: JSON.stringify(result, null, 2) }] };
  } catch (err) {
    return { content: [{ type: 'text', text: `Error: ${err.message}` }], isError: true };
  }
});

const transport = new StdioServerTransport();
await server.connect(transport);
