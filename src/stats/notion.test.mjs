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

const TODAY = '2026-08-25';
const IDS = Object.freeze({
  goalDb: '11111111-1111-1111-1111-111111111111',
  goalSource: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
  timeframeDb: '22222222-2222-2222-2222-222222222222',
  timeframeSource: 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb',
});

function schemaProperty(id, name, type, config = {}) {
  return { id, name, type, [type]: config };
}

function goalDatabase() {
  return {
    object: 'database',
    id: IDS.goalDb,
    title: [{ plain_text: '月度番茄目标' }],
    properties: {
      名称: schemaProperty('title', '名称', 'title'),
      分类: schemaProperty('cat', '分类', 'select', {
        options: [
          { id: 'hobby', name: '兴趣爱好' },
          { id: 'study', name: '学习' },
          { id: 'work', name: '工作' },
          { id: 'life', name: '生活' },
        ],
      }),
      本月总量: schemaProperty('target', '本月总量', 'number', { format: 'number' }),
      已完成: schemaProperty('done', '已完成', 'number', { format: 'number' }),
      所属月: schemaProperty('month-rel', '所属月', 'relation', {
        database_id: IDS.timeframeDb,
        data_source_id: IDS.timeframeSource,
        // 当前官方升级示例会省略 inner type；dual_property 本身仍足以证明双向关系。
        dual_property: {
          synced_property_id: 'reverse-rel',
          synced_property_name: '月度番茄目标',
        },
      }),
      '本地目标 ID': schemaProperty('local-id', '本地目标 ID', 'rich_text'),
      额外分析: schemaProperty('extra', '额外分析', 'formula', { expression: '1' }),
    },
  };
}

function timeframeDatabase() {
  return {
    object: 'database',
    id: IDS.timeframeDb,
    title: [{ plain_text: '时间框架' }],
    properties: {
      名称: schemaProperty('title', '名称', 'title'),
      类型: schemaProperty('kind', '类型', 'select', {
        options: [
          { id: 'day', name: '日' },
          { id: 'week', name: '周' },
          { id: 'month', name: '月' },
          { id: 'quarter', name: '季度' },
          { id: 'year', name: '年' },
        ],
      }),
      日期: schemaProperty('date', '日期', 'date'),
      月度番茄目标: schemaProperty('reverse-rel', '月度番茄目标', 'relation', {
        database_id: IDS.goalDb,
        data_source_id: IDS.goalSource,
        type: 'dual_property',
        dual_property: {
          synced_property_id: 'month-rel',
          synced_property_name: '所属月',
        },
      }),
    },
  };
}

function monthPage(id, start, end, kind = '月') {
  return {
    object: 'page',
    id,
    properties: {
      类型: { id: 'kind', type: 'select', select: { id: 'month', name: kind } },
      日期: { id: 'date', type: 'date', date: { start, end, time_zone: null } },
    },
  };
}

function pageList(results, hasMore = false, nextCursor = null) {
  return { object: 'list', type: 'page', results, has_more: hasMore, next_cursor: nextCursor };
}

function installPreflightHandler({
  goal = goalDatabase(),
  timeframe = timeframeDatabase(),
  query = () => pageList([monthPage('aug-page', '2026-08-01', '2026-08-31')]),
} = {}) {
  fetchHandler = async (url, init = {}) => {
    const path = new URL(url).pathname;
    if (path === `/v1/databases/${IDS.goalDb}`) return jsonResponse(200, goal);
    if (path === `/v1/databases/${IDS.timeframeDb}`) return jsonResponse(200, timeframe);
    if (path === `/v1/databases/${IDS.timeframeDb}/query`) {
      return jsonResponse(200, query(JSON.parse(init.body || '{}')));
    }
    const id = path.split('/').at(-1);
    return jsonResponse(200, { title: [{ plain_text: id }] });
  };
}

function assertPreflightReadOnly(allowedInvokeCommands = []) {
  for (const { url, init = {} } of requests) {
    const path = new URL(url).pathname;
    const method = init.method || 'GET';
    const allowed = (method === 'GET' && /^\/v1\/databases\/[^/]+$/.test(path))
      || (method === 'POST' && /^\/v1\/databases\/[^/]+\/query$/.test(path));
    assert.equal(allowed, true, `${method} ${path} must remain a read-only preflight request`);
  }
  assert.deepEqual(invokeCalls.map(({ cmd }) => cmd), allowedInvokeCommands);
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
    goalDbId: IDS.goalDb,
  };
  invokeCalls = [];
  requests = [];
  installPreflightHandler();
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
  }, TODAY);

  assert.equal(daily.ok, true);
  assert.equal(monthly.ok, false);
  assert.match(monthly.error, /检查月度目标 DB ID/);
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
  const config = { token: 'token-a', taskDbId: '', dayDbId: '', goalDbId: IDS.goalDb };
  const monthly = await notion.testGoalConnection(config, TODAY);
  const daily = await notion.testConnection(config);

  assert.equal(monthly.ok, true);
  assert.equal(daily.ok, false);
  assert.match(daily.error, /任务 DB ID/);
  assert.equal(requests.length, 3);
  assert.match(requests[0].url, new RegExp(`/databases/${IDS.goalDb}$`));
  assertPreflightReadOnly();
});

test('one connection check freezes the first configuration snapshot', async () => {
  fetchHandler = async (url) => {
    if (requests.length === 1) {
      storedConfig = {
        token: 'token-b',
        taskDbId: 'task-b',
        dayDbId: 'day-b',
        goalDbId: IDS.goalDb,
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

test('goal preflight reads and freezes its configuration exactly once', async () => {
  const baseHandler = fetchHandler;
  fetchHandler = async (url, init) => {
    if (requests.length === 1) {
      storedConfig = {
        token: 'token-b',
        taskDbId: 'task-b',
        dayDbId: 'day-b',
        goalDbId: '33333333-3333-3333-3333-333333333333',
      };
    }
    return baseHandler(url, init);
  };

  const result = await notion.testGoalConnection(null, TODAY);
  assert.equal(result.ok, true);
  assert.deepEqual(invokeCalls.map(({ cmd }) => cmd), ['get_notion_config']);
  assert.deepEqual(
    requests.map(({ init }) => init.headers.Authorization),
    ['Bearer token-a', 'Bearer token-a', 'Bearer token-a']
  );
  assert.deepEqual(
    requests.map(({ url }) => new URL(url).pathname),
    [
      `/v1/databases/${IDS.goalDb}`,
      `/v1/databases/${IDS.timeframeDb}`,
      `/v1/databases/${IDS.timeframeDb}/query`,
    ]
  );
  assertPreflightReadOnly(['get_notion_config']);
});

test('goal preflight validates both schemas and locates one current-month page without writes', async () => {
  const configBefore = structuredClone(storedConfig);
  const result = await notion.testGoalConnection(storedConfig, TODAY);

  assert.equal(result.ok, true);
  assert.equal(result.preflight.monthPageId, 'aug-page');
  assert.equal(result.preflight.timeframeDatabaseId, IDS.timeframeDb);
  assert.equal(result.preflight.timeframeDataSourceId, IDS.timeframeSource);
  assert.equal(result.preflight.today, TODAY);
  assert.deepEqual(storedConfig, configBefore);
  assert.deepEqual(
    requests.map(({ url, init }) => [init.method || 'GET', new URL(url).pathname]),
    [
      ['GET', `/v1/databases/${IDS.goalDb}`],
      ['GET', `/v1/databases/${IDS.timeframeDb}`],
      ['POST', `/v1/databases/${IDS.timeframeDb}/query`],
    ]
  );
  assert.deepEqual(JSON.parse(requests[2].init.body), {
    filter: { property: '类型', select: { equals: '月' } },
    page_size: 100,
  });
  assertPreflightReadOnly();
});

test('goal preflight accepts the complete pinned 2022 dual-relation shape', async () => {
  const goal = goalDatabase();
  goal.properties.所属月.relation.type = 'dual_property';
  installPreflightHandler({ goal });

  const result = await notion.testGoalConnection(storedConfig, TODAY);
  assert.equal(result.ok, true);
  assert.equal(result.preflight.monthPageId, 'aug-page');
  assertPreflightReadOnly();
});

test('each missing goal property reports its name, expected type, and actual type', async () => {
  const configBefore = structuredClone(storedConfig);
  const requirements = [
    ['名称', 'title'],
    ['分类', 'select'],
    ['本月总量', 'number'],
    ['已完成', 'number'],
    ['所属月', 'relation'],
    ['本地目标 ID', 'rich_text'],
  ];

  for (const [property, expected] of requirements) {
    requests = [];
    invokeCalls = [];
    const goal = goalDatabase();
    delete goal.properties[property];
    installPreflightHandler({ goal });

    const result = await notion.testGoalConnection(storedConfig, TODAY);
    assert.equal(result.ok, false, property);
    assert.equal(result.issue.code, 'property_schema', property);
    assert.equal(result.issue.property, property);
    assert.equal(result.issue.expected, expected);
    assert.equal(result.issue.actual, 'missing');
    assert.equal(requests.length, 1);
    assert.deepEqual(storedConfig, configBefore);
    assertPreflightReadOnly();
  }
});

test('missing timeframe properties report their expected and actual types before query', async () => {
  for (const [property, expected] of [['类型', 'select'], ['日期', 'date']]) {
    requests = [];
    invokeCalls = [];
    const timeframe = timeframeDatabase();
    delete timeframe.properties[property];
    installPreflightHandler({ timeframe });

    const result = await notion.testGoalConnection(storedConfig, TODAY);
    assert.equal(result.ok, false, property);
    assert.equal(result.issue.code, 'property_schema');
    assert.equal(result.issue.property, property);
    assert.equal(result.issue.expected, expected);
    assert.equal(result.issue.actual, 'missing');
    assert.equal(requests.length, 2);
    assertPreflightReadOnly();
  }
});

test('wrong goal and timeframe property types stop at the relevant schema boundary', async () => {
  const cases = [
    ...[
      ['名称', 'title'],
      ['分类', 'select'],
      ['本月总量', 'number'],
      ['已完成', 'number'],
      ['所属月', 'relation'],
      ['本地目标 ID', 'rich_text'],
    ].map(([property, expected]) => ({ scope: 'goal', property, expected, actual: 'formula' })),
    { scope: 'timeframe', property: '类型', expected: 'select', actual: 'multi_select' },
    { scope: 'timeframe', property: '日期', expected: 'date', actual: 'rich_text' },
  ];

  for (const item of cases) {
    requests = [];
    invokeCalls = [];
    const goal = goalDatabase();
    const timeframe = timeframeDatabase();
    const database = item.scope === 'goal' ? goal : timeframe;
    database.properties[item.property].type = item.actual;
    installPreflightHandler({ goal, timeframe });

    const result = await notion.testGoalConnection(storedConfig, TODAY);
    assert.equal(result.ok, false, `${item.scope}.${item.property}`);
    assert.equal(result.issue.code, 'property_schema');
    assert.equal(result.issue.property, item.property);
    assert.equal(result.issue.expected, item.expected);
    assert.equal(result.issue.actual, item.actual);
    assert.equal(requests.length, item.scope === 'goal' ? 1 : 2);
    assertPreflightReadOnly();
  }
});

test('category and month select option contracts are enforced', async () => {
  for (const missing of ['工作', '学习', '生活', '兴趣爱好']) {
    requests = [];
    invokeCalls = [];
    const goal = goalDatabase();
    goal.properties.分类.select.options = goal.properties.分类.select.options
      .filter(({ name }) => name !== missing);
    installPreflightHandler({ goal });
    const result = await notion.testGoalConnection(storedConfig, TODAY);
    assert.equal(result.ok, false, missing);
    assert.equal(result.issue.code, 'category_options');
    assert.equal(result.issue.property, '分类');
    assertPreflightReadOnly();
  }

  requests = [];
  invokeCalls = [];
  const goalWithExtraCategory = goalDatabase();
  goalWithExtraCategory.properties.分类.select.options.push({ id: 'other', name: '其它' });
  installPreflightHandler({ goal: goalWithExtraCategory });
  const extraResult = await notion.testGoalConnection(storedConfig, TODAY);
  assert.equal(extraResult.ok, false);
  assert.equal(extraResult.issue.code, 'category_options');
  assertPreflightReadOnly();

  requests = [];
  invokeCalls = [];
  const timeframe = timeframeDatabase();
  timeframe.properties.类型.select.options = timeframe.properties.类型.select.options
    .filter(({ name }) => name !== '月');
  installPreflightHandler({ timeframe });
  const monthResult = await notion.testGoalConnection(storedConfig, TODAY);
  assert.equal(monthResult.ok, false);
  assert.equal(monthResult.issue.code, 'select_option');
  assert.equal(monthResult.issue.property, '类型');
  assert.equal(requests.length, 2);
  assertPreflightReadOnly();
});

test('relation target, direction, reverse property, and reverse target form a closed loop', async () => {
  const cases = [
    {
      code: 'relation_target',
      mutate(goal) { delete goal.properties.所属月.relation.database_id; },
    },
    {
      code: 'relation_direction',
      mutate(goal) {
        goal.properties.所属月.relation.type = 'single_property';
        goal.properties.所属月.relation.single_property = {};
        delete goal.properties.所属月.relation.dual_property;
      },
    },
    {
      code: 'relation_reverse_metadata',
      mutate(goal) {
        goal.properties.所属月.relation.type = 'dual_property';
        goal.properties.所属月.relation.dual_property = {};
      },
    },
    {
      code: 'reverse_relation_missing',
      mutate(_goal, timeframe) { delete timeframe.properties.月度番茄目标; },
    },
    {
      code: 'reverse_relation_missing',
      mutate(goal) {
        goal.properties.所属月.relation.dual_property.synced_property_id = 'wrong-id';
        // 同名属性不能掩盖稳定 ID 不匹配。
      },
    },
    {
      code: 'reverse_relation_schema',
      mutate(_goal, timeframe) {
        timeframe.properties.月度番茄目标.type = 'formula';
      },
    },
    {
      code: 'reverse_relation_direction',
      mutate(_goal, timeframe) {
        timeframe.properties.月度番茄目标.relation.type = 'single_property';
        timeframe.properties.月度番茄目标.relation.single_property = {};
        delete timeframe.properties.月度番茄目标.relation.dual_property;
      },
    },
    {
      code: 'reverse_relation_target',
      mutate(_goal, timeframe) {
        timeframe.properties.月度番茄目标.relation.database_id = '33333333-3333-3333-3333-333333333333';
      },
    },
    {
      code: 'reverse_relation_pair',
      mutate(_goal, timeframe) {
        timeframe.properties.月度番茄目标.relation.dual_property.synced_property_id = 'other-goal-relation';
        // 同名「所属月」不能掩盖 reciprocal ID 不匹配。
      },
    },
  ];

  for (const item of cases) {
    requests = [];
    invokeCalls = [];
    const goal = goalDatabase();
    const timeframe = timeframeDatabase();
    item.mutate(goal, timeframe);
    installPreflightHandler({ goal, timeframe });
    const result = await notion.testGoalConnection(storedConfig, TODAY);
    assert.equal(result.ok, false, item.code);
    assert.equal(result.issue.code, item.code);
    assert.equal(requests.some(({ url }) => url.endsWith('/query')), false);
    assertPreflightReadOnly();
  }
});

test('encoded reverse property IDs match without relying on the property name', async () => {
  const goal = goalDatabase();
  const timeframe = timeframeDatabase();
  goal.properties.所属月.relation.dual_property.synced_property_id = 'reverse%3Arel';
  goal.properties.所属月.relation.dual_property.synced_property_name = '';
  timeframe.properties.月度番茄目标.id = 'reverse:rel';
  installPreflightHandler({ goal, timeframe });

  const result = await notion.testGoalConnection(storedConfig, TODAY);
  assert.equal(result.ok, true);
  assertPreflightReadOnly();
});

test('zero matching pages and multiple matching pages return distinct errors', async () => {
  installPreflightHandler({
    query: () => pageList([
      monthPage('july', '2026-07-01', '2026-07-31'),
      monthPage('rogue-week', '2026-08-01', '2026-08-31', '周'),
    ]),
  });
  const none = await notion.testGoalConnection(storedConfig, TODAY);
  assert.equal(none.ok, false);
  assert.equal(none.issue.code, 'month_page_not_found');
  assert.equal(none.issue.actual, 0);
  assertPreflightReadOnly();

  requests = [];
  invokeCalls = [];
  installPreflightHandler({
    query: (body) => body.start_cursor
      ? pageList([monthPage('aug-2', '2026-08-15', '2026-08-31')])
      : pageList([monthPage('aug-1', '2026-08-01', '2026-08-31')], true, 'cursor-1'),
  });
  const multiple = await notion.testGoalConnection(storedConfig, TODAY);
  assert.equal(multiple.ok, false);
  assert.equal(multiple.issue.code, 'month_page_ambiguous');
  assert.equal(multiple.issue.actual, 2);
  assert.equal(requests.filter(({ url }) => url.endsWith('/query')).length, 2);
  assertPreflightReadOnly();
});

test('pagination keeps the filter and finds the sole current month on a later page', async () => {
  installPreflightHandler({
    query: (body) => body.start_cursor
      ? pageList([monthPage('aug-later', '2026-08-01', '2026-08-31')])
      : pageList([monthPage('july', '2026-07-01', '2026-07-31')], true, 'cursor-1'),
  });

  const result = await notion.testGoalConnection(storedConfig, TODAY);
  assert.equal(result.ok, true);
  assert.equal(result.preflight.monthPageId, 'aug-later');
  const queryBodies = requests
    .filter(({ url }) => url.endsWith('/query'))
    .map(({ init }) => JSON.parse(init.body));
  assert.deepEqual(queryBodies, [
    { filter: { property: '类型', select: { equals: '月' } }, page_size: 100 },
    {
      filter: { property: '类型', select: { equals: '月' } },
      page_size: 100,
      start_cursor: 'cursor-1',
    },
  ]);
  assertPreflightReadOnly();
});

test('missing or invalid Rust snapshot dates fail locally without an HTTP request', async () => {
  for (const today of ['', '2026-02-31', '08/25/2026', '2026-08-25T00:00:00Z']) {
    requests = [];
    invokeCalls = [];
    const result = await notion.testGoalConnection(storedConfig, today);
    assert.equal(result.ok, false, today);
    assert.equal(result.issue.code, 'snapshot_date');
    assert.equal(requests.length, 0);
    assertPreflightReadOnly();
  }
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
