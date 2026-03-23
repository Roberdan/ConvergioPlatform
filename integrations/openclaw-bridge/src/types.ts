/**
 * Shared type definitions for the Convergio OpenClaw Bridge plugin.
 *
 * All interfaces describe the contract between OpenClaw and the
 * Convergio daemon API. Native fetch is used — no external deps.
 */

export interface ConvergioAgent {
  name: string;
  category: string;
  description: string;
  model: string;
  tools: string;
}

export interface InvokeRequest {
  agent_id?: string;
  message: string;
}

export interface InvokeResponse {
  ok: boolean;
  request_id: string;
  agent: string;
  status: string;
}

export interface ConvergioError {
  ok: false;
  error: string;
}

export interface OpenClawConfig {
  daemon_url: string;
  default_agent: string;
  timeout_ms: number;
}
