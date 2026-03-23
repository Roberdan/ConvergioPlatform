/**
 * HTTP client for the Convergio daemon API.
 * Uses native fetch — zero external dependencies.
 */
import type {
  OpenClawConfig,
  ConvergioAgent,
  InvokeRequest,
  InvokeResponse,
} from './types.js';

export class ConvergioDaemonClient {
  private readonly baseUrl: string;
  private readonly timeoutMs: number;

  constructor(config: OpenClawConfig) {
    // Strip trailing slash to avoid double-slash in URLs
    this.baseUrl = config.daemon_url.replace(/\/+$/, '');
    this.timeoutMs = config.timeout_ms;
  }

  /**
   * Fetch agent list from the daemon.
   * GET /api/openclaw/agents — returns parsed ConvergioAgent array.
   */
  async listAgents(): Promise<ConvergioAgent[]> {
    const url = `${this.baseUrl}/api/openclaw/agents`;
    const res = await fetch(url, {
      method: 'GET',
      headers: { 'Accept': 'application/json' },
      signal: AbortSignal.timeout(this.timeoutMs),
    });

    if (!res.ok) {
      const body = await res.text();
      throw new Error(
        `Convergio daemon error: ${res.status} ${res.statusText} — ${body}`
      );
    }

    const data: { agents: ConvergioAgent[] } = await res.json();
    return data.agents;
  }

  /**
   * Invoke an agent on the daemon.
   * POST /api/openclaw/invoke — sends JSON body, returns typed response.
   */
  async invokeAgent(req: InvokeRequest): Promise<InvokeResponse> {
    const url = `${this.baseUrl}/api/openclaw/invoke`;
    const res = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Accept': 'application/json',
      },
      body: JSON.stringify(req),
      signal: AbortSignal.timeout(this.timeoutMs),
    });

    if (!res.ok) {
      const body = await res.text();
      throw new Error(
        `Convergio daemon error: ${res.status} ${res.statusText} — ${body}`
      );
    }

    const data: InvokeResponse = await res.json();
    return data;
  }
}
