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
const CATEGORY_VALUES = ['工作', '学习', '生活', '兴趣爱好'];

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
    throw error;
  }
  return body;
}

async function notionFetch(path, init = {}) {
  return notionFetchWithConfig(await getNotionConfig(), path, init);
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

/** Phase 01 基础检查：只读目标 DB 元数据；完整 schema 与月份页留到 Phase 02。 */
export async function testGoalConnection(configSnapshot = null) {
  const cfg = await connectionConfig(configSnapshot);
  if (!cfg.token) return { ok: false, error: '请先填入 token' };
  if (!cfg.goalDbId) return { ok: false, error: '请先填入月度目标 DB ID' };
  try {
    const goal = await notionFetchWithConfig(cfg, `/databases/${cfg.goalDbId}`);
    const goalTitle = goal?.title?.[0]?.plain_text || '(无标题)';
    return { ok: true, message: `月度目标 DB:「${goalTitle}」` };
  } catch (e) {
    if (e?.status === 401) {
      return { ok: false, error: 'Token 无效或已失效，请检查 Internal Integration Token' };
    }
    if (e?.status === 403 || e?.status === 404) {
      return {
        ok: false,
        error: '无法读取月度目标 DB，请检查 DB ID，并确认该数据库已连接当前 Integration',
      };
    }
    return { ok: false, error: String(e.message || e) };
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
