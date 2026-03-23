/**
 * Tests for ConvergioDaemonClient.
 * Uses native fetch mock — no external test deps.
 */
import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import { ConvergioDaemonClient } from './client.js';
import type { OpenClawConfig, ConvergioAgent, InvokeResponse } from './types.js';

const TEST_CONFIG: OpenClawConfig = {
  daemon_url: 'http://localhost:9999',
  default_agent: 'ali-orchestrator',
  timeout_ms: 5000,
};

describe('ConvergioDaemonClient', () => {
  let originalFetch: typeof globalThis.fetch;

  beforeEach(() => {
    originalFetch = globalThis.fetch;
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  describe('listAgents', () => {
    it('should GET /api/openclaw/agents and return typed array', async () => {
      const agents: ConvergioAgent[] = [
        { name: 'ali-orchestrator', category: 'leadership', description: 'Chief of staff', model: 'claude-sonnet-4.6', tools: 'all' },
        { name: 'baccio', category: 'technical', description: 'Architect', model: 'claude-opus-4.6', tools: 'code' },
      ];
      let capturedUrl = '';
      let capturedInit: RequestInit | undefined;

      globalThis.fetch = async (input: string | URL | Request, init?: RequestInit) => {
        capturedUrl = String(input);
        capturedInit = init;
        return new Response(JSON.stringify({ agents }), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        });
      };

      const client = new ConvergioDaemonClient(TEST_CONFIG);
      const result = await client.listAgents();

      assert.equal(capturedUrl, 'http://localhost:9999/api/openclaw/agents');
      assert.equal(capturedInit?.method, 'GET');
      assert.deepEqual(result, agents);
    });

    it('should throw on non-2xx response', async () => {
      globalThis.fetch = async () => {
        return new Response('{"error":"forbidden"}', { status: 403 });
      };

      const client = new ConvergioDaemonClient(TEST_CONFIG);
      await assert.rejects(
        () => client.listAgents(),
        (err: Error) => {
          assert.match(err.message, /403/);
          return true;
        }
      );
    });
  });

  describe('invokeAgent', () => {
    it('should POST /api/openclaw/invoke with JSON body', async () => {
      const response: InvokeResponse = {
        ok: true,
        request_id: 'req-001',
        agent: 'ali-orchestrator',
        status: 'completed',
      };
      let capturedUrl = '';
      let capturedInit: RequestInit | undefined;

      globalThis.fetch = async (input: string | URL | Request, init?: RequestInit) => {
        capturedUrl = String(input);
        capturedInit = init;
        return new Response(JSON.stringify(response), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        });
      };

      const client = new ConvergioDaemonClient(TEST_CONFIG);
      const result = await client.invokeAgent({
        agent_id: 'ali-orchestrator',
        message: 'list all plans',
      });

      assert.equal(capturedUrl, 'http://localhost:9999/api/openclaw/invoke');
      assert.equal(capturedInit?.method, 'POST');
      const body = JSON.parse(capturedInit?.body as string);
      assert.equal(body.agent_id, 'ali-orchestrator');
      assert.equal(body.message, 'list all plans');
      assert.deepEqual(result, response);
    });

    it('should throw with status and body on non-2xx', async () => {
      globalThis.fetch = async () => {
        return new Response('{"error":"agent not found"}', { status: 404 });
      };

      const client = new ConvergioDaemonClient(TEST_CONFIG);
      await assert.rejects(
        () => client.invokeAgent({ message: 'hello' }),
        (err: Error) => {
          assert.match(err.message, /404/);
          assert.match(err.message, /agent not found/);
          return true;
        }
      );
    });
  });
});
