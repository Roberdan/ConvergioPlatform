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

  // ── UI Builder (nasra-app-builder) ──
  { name: 'convergio_ui_analyze', description: 'Analyze a repo backend: discover API endpoints, types, auth, realtime. Returns structured API surface.', inputSchema: obj({ repo_path: str('Absolute path to repo'), probe: { type: 'boolean', description: 'Probe running endpoints (default false)' } }, ['repo_path']) },
  { name: 'convergio_ui_map', description: 'Map analyzed API surface to convergio-design components using CKB. Returns page/component mapping spec.', inputSchema: obj({ api_surface: { type: 'object', description: 'API surface from convergio_ui_analyze' }, ckb_path: str('Path to ckb.json (optional, auto-detected)') }, ['api_surface']) },
  { name: 'convergio_ui_generate', description: 'Generate/scaffold Next.js + Tauri app from component mapping. Mode: scaffold (new) or augment (existing).', inputSchema: obj({ mapping: { type: 'object', description: 'Mapping from convergio_ui_map' }, target_path: str('Target repo path'), mode: str('scaffold | augment') }, ['mapping', 'target_path', 'mode']) },
  { name: 'convergio_ui_fix', description: 'Analyze existing UI and fix DS alignment. Detects anti-patterns and replaces with proper convergio-design usage.', inputSchema: obj({ repo_path: str('Path to repo with existing UI') }, ['repo_path']) },
  { name: 'convergio_ui_validate', description: 'Validate UI against convergio-design best practices. Returns score, issues, and component coverage.', inputSchema: obj({ repo_path: str('Path to repo to validate') }, ['repo_path']) },
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

  // UI Builder — these are agent-orchestrated tools, not direct daemon calls.
  // They return structured guidance for the nasra-app-builder agent.
  convergio_ui_analyze: async (a) => {
    const { repo_path, probe } = a;
    const { execSync } = await import('node:child_process');
    const { existsSync, readFileSync } = await import('node:fs');
    const result = { endpoints: [], types: [], auth_model: 'unknown', realtime: [] };

    // Detect OpenAPI spec
    try {
      const specs = execSync(`find "${repo_path}" -maxdepth 3 \\( -name 'openapi.*' -o -name 'swagger.*' \\) 2>/dev/null`, { encoding: 'utf8' }).trim();
      if (specs) result.openapi_spec = specs.split('\n')[0];
    } catch { /* no spec found */ }

    // Detect routes by language
    const patterns = [
      { lang: 'rust', glob: '*.rs', pat: '\\.(get|post|put|delete|route)\\(' },
      { lang: 'typescript', glob: '*.ts', pat: '(router|app)\\.(get|post|put|delete)\\(' },
      { lang: 'python', glob: '*.py', pat: '@(app|router)\\.(get|post|put|delete)' },
    ];
    for (const { lang, glob, pat } of patterns) {
      try {
        const hits = execSync(`grep -rn '${pat}' "${repo_path}" --include='${glob}' 2>/dev/null | head -50`, { encoding: 'utf8' }).trim();
        if (hits) result.endpoints.push({ language: lang, routes: hits.split('\n').length, sample: hits.substring(0, 500) });
      } catch { /* no matches */ }
    }

    // Detect Next.js API routes
    try {
      const nextRoutes = execSync(`find "${repo_path}/src/app/api" -name 'route.ts' -o -name 'route.js' 2>/dev/null`, { encoding: 'utf8' }).trim();
      if (nextRoutes) result.endpoints.push({ language: 'nextjs', routes: nextRoutes.split('\n').length, sample: nextRoutes.substring(0, 500) });
    } catch { /* no next routes */ }

    // Detect type files
    try {
      const typeFiles = execSync(`find "${repo_path}" -maxdepth 4 -name '*types*.ts' -o -name '*api*.ts' 2>/dev/null | head -10`, { encoding: 'utf8' }).trim();
      if (typeFiles) result.types = typeFiles.split('\n');
    } catch { /* no type files */ }

    // Detect WebSocket/SSE
    try {
      const ws = execSync(`grep -rn 'WebSocket\\|EventSource\\|SSE\\|ws://' "${repo_path}/src" --include='*.ts' --include='*.tsx' 2>/dev/null | head -10`, { encoding: 'utf8' }).trim();
      if (ws) result.realtime = ws.split('\n').map(l => l.trim());
    } catch { /* no realtime */ }

    return result;
  },

  convergio_ui_map: async (a) => {
    const { api_surface, ckb_path } = a;
    const { readFileSync, existsSync } = await import('node:fs');
    const { execSync } = await import('node:child_process');

    // Find CKB
    let ckbFile = ckb_path;
    if (!ckbFile) {
      try {
        ckbFile = execSync('find /Users/Roberdan/GitHub/convergio-design -name ckb.json -path "*/dist/knowledge/*" 2>/dev/null | head -1', { encoding: 'utf8' }).trim();
      } catch { /* not found */ }
    }
    if (!ckbFile || !existsSync(ckbFile)) return { error: 'CKB not found. Run generate-ckb.mjs in convergio-design first.' };

    const ckb = JSON.parse(readFileSync(ckbFile, 'utf8'));
    return {
      ckb_version: ckb.version,
      package_version: ckb.packageVersion,
      available_components: ckb.webComponents.length,
      available_modules: Object.keys(ckb.tsModules).length,
      composition_rules: ckb.compositionRules.map(r => ({ id: r.id, pattern: r.pattern, components: r.components })),
      mapping_hints: ckb.mappingHints.map(h => ({ id: h.id, apiPattern: h.apiPattern, suggestedComponent: h.suggestedComponent })),
      themes: ckb.themes.map(t => t.id),
      message: 'Use composition_rules and mapping_hints to match api_surface endpoints to components. The nasra-app-builder agent has detailed protocols for this mapping.',
    };
  },

  convergio_ui_generate: async (a) => {
    return {
      status: 'delegated',
      message: 'UI generation is an agent-level operation. Use the nasra-app-builder agent with mode=' + a.mode + ' on target_path=' + a.target_path + '. The agent handles file creation, DS integration, Tauri setup, and PR workflow.',
      agent: 'nasra-app-builder',
      mode: a.mode,
      target: a.target_path,
    };
  },

  convergio_ui_fix: async (a) => {
    const { execSync } = await import('node:child_process');
    const issues = [];

    // Detect anti-patterns
    try {
      const noTokenImport = execSync(`grep -rL '@convergio/design-elements' "${a.repo_path}/src" --include='*.css' 2>/dev/null | head -5`, { encoding: 'utf8' }).trim();
      if (noTokenImport) issues.push({ type: 'missing-elements-import', files: noTokenImport.split('\n'), severity: 'high' });
    } catch { /* ok */ }

    try {
      const hardcodedColors = execSync(`grep -rn '#[0-9a-fA-F]\\{6\\}' "${a.repo_path}/src" --include='*.css' --include='*.tsx' 2>/dev/null | grep -v 'node_modules' | head -20`, { encoding: 'utf8' }).trim();
      if (hardcodedColors) issues.push({ type: 'hardcoded-colors', count: hardcodedColors.split('\n').length, severity: 'medium' });
    } catch { /* ok */ }

    try {
      const noWC = execSync(`grep -rn 'mn-' "${a.repo_path}/src" --include='*.tsx' --include='*.ts' 2>/dev/null | wc -l`, { encoding: 'utf8' }).trim();
      if (parseInt(noWC) < 5) issues.push({ type: 'no-ds-components', message: 'Very few or no convergio-design components used', severity: 'critical' });
    } catch { /* ok */ }

    return { issues, message: 'Use nasra-app-builder agent to fix these issues. Mode: fix.' };
  },

  convergio_ui_validate: async (a) => {
    const { execSync } = await import('node:child_process');
    const result = { score: 0, issues: [], components_used: [], suggestions: [] };

    // Count DS component usage
    try {
      const wcUsage = execSync(`grep -roh 'mn-[a-z-]*' "${a.repo_path}/src" --include='*.tsx' --include='*.html' 2>/dev/null | sort -u`, { encoding: 'utf8' }).trim();
      result.components_used = wcUsage ? wcUsage.split('\n') : [];
    } catch { /* ok */ }

    // Check CSS token imports
    try {
      execSync(`grep -q '@convergio/design-tokens' "${a.repo_path}/src/app/globals.css" 2>/dev/null`);
      result.score += 20;
    } catch { result.issues.push('Missing @convergio/design-tokens CSS import'); }

    try {
      execSync(`grep -q '@convergio/design-elements/css' "${a.repo_path}/src/app/globals.css" 2>/dev/null`);
      result.score += 20;
    } catch { result.issues.push('Missing @convergio/design-elements CSS import'); }

    // Score by component count
    result.score += Math.min(60, result.components_used.length * 4);

    if (result.components_used.length < 5) result.suggestions.push('Consider using more DS components. Run convergio_ui_map to see available components.');
    if (!result.components_used.includes('mn-header-shell')) result.suggestions.push('Use mn-header-shell for the app header');
    if (!result.components_used.includes('mn-data-table')) result.suggestions.push('Use mn-data-table for data lists');

    return result;
  },
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
