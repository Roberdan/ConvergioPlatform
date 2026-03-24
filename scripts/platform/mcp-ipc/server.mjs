#!/usr/bin/env node
// Convergio MCP Server — exposes daemon API as MCP tools.
// Claude calls these directly instead of Bash(curl).

import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';

const API = process.env.CONVERGIO_API_URL || 'http://localhost:8420';

async function api(path) {
  const res = await fetch(`${API}${path}`);
  if (!res.ok) return { error: res.statusText };
  return res.json();
}

async function apiPost(path, body = {}) {
  const res = await fetch(`${API}${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  if (!res.ok) return { error: res.statusText };
  return res.json();
}

const TOOLS = [
  {
    name: 'convergio_health',
    description: 'Daemon health: uptime, DB status, peer count',
    inputSchema: { type: 'object', properties: {} },
  },
  {
    name: 'convergio_plans',
    description: 'List all plans with status, task counts',
    inputSchema: { type: 'object', properties: {} },
  },
  {
    name: 'convergio_plan_detail',
    description: 'Full plan detail with waves and tasks',
    inputSchema: {
      type: 'object',
      properties: { plan_id: { type: 'number', description: 'Plan ID' } },
      required: ['plan_id'],
    },
  },
  {
    name: 'convergio_agents',
    description: 'Running and recent agents',
    inputSchema: { type: 'object', properties: {} },
  },
  {
    name: 'convergio_mesh',
    description: 'Mesh peers with CPU, memory, online status',
    inputSchema: { type: 'object', properties: {} },
  },
  {
    name: 'convergio_cost',
    description: 'Cost breakdown by model, project, date',
    inputSchema: {
      type: 'object',
      properties: { days: { type: 'number', description: 'Days back (default 7)' } },
    },
  },
  {
    name: 'convergio_events',
    description: 'Recent workspace events (file ops, git, quality gates)',
    inputSchema: {
      type: 'object',
      properties: { limit: { type: 'number', description: 'Max events (default 20)' } },
    },
  },
  {
    name: 'convergio_workspaces',
    description: 'Active workspaces with branch, plan, status',
    inputSchema: { type: 'object', properties: {} },
  },
  {
    name: 'convergio_create_plan',
    description: 'Create a new plan',
    inputSchema: {
      type: 'object',
      properties: {
        project: { type: 'string', description: 'Project name' },
        name: { type: 'string', description: 'Plan name' },
      },
      required: ['project', 'name'],
    },
  },
  {
    name: 'convergio_update_task',
    description: 'Update task status',
    inputSchema: {
      type: 'object',
      properties: {
        task_id: { type: 'string', description: 'Task ID' },
        status: { type: 'string', description: 'New status: pending|in_progress|done|blocked' },
        summary: { type: 'string', description: 'Optional summary' },
      },
      required: ['task_id', 'status'],
    },
  },
  {
    name: 'convergio_mesh_exec',
    description: 'Execute command on a mesh peer',
    inputSchema: {
      type: 'object',
      properties: {
        peer: { type: 'string', description: 'Peer name' },
        command: { type: 'string', description: 'Command to execute' },
      },
      required: ['peer', 'command'],
    },
  },
  {
    name: 'convergio_stop_agent',
    description: 'Stop a running agent',
    inputSchema: {
      type: 'object',
      properties: { name: { type: 'string', description: 'Agent name' } },
      required: ['name'],
    },
  },
];

const HANDLERS = {
  convergio_health: () => api('/api/health'),
  convergio_plans: () => api('/api/plan-db/list'),
  convergio_plan_detail: (args) => api(`/api/plan-db/json/${args.plan_id}`),
  convergio_agents: () => api('/api/agents'),
  convergio_mesh: () => api('/api/mesh'),
  convergio_cost: (args) => api(`/api/metrics/cost?days=${args.days || 7}`),
  convergio_events: (args) => api(`/api/workspace/events?limit=${args.limit || 20}`),
  convergio_workspaces: () => api('/api/workspace/list'),
  convergio_create_plan: (args) =>
    apiPost('/api/plan-db/create', { project: args.project, name: args.name }),
  convergio_update_task: (args) =>
    apiPost('/api/plan-db/task/update', {
      task_id: args.task_id, status: args.status, summary: args.summary || '',
    }),
  convergio_mesh_exec: (args) =>
    apiPost('/api/mesh/exec', { peer: args.peer, command: args.command }),
  convergio_stop_agent: (args) =>
    apiPost('/api/ipc/agents/unregister', { name: args.name }),
};

const server = new Server(
  { name: 'convergio', version: '1.0.0' },
  { capabilities: { tools: {} } },
);

server.setRequestHandler({ method: 'tools/list' }, async () => ({ tools: TOOLS }));

server.setRequestHandler({ method: 'tools/call' }, async (request) => {
  const { name, arguments: args } = request.params;
  const handler = HANDLERS[name];
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
