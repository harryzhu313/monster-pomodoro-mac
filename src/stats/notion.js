// Notion 导出:近乎原样移植旧 background/service-worker.js:902-1142,
// fetch 换 tauri-plugin-http 的 fetch(scope 仅 api.notion.com,无 CORS),
// 配置经 get/set_notion_config 存 store.json,导出日志经 log_notion_export 记账。
// ⚠️ 红线 2:字段名与分类选项是用户既有 Notion 数据库的 schema 常量,不可改动。

const tauri = window.__TAURI__ ?? null;
const httpFetch = tauri?.http?.fetch ?? window.fetch.bind(window);

const NOTION_VERSION = '2022-06-28';
const NOTION_API = 'https://api.notion.com/v1';
const EMPTY_NOTION_CONFIG = Object.freeze({
  token: '',
  taskDbId: '',
  dayDbId: '',
  goalDbId: '',
});

/** 分类白名单(= Notion select 选项,不可改) */
const CATEGORY_VALUES = Object.freeze(['工作', '学习', '生活', '兴趣爱好']);
const GOAL_SCHEMA = Object.freeze([
  ['名称', 'title'],
  ['分类', 'select'],
  ['本月总量', 'number'],
  ['已完成', 'number'],
  ['所属月', 'relation'],
  ['本地目标 ID', 'rich_text'],
]);
const TIMEFRAME_SCHEMA = Object.freeze([
  ['类型', 'select'],
  ['日期', 'date'],
]);

async function invoke(cmd, args) {
  if (!tauri) {
    console.log('[mock invoke]', cmd, args ?? '');
    return null;
  }
  return tauri.core.invoke(cmd, args);
}

export async function getNotionConfig() {
  return normalizeNotionConfig((await invoke('get_notion_config')) ?? EMPTY_NOTION_CONFIG);
}

export function setNotionConfig(config) {
  return invoke('set_notion_config', { config });
}

function notionHeaders(token) {
  return {
    Authorization: `Bearer ${token}`,
    'Notion-Version': NOTION_VERSION,
    'Content-Type': 'application/json',
  };
}

function normalizeNotionConfig(config) {
  return {
    token: String(config?.token || ''),
    taskDbId: String(config?.taskDbId || ''),
    dayDbId: String(config?.dayDbId || ''),
    goalDbId: String(config?.goalDbId || ''),
  };
}

async function connectionConfig(configSnapshot) {
  const config = configSnapshot ?? await getNotionConfig();
  return Object.freeze(normalizeNotionConfig(config));
}

/** 用固定配置完成一次请求；连接测试靠它避免执行中途混用不同 Token。 */
async function notionFetchWithConfig(config, path, init = {}) {
  const cfg = normalizeNotionConfig(config);
  if (!cfg.token) throw new Error('尚未配置 Notion token');
  const res = await httpFetch(NOTION_API + path, {
    ...init,
    headers: { ...notionHeaders(cfg.token), ...(init.headers || {}) },
  });
  const text = await res.text();
  let body = null;
  try {
    body = text ? JSON.parse(text) : null;
  } catch {
    body = { raw: text };
  }
  if (!res.ok) {
    const msg = body?.message || body?.raw || `HTTP ${res.status}`;
    const error = new Error(`Notion API ${res.status}: ${msg}`);
    error.status = res.status;
    error.apiCode = body?.code || '';
    error.apiMessage = msg;
    throw error;
  }
  return body;
}

async function notionFetch(path, init = {}) {
  return notionFetchWithConfig(await getNotionConfig(), path, init);
}

class GoalPreflightError extends Error {
  constructor(code, message, issue = {}) {
    super(message);
    this.name = 'GoalPreflightError';
    this.code = code;
    this.issue = Object.freeze({ code, ...issue });
  }
}

function failPreflight(code, message, issue) {
  throw new GoalPreflightError(code, message, issue);
}

function propertyType(property) {
  if (!property) return 'missing';
  return typeof property.type === 'string' && property.type ? property.type : 'unknown';
}

/** 精确按属性名验证 schema；额外属性不参与判断。 */
function requireProperties(database, scope, requirements) {
  const properties = database?.properties || {};
  for (const [name, expected] of requirements) {
    const actual = propertyType(properties[name]);
    if (actual !== expected) {
      const actualText = actual === 'missing' ? '缺失' : actual;
      const action = scope === '月度目标' && name === '所属月' && actual === 'missing'
        ? '请确认该属性存在，并把它指向的「时间框架」数据库连接到当前 Integration'
        : '请在 Notion 中修正该属性';
      failPreflight(
        'property_schema',
        `${scope}属性「${name}」应为 ${expected}，实际为 ${actualText}；${action}`,
        { scope, property: name, expected, actual }
      );
    }
  }
  return properties;
}

function selectOptionNames(property) {
  return (property?.select?.options || [])
    .map((option) => String(option?.name || ''))
    .filter(Boolean);
}

function requireExactCategoryOptions(property) {
  const actual = selectOptionNames(property);
  const expected = [...CATEGORY_VALUES];
  const actualSet = new Set(actual);
  const exact = actual.length === expected.length && expected.every((name) => actualSet.has(name));
  if (!exact) {
    failPreflight(
      'category_options',
      `月度目标属性「分类」的选项必须且只能是 ${expected.join(' / ')}；请在 Notion 中修正选项`,
      { scope: '月度目标', property: '分类', expected, actual }
    );
  }
}

function requireSelectOption(property, scope, propertyName, optionName) {
  const actual = selectOptionNames(property);
  if (!actual.includes(optionName)) {
    failPreflight(
      'select_option',
      `${scope}属性「${propertyName}」缺少选项「${optionName}」；请先在 Notion 中添加`,
      { scope, property: propertyName, expected: optionName, actual }
    );
  }
}

function compactNotionId(value) {
  return String(value || '').replace(/-/g, '').toLowerCase();
}

function sameNotionId(left, right) {
  const a = compactNotionId(left);
  const b = compactNotionId(right);
  return !!a && !!b && a === b;
}

function propertyId(value) {
  const raw = String(value || '');
  try {
    return decodeURIComponent(raw);
  } catch {
    return raw;
  }
}

function samePropertyId(left, right) {
  return !!left && !!right && propertyId(left) === propertyId(right);
}

function relationShape(property) {
  const relation = property?.relation || {};
  const type = relation.type
    || (relation.dual_property ? 'dual_property' : '')
    || (relation.single_property ? 'single_property' : '');
  const dual = relation.dual_property || {};
  return {
    databaseId: String(relation.database_id || ''),
    dataSourceId: String(relation.data_source_id || ''),
    type,
    syncedPropertyId: String(dual.synced_property_id || relation.synced_property_id || ''),
    syncedPropertyName: String(dual.synced_property_name || relation.synced_property_name || ''),
  };
}

function requireGoalRelation(property) {
  const relation = relationShape(property);
  if (!relation.databaseId) {
    failPreflight(
      'relation_target',
      '月度目标属性「所属月」没有可查询的目标数据库；请重新关联到「时间框架」数据库',
      { scope: '月度目标', property: '所属月', expected: 'relation.database_id', actual: 'missing' }
    );
  }
  if (relation.type !== 'dual_property') {
    failPreflight(
      'relation_direction',
      '月度目标属性「所属月」必须开启双向 relation；请在 Notion 中显示对应的反向属性',
      { scope: '月度目标', property: '所属月', expected: 'dual_property', actual: relation.type || 'unknown' }
    );
  }
  if (!relation.syncedPropertyId && !relation.syncedPropertyName) {
    failPreflight(
      'relation_reverse_metadata',
      '无法从「所属月」确认时间框架中的反向 relation；请删除后重新建立双向关联',
      { scope: '月度目标', property: '所属月', expected: '双向属性 ID 或名称', actual: 'missing' }
    );
  }
  return relation;
}

function findReverseRelation(properties, relation) {
  const entries = Object.entries(properties || {});
  if (relation.syncedPropertyId) {
    return entries.find(([, property]) => samePropertyId(property?.id, relation.syncedPropertyId));
  }
  if (relation.syncedPropertyName) {
    return entries.find(([name, property]) => (
      name === relation.syncedPropertyName || property?.name === relation.syncedPropertyName
    ));
  }
  return undefined;
}

function requireReverseRelation(timeframeProperties, goalRelation, goalDatabaseId, goalProperty) {
  const found = findReverseRelation(timeframeProperties, goalRelation);
  const expectedName = goalRelation.syncedPropertyName || goalRelation.syncedPropertyId;
  if (!found) {
    failPreflight(
      'reverse_relation_missing',
      `时间框架中缺少与「所属月」对应的反向 relation「${expectedName}」；请检查双向关联`,
      { scope: '时间框架', property: expectedName, expected: 'relation', actual: 'missing' }
    );
  }
  const [name, property] = found;
  const actualType = propertyType(property);
  if (actualType !== 'relation') {
    failPreflight(
      'reverse_relation_schema',
      `时间框架属性「${name}」应为 relation，实际为 ${actualType}；请修正双向关联`,
      { scope: '时间框架', property: name, expected: 'relation', actual: actualType }
    );
  }
  const reverse = relationShape(property);
  if (reverse.type !== 'dual_property') {
    failPreflight(
      'reverse_relation_direction',
      `时间框架属性「${name}」不是双向 relation；请重新连接「所属月」`,
      { scope: '时间框架', property: name, expected: 'dual_property', actual: reverse.type || 'unknown' }
    );
  }
  if (!sameNotionId(reverse.databaseId, goalDatabaseId)) {
    failPreflight(
      'reverse_relation_target',
      `时间框架属性「${name}」没有反向指向当前月度目标库；请修正 relation 目标`,
      { scope: '时间框架', property: name, expected: '当前月度目标库', actual: reverse.databaseId || 'missing' }
    );
  }
  const goalPropertyId = String(goalProperty?.id || '');
  const goalPropertyName = String(goalProperty?.name || '所属月');
  const reciprocalMatches = reverse.syncedPropertyId
    ? samePropertyId(reverse.syncedPropertyId, goalPropertyId)
    : reverse.syncedPropertyName === goalPropertyName;
  if (!reciprocalMatches) {
    const actual = reverse.syncedPropertyId || reverse.syncedPropertyName || 'missing';
    failPreflight(
      'reverse_relation_pair',
      `时间框架属性「${name}」没有与目标库「所属月」组成同一组双向 relation；请重新连接这两个属性`,
      { scope: '时间框架', property: name, expected: goalPropertyId || goalPropertyName, actual }
    );
  }
}

function isoDatePart(value) {
  const match = /^(\d{4})-(\d{2})-(\d{2})/.exec(String(value || ''));
  if (!match) return '';
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  if (month < 1 || month > 12 || day < 1) return '';
  const monthDays = new Date(Date.UTC(year, month, 0)).getUTCDate();
  return day <= monthDays ? `${match[1]}-${match[2]}-${match[3]}` : '';
}

function requireSnapshotDate(today) {
  const normalized = isoDatePart(today);
  if (!normalized || normalized !== today) {
    failPreflight(
      'snapshot_date',
      '本地日期尚未就绪，请稍后重试；App 不会使用浏览器时间替代',
      { scope: 'App 快照', property: 'today', expected: 'YYYY-MM-DD', actual: String(today || 'missing') }
    );
  }
  return normalized;
}

function pageContainsDate(page, today) {
  if (page?.object !== 'page') return false;
  if (page?.properties?.类型?.select?.name !== '月') return false;
  const date = page?.properties?.日期?.date;
  const start = isoDatePart(date?.start);
  const end = isoDatePart(date?.end) || start;
  return !!start && start <= today && today <= end;
}

async function queryDatabasePages(config, databaseId, filter) {
  const pages = [];
  const cursors = new Set();
  let cursor = '';
  do {
    const body = { filter, page_size: 100 };
    if (cursor) body.start_cursor = cursor;
    const response = await notionFetchWithConfig(config, `/databases/${databaseId}/query`, {
      method: 'POST',
      body: JSON.stringify(body),
    });
    pages.push(...(Array.isArray(response?.results) ? response.results : []));
    if (!response?.has_more) break;
    const nextCursor = String(response?.next_cursor || '');
    if (!nextCursor || cursors.has(nextCursor)) {
      failPreflight(
        'pagination',
        '时间框架分页结果不完整，请稍后重新测试连接',
        { scope: '时间框架', property: 'next_cursor', expected: '新的游标', actual: nextCursor || 'missing' }
      );
    }
    cursors.add(nextCursor);
    cursor = nextCursor;
  } while (cursor);
  return pages;
}

function databaseTitle(database) {
  return database?.title?.map((part) => part?.plain_text || part?.text?.content || '').join('')
    || '(无标题)';
}

function accessError(scope, error) {
  if (error?.status === 401) {
    return new GoalPreflightError(
      'token',
      'Token 无效或已失效，请检查 Internal Integration Token',
      { scope, expected: '有效且具备读取权限的 Token', actual: '401' }
    );
  }
  if (error?.status === 403 || error?.status === 404) {
    const action = scope === '月度目标'
      ? '请检查月度目标 DB ID，并确认该数据库已连接当前 Integration'
      : '请在「时间框架」数据库右上角 Connections 中添加当前 Integration，并检查「所属月」relation';
    return new GoalPreflightError(
      'database_access',
      `无法读取${scope} DB；${action}`,
      { scope, expected: '可读取', actual: String(error.status) }
    );
  }
  if (error?.status === 400 && /multiple data sources/i.test(error?.apiMessage || '')) {
    return new GoalPreflightError(
      'multiple_data_sources',
      `${scope}数据库包含多个数据源，当前兼容版本无法读取；请改用单数据源数据库`,
      { scope, expected: '单数据源数据库', actual: 'multiple data sources' }
    );
  }
  if (error?.status === 429) {
    return new GoalPreflightError(
      'rate_limited',
      'Notion 请求过于频繁，请稍后重新测试',
      { scope, expected: '请求成功', actual: '429' }
    );
  }
  if (error?.status >= 500) {
    return new GoalPreflightError(
      'notion_unavailable',
      'Notion 服务暂时不可用，请稍后重新测试',
      { scope, expected: '请求成功', actual: String(error.status) }
    );
  }
  return new GoalPreflightError(
    'notion_request',
    `${scope}读取失败：${String(error?.message || error)}`,
    { scope, expected: '请求成功', actual: String(error?.message || error) }
  );
}

async function retrieveDatabase(config, databaseId, scope) {
  try {
    return await notionFetchWithConfig(config, `/databases/${databaseId}`);
  } catch (error) {
    throw accessError(scope, error);
  }
}

/**
 * Phase 02 零写入预检。today 必须来自同一次 Rust Snapshot；这里不读取系统时间。
 * 返回的 monthPageId/timeframeDatabaseId 将在 Phase 03 作为同步核心输入。
 */
export async function preflightGoalSync(today, configSnapshot = null) {
  const snapshotDate = requireSnapshotDate(today);
  const config = await connectionConfig(configSnapshot);
  if (!config.token) {
    failPreflight('config', '请先填入 token', {
      scope: 'Notion 配置', property: 'token', expected: '非空', actual: 'missing',
    });
  }
  if (!config.goalDbId) {
    failPreflight('config', '请先填入月度目标 DB ID', {
      scope: 'Notion 配置', property: 'goalDbId', expected: '非空', actual: 'missing',
    });
  }

  const goalDatabase = await retrieveDatabase(config, config.goalDbId, '月度目标');
  const goalProperties = requireProperties(goalDatabase, '月度目标', GOAL_SCHEMA);
  requireExactCategoryOptions(goalProperties.分类);
  const goalRelation = requireGoalRelation(goalProperties.所属月);

  const timeframeDatabase = await retrieveDatabase(config, goalRelation.databaseId, '时间框架');
  const timeframeProperties = requireProperties(timeframeDatabase, '时间框架', TIMEFRAME_SCHEMA);
  requireSelectOption(timeframeProperties.类型, '时间框架', '类型', '月');
  requireReverseRelation(
    timeframeProperties,
    goalRelation,
    goalDatabase.id || config.goalDbId,
    goalProperties.所属月
  );

  let pages;
  try {
    pages = await queryDatabasePages(config, goalRelation.databaseId, {
      property: '类型',
      select: { equals: '月' },
    });
  } catch (error) {
    if (error instanceof GoalPreflightError) throw error;
    throw accessError('时间框架', error);
  }
  const matches = pages.filter((page) => pageContainsDate(page, snapshotDate));
  if (matches.length === 0) {
    failPreflight(
      'month_page_not_found',
      `时间框架中没有日期范围包含 ${snapshotDate} 的「月」页面；请先补齐当月页面后重试`,
      { scope: '时间框架', property: '日期', expected: `唯一页面包含 ${snapshotDate}`, actual: 0 }
    );
  }
  if (matches.length > 1) {
    failPreflight(
      'month_page_ambiguous',
      `时间框架中有 ${matches.length} 个「月」页面同时包含 ${snapshotDate}；请只保留一个匹配页面`,
      { scope: '时间框架', property: '日期', expected: `唯一页面包含 ${snapshotDate}`, actual: matches.length }
    );
  }
  const monthPageId = String(matches[0]?.id || '');
  if (!monthPageId) {
    failPreflight(
      'month_page_id',
      '已找到当月页面，但 Notion 没有返回页面 ID；请稍后重新测试',
      { scope: '时间框架', property: 'page.id', expected: '非空', actual: 'missing' }
    );
  }

  return Object.freeze({
    goalDatabaseTitle: databaseTitle(goalDatabase),
    timeframeDatabaseTitle: databaseTitle(timeframeDatabase),
    timeframeDatabaseId: goalRelation.databaseId,
    timeframeDataSourceId: goalRelation.dataSourceId,
    monthPageId,
    today: snapshotDate,
  });
}

/** 测试连接:读任务 DB 的 schema;日页面 DB 有值也顺便读(旧 notionTestConnection) */
export async function testConnection(configSnapshot = null) {
  const cfg = await connectionConfig(configSnapshot);
  if (!cfg.token) return { ok: false, error: '请先填入 token' };
  if (!cfg.taskDbId) return { ok: false, error: '请先填入任务 DB ID' };
  try {
    const task = await notionFetchWithConfig(cfg, `/databases/${cfg.taskDbId}`);
    const taskTitle = task?.title?.[0]?.plain_text || '(无标题)';
    let dayInfo = '';
    if (cfg.dayDbId) {
      const day = await notionFetchWithConfig(cfg, `/databases/${cfg.dayDbId}`);
      const dayTitle = day?.title?.[0]?.plain_text || '(无标题)';
      dayInfo = `｜日页面 DB:「${dayTitle}」`;
    }
    return { ok: true, message: `任务 DB:「${taskTitle}」${dayInfo}` };
  } catch (e) {
    return { ok: false, error: String(e.message || e) };
  }
}

/** Phase 02 完整只读检查：schema、双向 relation、时间框架与唯一月份页。 */
export async function testGoalConnection(configSnapshot = null, today = '') {
  try {
    const result = await preflightGoalSync(today, configSnapshot);
    return {
      ok: true,
      message: `月度目标与时间框架结构正确，已定位 ${result.today.slice(0, 7)} 的唯一月份页面`,
      preflight: result,
    };
  } catch (error) {
    return {
      ok: false,
      error: String(error?.message || error),
      issue: error?.issue || { code: 'unknown' },
    };
  }
}

async function findDayPageId(dayDbId, isoDate) {
  const resp = await notionFetch(`/databases/${dayDbId}/query`, {
    method: 'POST',
    body: JSON.stringify({
      filter: { property: '日期', date: { equals: isoDate } },
      page_size: 1,
    }),
  });
  return resp?.results?.[0]?.id || null;
}

async function findTaskPagesByDate(taskDbId, isoDate) {
  const pages = [];
  let cursor = null;
  do {
    const body = {
      filter: { property: '日期', date: { equals: isoDate } },
      page_size: 100,
    };
    if (cursor) body.start_cursor = cursor;
    const resp = await notionFetch(`/databases/${taskDbId}/query`, {
      method: 'POST',
      body: JSON.stringify(body),
    });
    pages.push(...(resp?.results || []));
    cursor = resp?.has_more ? resp.next_cursor : null;
  } while (cursor);
  return pages;
}

function taskExportTitle(task) {
  return String(task.title || '(未命名)').slice(0, 200);
}

function notionPageTaskTitle(page) {
  const title = page?.properties?.任务名?.title || [];
  return title.map((part) => part.plain_text || part.text?.content || '').join('');
}

function buildTaskPageIndex(pages) {
  const byTitle = new Map();
  let existingDuplicates = 0;
  for (const page of pages) {
    const title = notionPageTaskTitle(page);
    const entry = byTitle.get(title);
    if (entry) {
      entry.duplicates.push(page);
      existingDuplicates++;
    } else {
      byTitle.set(title, { page, duplicates: [] });
    }
  }
  return { byTitle, existingDuplicates };
}

function buildTaskPageProps(task, isoDate, dayPageId) {
  const category = CATEGORY_VALUES.includes(task.category) ? task.category : '工作';
  const used = Number(task.used) || 0;
  const planned = Number(task.planned) || 0;
  const overflow = Math.max(0, used - planned);
  const rotten = Number(task.rotten) || 0;
  const props = {
    任务名: {
      title: [{ type: 'text', text: { content: taskExportTitle(task) } }],
    },
    日期: { date: { start: isoDate } },
    计划番茄: { number: planned },
    实际番茄: { number: used },
    超额番茄: { number: overflow },
    放弃番茄: { number: rotten },
    分类: { select: { name: category } },
  };
  if (dayPageId) {
    props['所属日'] = { relation: [{ id: dayPageId }] };
  }
  return props;
}

/** 把某一天的任务批量导入 Notion(旧 notionExportDay);tasks 由调用方从 bundle 提供 */
export async function exportDay(date, tasks) {
  const cfg = await getNotionConfig();
  if (!cfg.token || !cfg.taskDbId) {
    return { ok: false, error: '请先填入 Notion token 和任务 DB ID' };
  }
  if (!tasks || tasks.length === 0) {
    return { ok: false, error: '这一天没有任务可导入' };
  }

  let dayPageId = null;
  if (cfg.dayDbId) {
    try {
      dayPageId = await findDayPageId(cfg.dayDbId, date);
    } catch (e) {
      return { ok: false, error: `查询日页面失败:${e.message || e}` };
    }
  }

  let taskPageIndex = null;
  try {
    const existingPages = await findTaskPagesByDate(cfg.taskDbId, date);
    taskPageIndex = buildTaskPageIndex(existingPages);
  } catch (e) {
    return { ok: false, error: `查询已导入任务失败:${e.message || e}` };
  }

  const errors = [];
  let created = 0;
  let updated = 0;
  for (const t of tasks) {
    const properties = buildTaskPageProps(t, date, dayPageId);
    const existing = taskPageIndex.byTitle.get(taskExportTitle(t));
    try {
      if (existing?.page?.id) {
        await notionFetch(`/pages/${existing.page.id}`, {
          method: 'PATCH',
          body: JSON.stringify({ properties }),
        });
        updated++;
      } else {
        const createdPage = await notionFetch('/pages', {
          method: 'POST',
          body: JSON.stringify({
            parent: { database_id: cfg.taskDbId },
            properties,
          }),
        });
        taskPageIndex.byTitle.set(taskExportTitle(t), { page: createdPage, duplicates: [] });
        created++;
      }
    } catch (e) {
      errors.push({ task: t.title, error: String(e.message || e) });
    }
  }

  const summary = {
    ok: errors.length === 0,
    created,
    updated,
    failed: errors.length,
    total: tasks.length,
    existingDuplicates: taskPageIndex.existingDuplicates,
    dayPageLinked: !!dayPageId,
    errors,
  };

  await invoke('log_notion_export', { date, summary });
  return summary;
}
