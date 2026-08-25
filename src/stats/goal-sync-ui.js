function monthKeyFromToday(today) {
  const match = /^(\d{4})-(\d{2})-\d{2}$/.exec(String(today || ''));
  if (!match) return '';
  const month = Number(match[2]);
  return month >= 1 && month <= 12 ? `${match[1]}-${match[2]}` : '';
}

export function formatMonthLabel(month) {
  const match = /^(\d{4})-(\d{2})$/.exec(String(month || ''));
  if (!match) return '当前月份未就绪';
  return `${match[1]} 年 ${Number(match[2])} 月`;
}

function safeCount(value) {
  const number = Number(value);
  return Number.isFinite(number) && number >= 0 ? Math.floor(number) : 0;
}

function countSummary(summary) {
  return Object.freeze({
    created: safeCount(summary?.created),
    updated: safeCount(summary?.updated),
    failed: safeCount(summary?.failed),
  });
}

function countsText(summary) {
  return `新增 ${summary.created} / 更新 ${summary.updated} / 失败 ${summary.failed}`;
}

export function latestMonthlyGoalSummary(exportLog) {
  if (!exportLog || typeof exportLog !== 'object') return null;
  const entries = [];
  for (const [key, value] of Object.entries(exportLog)) {
    const match = /^monthly:(\d{4}-\d{2})$/.exec(key);
    if (!match || !value || typeof value !== 'object') continue;
    const at = String(value.at || '');
    const atMs = Date.parse(at);
    entries.push(Object.freeze({
      month: match[1],
      at,
      atMs: Number.isFinite(atMs) ? atMs : 0,
      ...countSummary(value),
    }));
  }
  entries.sort((left, right) => (
    left.atMs - right.atMs || left.month.localeCompare(right.month)
  ));
  return entries.at(-1) || null;
}

/**
 * 只让会改变月度目标业务字段的快照触碰表单行 DOM。
 * 完成量、今日勾选、同步状态和导出摘要变化都不应打断正在编辑的草稿。
 */
export function monthlyGoalRowsRenderKey(snapshot) {
  const goals = Array.isArray(snapshot?.monthlyGoals) ? snapshot.monthlyGoals : [];
  const displayedGoals = goals.map((goal) => [
    String(goal?.id || ''),
    String(goal?.title || ''),
    String(goal?.category || ''),
    Number(goal?.monthTarget) || 0,
  ]);
  return JSON.stringify([String(snapshot?.today || '').slice(0, 7), displayedGoals]);
}

function formatSyncTimestamp(at) {
  const date = new Date(at);
  if (!Number.isFinite(date.getTime())) return '时间未知';
  return date.toLocaleString('zh-CN', {
    year: 'numeric',
    month: 'numeric',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  });
}

function resultErrors(errors) {
  if (!Array.isArray(errors)) return [];
  return errors.map((item) => {
    const title = String(item?.goalTitle || item?.goalId || '未命名目标');
    const message = String(item?.error || '未知错误');
    return `“${title}”：${message}`;
  });
}

function resultView(summary) {
  const counts = countSummary(summary);
  const succeeded = counts.created + counts.updated;
  if (summary?.empty) {
    return {
      kind: 'idle',
      headline: '当前月没有可同步的目标',
      detail: '先添加至少一个月度目标，再同步到 Notion。',
      errors: [],
    };
  }
  if (counts.failed > 0 && succeeded > 0) {
    return {
      kind: 'partial',
      headline: `部分完成 · ${countsText(counts)}`,
      detail: '成功项已经保留；修复下面的问题后，可直接再次同步。',
      errors: resultErrors(summary?.errors),
    };
  }
  if (counts.failed > 0) {
    return {
      kind: 'error',
      headline: `同步未完成 · ${countsText(counts)}`,
      detail: '本地目标没有被改动；修复下面的问题后可安全重试。',
      errors: resultErrors(summary?.errors),
    };
  }
  return {
    kind: 'success',
    headline: `同步完成 · ${countsText(counts)}`,
    detail: '重复同步只会更新同一月份、同一目标 ID 的记录。',
    errors: [],
  };
}

export function buildMonthlyGoalSyncView({
  today,
  goalCount = 0,
  configReady = true,
  configured = false,
  syncing = false,
  syncStage = 'validating',
  syncMonth = '',
  attempt = null,
  exportLog = null,
} = {}) {
  const liveMonth = monthKeyFromToday(today);
  const month = syncing && /^\d{4}-\d{2}$/.test(syncMonth) ? syncMonth : liveMonth;
  const monthLabel = formatMonthLabel(month);
  const hasGoals = Number(goalCount) > 0;
  const buttonDisabled = syncing || !configReady || !configured || !hasGoals || !month;
  const buttonText = syncing
    ? '同步中…'
    : !configReady
      ? '读取配置中…'
    : !month
      ? '日期未就绪'
      : !configured
        ? '先配置 Notion'
        : !hasGoals
          ? '暂无目标'
          : '同步本月到 Notion';
  const buttonTitle = syncing
    ? '正在同步，请等待本次操作完成'
    : !configReady
      ? '正在读取本机保存的 Notion 配置'
    : !configured
      ? '请先在下方 Notion 导出中填写 Token 和月度目标 DB ID'
      : !hasGoals
        ? '当前月没有可同步的目标'
        : `创建或更新 ${monthLabel} 的 Notion 月度快照`;

  let status;
  if (syncing) {
    const writing = syncStage === 'syncing';
    status = {
      kind: 'loading',
      headline: writing
        ? `正在同步 ${monthLabel}…`
        : `正在校验 ${monthLabel} 的 Notion 结构…`,
      detail: writing
        ? '正在逐项目创建或更新；本地目标编辑、计时和休息流程不受影响。'
        : '写入尚未开始；正在核对数据库结构、月份页面和现有记录。',
      errors: [],
    };
  } else if (!configReady) {
    status = {
      kind: 'loading',
      headline: '正在读取 Notion 配置…',
      detail: 'Token 只从本机读取，不会显示在同步摘要中。',
      errors: [],
    };
  } else if (attempt?.month === month && attempt.kind === 'result') {
    status = resultView(attempt.summary);
  } else if (attempt?.month === month && attempt.kind === 'error') {
    status = {
      kind: 'error',
      headline: '同步失败',
      detail: String(attempt.message || '未知错误'),
      errors: [],
    };
  } else if (!month) {
    status = {
      kind: 'idle',
      headline: '本地日期尚未就绪',
      detail: '稍后刷新窗口即可；App 不会使用浏览器时间代替 Rust 快照。',
      errors: [],
    };
  } else if (!configured) {
    status = {
      kind: 'idle',
      headline: '先完成 Notion 配置',
      detail: '请在下方填写 Token 和月度目标 DB ID，并先测试目标库连接。',
      errors: [],
    };
  } else if (!hasGoals) {
    status = {
      kind: 'idle',
      headline: '当前月没有可同步的目标',
      detail: '先添加至少一个月度目标；此状态不会发送 Notion 请求。',
      errors: [],
    };
  } else {
    const latest = latestMonthlyGoalSummary(exportLog);
    if (latest) {
      const counts = countSummary(latest);
      const succeeded = counts.created + counts.updated;
      status = {
        kind: counts.failed > 0 ? (succeeded > 0 ? 'partial' : 'error') : 'success',
        headline: `最近同步 · ${formatMonthLabel(latest.month)}`,
        detail: `${formatSyncTimestamp(latest.at)} · ${countsText(counts)}`,
        errors: [],
      };
    } else {
      status = {
        kind: 'idle',
        headline: `${monthLabel} 尚未同步`,
        detail: '只会创建或更新当前月记录，不会删除或归档 Notion 内容。',
        errors: [],
      };
    }
  }

  return Object.freeze({
    month,
    monthLabel,
    buttonDisabled,
    buttonText,
    buttonTitle,
    ...status,
    errors: Object.freeze(status.errors),
  });
}

/** 同一窗口只允许一个同步 Promise；重复触发共享当前结果。 */
export function createSingleFlight(run) {
  let current = null;
  return (...args) => {
    if (current) return current;
    current = Promise.resolve()
      .then(() => run(...args))
      .finally(() => {
        current = null;
      });
    return current;
  };
}

/** 跟踪已触发的本地目标保存；settleCurrent 只等待调用瞬间已经存在的操作。 */
export function createMutationTracker() {
  const pending = new Set();
  return Object.freeze({
    track(promise) {
      const operation = Promise.resolve(promise);
      pending.add(operation);
      operation.then(
        () => pending.delete(operation),
        () => pending.delete(operation)
      );
      return operation;
    },
    settleCurrent() {
      return Promise.all([...pending]);
    },
    get size() {
      return pending.size;
    },
  });
}

/** 并发刷新只应用目前最新的成功响应；较新的失败请求不会淘汰旧成功结果。 */
export function createLatestSuccessGate() {
  let issuedRevision = 0;
  let appliedRevision = 0;
  return Object.freeze({
    issue() {
      issuedRevision += 1;
      return issuedRevision;
    },
    accept(revision) {
      if (!Number.isInteger(revision) || revision < appliedRevision) return false;
      appliedRevision = revision;
      return true;
    },
  });
}
