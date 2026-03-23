/**
 * Tests for the plugin entry point (register function).
 */
import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import { register } from './index.js';

interface ToolDef {
  description: string;
  schema: Record<string, unknown>;
  execute: (args: Record<string, unknown>) => Promise<string>;
}

function createSpy(): {
  registered: Record<string, ToolDef>;
  api: { registerTool: (name: string, def: ToolDef) => void };
} {
  const registered: Record<string, ToolDef> = {};
  return {
    registered,
    api: {
      registerTool: (name: string, def: ToolDef) => {
        registered[name] = def;
      },
    },
  };
}

describe('register', () => {
  let originalFetch: typeof globalThis.fetch;

  beforeEach(() => {
    originalFetch = globalThis.fetch;
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it('should register convergio-invoke and convergio-agents tools', () => {
    const { registered, api } = createSpy();
    register(api);

    assert.ok(registered['convergio-invoke'], 'convergio-invoke not registered');
    assert.ok(registered['convergio-agents'], 'convergio-agents not registered');
  });

  it('should use daemon_url from getConfig if available', () => {
    const { registered, api } = createSpy();
    const apiWithConfig = {
      ...api,
      getConfig: (key: string) => {
        if (key === 'convergio') {
          return { daemon_url: 'http://custom:1234' };
        }
        return undefined;
      },
    };

    register(apiWithConfig);
    assert.ok(registered['convergio-invoke']);
  });

  it('convergio-invoke execute should call invokeAgent and return string', async () => {
    const invokeResponse = {
      ok: true,
      request_id: 'req-002',
      agent: 'ali-orchestrator',
      status: 'completed',
    };

    globalThis.fetch = async () => {
      return new Response(JSON.stringify(invokeResponse), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      });
    };

    const { registered, api } = createSpy();
    register(api);

    const result = await registered['convergio-invoke'].execute({ message: 'hello' });
    assert.equal(typeof result, 'string');
    assert.ok(result.length > 0);
  });

  it('convergio-invoke should default to ali-orchestrator when no agent specified', async () => {
    let capturedBody = '';
    globalThis.fetch = async (_input: string | URL | Request, init?: RequestInit) => {
      capturedBody = init?.body as string;
      return new Response(JSON.stringify({
        ok: true, request_id: 'req-003', agent: 'ali-orchestrator', status: 'completed',
      }), { status: 200 });
    };

    const { registered, api } = createSpy();
    register(api);
    await registered['convergio-invoke'].execute({ message: 'test' });

    const body = JSON.parse(capturedBody);
    assert.equal(body.agent_id, 'ali-orchestrator');
  });

  it('convergio-agents execute should return formatted agent list', async () => {
    const agents = [
      { name: 'ali-orchestrator', category: 'leadership', description: 'Chief of staff', model: 'claude-sonnet-4.6', tools: 'all' },
    ];

    globalThis.fetch = async () => {
      return new Response(JSON.stringify({ agents }), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      });
    };

    const { registered, api } = createSpy();
    register(api);

    const result = await registered['convergio-agents'].execute({});
    assert.ok(result.includes('ali-orchestrator'));
    assert.ok(result.includes('Chief of staff'));
  });
});
