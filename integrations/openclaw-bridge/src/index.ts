/**
 * OpenClaw plugin entry point for the Convergio bridge.
 * Registers convergio-invoke and convergio-agents tools.
 */
import { ConvergioDaemonClient } from './client.js';
import type { OpenClawConfig } from './types.js';

interface ToolDefinition {
  description: string;
  schema: Record<string, unknown>;
  execute: (args: Record<string, unknown>) => Promise<string>;
}

interface PluginApi {
  registerTool: (name: string, definition: ToolDefinition) => void;
  getConfig?: (key: string) => Record<string, unknown> | undefined;
}

/**
 * Build config from api.getConfig, env, or defaults.
 * Priority: getConfig > env > hardcoded default.
 */
function resolveConfig(api: PluginApi): OpenClawConfig {
  const pluginConfig = api.getConfig?.('convergio');
  const daemonUrl =
    (pluginConfig?.daemon_url as string | undefined) ??
    process.env.CONVERGIO_DAEMON_URL ??
    'http://localhost:8420';

  return {
    daemon_url: daemonUrl,
    default_agent: 'ali-orchestrator',
    timeout_ms: 120_000,
  };
}

/**
 * Register Convergio tools with the OpenClaw plugin API.
 * All invocations default to Ali (chief-of-staff orchestrator)
 * when no agent is specified.
 */
export function register(api: PluginApi): void {
  const config = resolveConfig(api);
  const client = new ConvergioDaemonClient(config);

  api.registerTool('convergio-invoke', {
    description: 'Invoke a Convergio agent via the daemon API',
    schema: {
      type: 'object',
      properties: {
        agent: { type: 'string', description: 'Agent name (defaults to ali-orchestrator)' },
        message: { type: 'string', description: 'Message to send to the agent' },
      },
      required: ['message'],
    },
    execute: async (args: Record<string, unknown>): Promise<string> => {
      const agentId = (args.agent as string | undefined) ?? config.default_agent;
      const message = args.message as string;
      const response = await client.invokeAgent({ agent_id: agentId, message });
      return JSON.stringify(response);
    },
  });

  api.registerTool('convergio-agents', {
    description: 'List available Convergio agents',
    schema: {
      type: 'object',
      properties: {},
    },
    execute: async (): Promise<string> => {
      const agents = await client.listAgents();
      return agents
        .map((a) => `${a.name}: ${a.description}`)
        .join('\n');
    },
  });
}
