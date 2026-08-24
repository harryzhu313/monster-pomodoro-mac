import test, { beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

let storedConfig;
let invokeCalls;
let requests;
let fetchHandler;

function jsonResponse(status, body) {
  return {
    ok: status >= 200 && status < 300,
    status,
    text: async () => JSON.stringify(body),
  };
}

globalThis.window = {
  __TAURI__: {
    core: {
      invoke: async (cmd, args) => {
        invokeCalls.push({ cmd, args });
        if (cmd === 'get_notion_config') return structuredClone(storedConfig);
        if (cmd === 'set_notion_config') {
          Object.assign(storedConfig, args?.config || {});
          return null;
        }
        if (cmd === 'log_notion_export') return null;
        throw new Error(`unexpected invoke: ${cmd}`);
      },
    },
    http: {
      fetch: async (url, init = {}) => {
        requests.push({ url, init });
        return fetchHandler(url, init);
      },
    },
  },
};

const notionSource = await readFile(new URL('./notion.js', import.meta.url), 'utf8');
const notionDataUrl = `data:text/javascript;base64,${Buffer.from(notionSource).toString('base64')}`;
const notion = await import(notionDataUrl);

beforeEach(() => {
  storedConfig = {
    token: 'token-a',
    taskDbId: 'task-a',
    dayDbId: 'day-a',
    goalDbId: 'goal-a',
  };
  invokeCalls = [];
  requests = [];
  fetchHandler = async (url) => {
    const id = url.split('/').at(-1);
    return jsonResponse(200, { title: [{ plain_text: id }] });
  };
});

test('setNotionConfig keeps patch payloads partial, including an explicit empty string', async () => {
  await notion.setNotionConfig({ goalDbId: 'goal-b' });
  assert.deepEqual(invokeCalls.at(-1), {
    cmd: 'set_notion_config',
    args: { config: { goalDbId: 'goal-b' } },
  });
  assert.equal(storedConfig.taskDbId, 'task-a');
  assert.equal(storedConfig.goalDbId, 'goal-b');

  await notion.setNotionConfig({ goalDbId: '' });
  assert.equal(storedConfig.goalDbId, '');
  assert.equal(storedConfig.token, 'token-a');
});

test('daily and monthly connection checks are independent and read-only', async () => {
  fetchHandler = async (url) => {
    if (url.endsWith('/bad-goal')) {
      return jsonResponse(404, { message: 'not found' });
    }
    return jsonResponse(200, { title: [{ plain_text: url.split('/').at(-1) }] });
  };

  const daily = await notion.testConnection({
    token: 'token-a',
    taskDbId: 'task-a',
    dayDbId: 'day-a',
    goalDbId: 'bad-goal',
  });
  const monthly = await notion.testGoalConnection({
    token: 'token-a',
    taskDbId: 'task-a',
    dayDbId: 'day-a',
    goalDbId: 'bad-goal',
  });

  assert.equal(daily.ok, true);
  assert.equal(monthly.ok, false);
  assert.match(monthly.error, /检查 DB ID/);
  assert.deepEqual(
    requests.map(({ url }) => url.split('/').at(-1)),
    ['task-a', 'day-a', 'bad-goal']
  );
  for (const { url, init } of requests) {
    assert.match(url, /\/v1\/databases\//);
    assert.equal(init.method, undefined);
  }
});

test('a valid monthly database does not require the daily database fields', async () => {
  const config = { token: 'token-a', taskDbId: '', dayDbId: '', goalDbId: 'goal-a' };
  const monthly = await notion.testGoalConnection(config);
  const daily = await notion.testConnection(config);

  assert.equal(monthly.ok, true);
  assert.equal(daily.ok, false);
  assert.match(daily.error, /任务 DB ID/);
  assert.equal(requests.length, 1);
  assert.match(requests[0].url, /\/databases\/goal-a$/);
});

test('one connection check freezes the first configuration snapshot', async () => {
  fetchHandler = async (url) => {
    if (requests.length === 1) {
      storedConfig = {
        token: 'token-b',
        taskDbId: 'task-b',
        dayDbId: 'day-b',
        goalDbId: 'goal-b',
      };
    }
    return jsonResponse(200, { title: [{ plain_text: url.split('/').at(-1) }] });
  };

  const result = await notion.testConnection();
  assert.equal(result.ok, true);
  assert.equal(
    invokeCalls.filter(({ cmd }) => cmd === 'get_notion_config').length,
    1
  );
  assert.deepEqual(
    requests.map(({ url }) => url.split('/').at(-1)),
    ['task-a', 'day-a']
  );
  assert.deepEqual(
    requests.map(({ init }) => init.headers.Authorization),
    ['Bearer token-a', 'Bearer token-a']
  );
});

test('daily export ignores goalDbId and keeps its existing request flow', async () => {
  storedConfig.goalDbId = 'bad-goal';
  storedConfig.dayDbId = '';
  fetchHandler = async (url, init) => {
    if (url.endsWith('/databases/task-a/query')) {
      return jsonResponse(200, { results: [], has_more: false });
    }
    if (url.endsWith('/pages') && init.method === 'POST') {
      return jsonResponse(200, { id: 'created-task-page' });
    }
    return jsonResponse(500, { message: `unexpected request: ${url}` });
  };

  const result = await notion.exportDay('2026-08-25', [
    { title: '测试任务', planned: 1, used: 0, rotten: 0, category: '学习' },
  ]);

  assert.equal(result.ok, true);
  assert.equal(result.created, 1);
  assert.equal(result.updated, 0);
  assert.equal(requests.some(({ url }) => url.includes('bad-goal')), false);
  assert.deepEqual(
    requests.map(({ init }) => init.method),
    ['POST', 'POST']
  );
});
