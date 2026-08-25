import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const source = await readFile(new URL('./goal-sync-ui.js', import.meta.url), 'utf8');
const sourceUrl = `data:text/javascript;base64,${Buffer.from(source).toString('base64')}`;
const {
  buildMonthlyGoalSyncView,
  createLatestSuccessGate,
  createMutationTracker,
  createSingleFlight,
  latestMonthlyGoalSummary,
  monthlyGoalRowsRenderKey,
} = await import(sourceUrl);

test('configuration, empty-goal, and loading states expose distinct button guidance', () => {
  const loadingConfig = buildMonthlyGoalSyncView({
    today: '2026-08-25', goalCount: 2, configReady: false,
  });
  assert.equal(loadingConfig.buttonDisabled, true);
  assert.equal(loadingConfig.buttonText, '读取配置中…');
  assert.equal(loadingConfig.kind, 'loading');

  const unconfigured = buildMonthlyGoalSyncView({
    today: '2026-08-25', goalCount: 2, configured: false,
  });
  assert.equal(unconfigured.buttonDisabled, true);
  assert.equal(unconfigured.buttonText, '先配置 Notion');
  assert.equal(unconfigured.headline, '先完成 Notion 配置');

  const empty = buildMonthlyGoalSyncView({
    today: '2026-08-25', goalCount: 0, configured: true,
  });
  assert.equal(empty.buttonDisabled, true);
  assert.equal(empty.buttonText, '暂无目标');
  assert.match(empty.detail, /不会发送 Notion 请求/);

  const loading = buildMonthlyGoalSyncView({
    today: '2026-08-25', goalCount: 2, configured: true, syncing: true,
  });
  assert.equal(loading.buttonDisabled, true);
  assert.equal(loading.buttonText, '同步中…');
  assert.equal(loading.kind, 'loading');
  assert.match(loading.headline, /正在校验/);

  const writing = buildMonthlyGoalSyncView({
    today: '2026-08-25', goalCount: 2, configured: true,
    syncing: true, syncStage: 'syncing',
  });
  assert.match(writing.headline, /正在同步/);
  assert.doesNotMatch(writing.headline, /正在校验/);

  const acrossMidnight = buildMonthlyGoalSyncView({
    today: '2026-09-01', goalCount: 2, configured: true,
    syncing: true, syncStage: 'syncing', syncMonth: '2026-08',
  });
  assert.equal(acrossMidnight.month, '2026-08');
  assert.match(acrossMidnight.headline, /2026 年 8 月/);
});

test('successful, partial, and failed attempts render stable counts and actionable errors', () => {
  const base = { today: '2026-08-25', goalCount: 2, configured: true };
  const success = buildMonthlyGoalSyncView({
    ...base,
    attempt: {
      kind: 'result', month: '2026-08',
      summary: { created: 2, updated: 0, failed: 0, errors: [] },
    },
  });
  assert.equal(success.kind, 'success');
  assert.match(success.headline, /新增 2 \/ 更新 0 \/ 失败 0/);

  const partial = buildMonthlyGoalSyncView({
    ...base,
    attempt: {
      kind: 'result', month: '2026-08',
      summary: {
        created: 1, updated: 0, failed: 1,
        errors: [{ goalTitle: '内参阅读', error: '写入权限不足' }],
      },
    },
  });
  assert.equal(partial.kind, 'partial');
  assert.deepEqual(partial.errors, ['“内参阅读”：写入权限不足']);

  const failed = buildMonthlyGoalSyncView({
    ...base,
    attempt: { kind: 'error', month: '2026-08', message: '当月页面不唯一' },
  });
  assert.equal(failed.kind, 'error');
  assert.equal(failed.detail, '当月页面不唯一');

  const allFailedResult = buildMonthlyGoalSyncView({
    ...base,
    attempt: {
      kind: 'result', month: '2026-08',
      summary: {
        created: 0, updated: 0, failed: 2,
        errors: [
          { goalTitle: '目标一', error: '没有写入权限' },
          { goalTitle: '目标二', error: '请求内容无效' },
        ],
      },
    },
  });
  assert.equal(allFailedResult.kind, 'error');
  assert.match(allFailedResult.headline, /新增 0 \/ 更新 0 \/ 失败 2/);
  assert.equal(allFailedResult.errors.length, 2);

  const becameEmpty = buildMonthlyGoalSyncView({
    ...base,
    attempt: {
      kind: 'result', month: '2026-08',
      summary: { empty: true, created: 0, updated: 0, failed: 0 },
    },
  });
  assert.equal(becameEmpty.kind, 'idle');
  assert.equal(becameEmpty.headline, '当前月没有可同步的目标');
});

test('latest monthly summary is selected by timestamp and contains only safe counts', () => {
  const latest = latestMonthlyGoalSummary({
    '2026-08-24': { at: '2026-08-24T10:00:00+08:00', created: 99 },
    'monthly:2026-07': {
      at: '2026-08-01T10:00:00+08:00', created: 3, updated: 1, failed: 0,
      token: 'must-not-surface', title: 'must-not-surface',
    },
    'monthly:2026-08': {
      at: '2026-08-25T10:00:00+08:00', created: 0, updated: 3, failed: 1,
      errors: [{ pageId: 'must-not-surface' }],
    },
  });
  assert.deepEqual(latest, {
    month: '2026-08',
    at: '2026-08-25T10:00:00+08:00',
    atMs: Date.parse('2026-08-25T10:00:00+08:00'),
    created: 0,
    updated: 3,
    failed: 1,
  });

  const view = buildMonthlyGoalSyncView({
    today: '2026-09-01', goalCount: 1, configured: true,
    exportLog: { 'monthly:2026-08': latest },
  });
  assert.equal(view.kind, 'partial');
  assert.match(view.headline, /2026 年 8 月/);
  assert.match(view.detail, /新增 0 \/ 更新 3 \/ 失败 1/);
  assert.doesNotMatch(JSON.stringify(view), /must-not-surface/);
});

test('an old-month in-memory attempt never replaces the current month status', () => {
  const view = buildMonthlyGoalSyncView({
    today: '2026-09-01', goalCount: 1, configured: true,
    attempt: {
      kind: 'result', month: '2026-08',
      summary: { created: 9, updated: 0, failed: 0 },
    },
  });
  assert.equal(view.headline, '2026 年 9 月 尚未同步');
  assert.doesNotMatch(view.headline, /新增 9/);
});

test('single-flight shares one in-flight promise and permits a later retry', async () => {
  let calls = 0;
  let release;
  const gate = new Promise((resolve) => { release = resolve; });
  const run = createSingleFlight(async () => {
    calls++;
    await gate;
    return calls;
  });

  const first = run();
  const second = run();
  assert.equal(first, second);
  assert.equal(calls, 0);
  release();
  assert.equal(await first, 1);
  assert.equal(calls, 1);
  assert.equal(await run(), 2);
  assert.equal(calls, 2);
});

test('single-flight clears a rejected operation so the next click can retry', async () => {
  let calls = 0;
  const run = createSingleFlight(async () => {
    calls++;
    if (calls === 1) throw new Error('temporary');
    return 'recovered';
  });
  await assert.rejects(run(), /temporary/);
  assert.equal(await run(), 'recovered');
  assert.equal(calls, 2);
});

test('mutation tracker waits only for saves already started at snapshot time', async () => {
  const tracker = createMutationTracker();
  let releaseFirst;
  let releaseLater;
  const first = new Promise((resolve) => { releaseFirst = resolve; });
  const later = new Promise((resolve) => { releaseLater = resolve; });
  tracker.track(first);

  const settling = tracker.settleCurrent();
  tracker.track(later);
  releaseFirst(true);
  assert.deepEqual(await settling, [true]);
  assert.equal(tracker.size, 1);

  releaseLater(false);
  assert.deepEqual(await tracker.settleCurrent(), [false]);
  assert.equal(tracker.size, 0);
});

test('goal row render key ignores progress and sync-only changes but tracks editable fields', () => {
  const base = {
    today: '2026-08-25',
    monthlyGoals: [{
      id: 'g1', title: '阅读', category: '学习', monthTarget: 30,
      monthDone: 2, checkedToday: false, locked: false,
    }],
  };
  const baseKey = monthlyGoalRowsRenderKey(base);
  assert.equal(monthlyGoalRowsRenderKey({
    ...base,
    monthlyGoals: [{ ...base.monthlyGoals[0], monthDone: 9, checkedToday: true, locked: true }],
    notionExportLog: { 'monthly:2026-08': { created: 1 } },
  }), baseKey);
  assert.notEqual(monthlyGoalRowsRenderKey({
    ...base,
    monthlyGoals: [{ ...base.monthlyGoals[0], title: '新标题' }],
  }), baseKey);
  assert.notEqual(monthlyGoalRowsRenderKey({
    ...base,
    monthlyGoals: [{ ...base.monthlyGoals[0], id: 'g2' }],
  }), baseKey);
  assert.notEqual(monthlyGoalRowsRenderKey({
    ...base,
    monthlyGoals: [{ ...base.monthlyGoals[0], category: '工作' }],
  }), baseKey);
  assert.notEqual(monthlyGoalRowsRenderKey({
    ...base,
    monthlyGoals: [{ ...base.monthlyGoals[0], monthTarget: 31 }],
  }), baseKey);
  assert.notEqual(monthlyGoalRowsRenderKey({ ...base, today: '2026-09-01' }), baseKey);
});

test('latest-success gate keeps a valid older response when a newer request fails', () => {
  const firstGate = createLatestSuccessGate();
  const older = firstGate.issue();
  const newer = firstGate.issue();
  assert.equal(firstGate.accept(newer), true);
  assert.equal(firstGate.accept(older), false);

  const failedNewerGate = createLatestSuccessGate();
  const olderSuccess = failedNewerGate.issue();
  failedNewerGate.issue(); // 新请求失败时不会调用 accept。
  assert.equal(failedNewerGate.accept(olderSuccess), true);
});
