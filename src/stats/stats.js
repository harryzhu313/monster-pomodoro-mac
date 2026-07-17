// 统计与设置窗口:渲染逻辑平移旧 options.js。
// 数据来源:get_state 快照(settings/last7Days) + get_stats_bundle(整年 stats/归档/徽章)。
// 设置改动 → update_settings command;历史完成态 → set_history_task_done。

const tauri = window.__TAURI__ ?? null;

async function invoke(cmd, args) {
  if (!tauri) {
    console.log('[mock invoke]', cmd, args ?? '');
    return null;
  }
  return tauri.core.invoke(cmd, args);
}

const els = {
  chime: document.getElementById('chime'),
  whiteNoise: document.getElementById('white-noise'),
  lastMinute: document.getElementById('last-minute'),
  longBreakEnabled: document.getElementById('long-break-enabled'),
  longBreakEvery: document.getElementById('long-break-every'),
  longBreakMinutes: document.getElementById('long-break-minutes'),
  themeSelect: document.getElementById('theme-select'),
  autoStart: document.getElementById('auto-start'),
  chart: document.getElementById('chart'),
  statCurrent: document.getElementById('stat-current'),
  statLongest: document.getElementById('stat-longest'),
  statTotal: document.getElementById('stat-total'),
  btnClearToday: document.getElementById('btn-clear-today'),
  historyList: document.getElementById('history-list'),
  historyCard: document.getElementById('history-card'),
  historyPanel: document.getElementById('history-panel'),
  historyMeta: document.getElementById('history-meta'),
  btnToggleHistory: document.getElementById('btn-toggle-history'),
  badgesCount: document.getElementById('badges-count'),
  badgesStreak: document.getElementById('badges-streak'),
  badgesSlots: document.getElementById('badges-slots'),
  heatmapGrid: document.getElementById('heatmap-grid'),
  heatmapMonths: document.getElementById('heatmap-months'),
  heatmapMeta: document.getElementById('heatmap-meta'),
};

const HISTORY_MAX_DAYS = 30;
const HISTORY_COLLAPSED_KEY = 'historyCollapsed';
const BADGES_SLOT_TOTAL = 52;
const LOVE_MONSTER_URL = '../shared/monsters/love.svg';

let bundle = null; // get_stats_bundle 结果
let snap = null;   // get_state 快照
let lastTodayStatsJson = '';

// —— 设置区(平移旧 renderSettings/patchSettings)——

function renderSettings(s) {
  els.chime.checked = !!s.chimeEnabled;
  els.whiteNoise.checked = !!s.whiteNoiseEnabled;
  els.lastMinute.checked = !!s.lastMinuteEnabled;
  els.longBreakEnabled.checked = !!s.longBreakEnabled;
  els.longBreakEvery.value = String(s.longBreakEvery);
  els.longBreakEvery.disabled = !s.longBreakEnabled;
  els.longBreakMinutes.value = String(s.longBreakMinutes);
  els.longBreakMinutes.disabled = !s.longBreakEnabled;
  els.themeSelect.value = s.theme;
  els.autoStart.checked = !!s.autoStartNextFocus;
}

function patch(patchObj) {
  return invoke('update_settings', { patch: patchObj });
}

els.chime.addEventListener('change', () => patch({ chimeEnabled: els.chime.checked }));
els.whiteNoise.addEventListener('change', () => patch({ whiteNoiseEnabled: els.whiteNoise.checked }));
els.lastMinute.addEventListener('change', () => patch({ lastMinuteEnabled: els.lastMinute.checked }));
els.longBreakEnabled.addEventListener('change', () => {
  els.longBreakEvery.disabled = !els.longBreakEnabled.checked;
  els.longBreakMinutes.disabled = !els.longBreakEnabled.checked;
  patch({ longBreakEnabled: els.longBreakEnabled.checked });
});
els.longBreakEvery.addEventListener('change', () =>
  patch({ longBreakEvery: Number(els.longBreakEvery.value) })
);
els.longBreakMinutes.addEventListener('change', () =>
  patch({ longBreakMinutes: Number(els.longBreakMinutes.value) })
);
els.themeSelect.addEventListener('change', () => patch({ theme: els.themeSelect.value }));
els.autoStart.addEventListener('change', () => patch({ autoStartNextFocus: els.autoStart.checked }));

// —— 7 天柱状图 + 连续/累计(平移旧 renderChart/computeStreaks)——

function renderChart(days) {
  const max = Math.max(1, ...days.map((d) => d.completed));
  els.chart.innerHTML = '';
  days.forEach((d, idx) => {
    const isToday = idx === days.length - 1;
    const heightPct = (d.completed / max) * 100;
    const rottenHtml =
      d.rotten > 0 ? `<div class="chart-rotten" title="烂番茄(放弃的专注)">🤪${d.rotten}</div>` : '';
    const bar = document.createElement('div');
    bar.className = 'chart-day' + (isToday ? ' is-today' : '');
    bar.innerHTML = `
      <div class="chart-count">${d.completed}</div>
      <div class="chart-bar-wrap">
        <div class="chart-bar ${d.completed > 0 ? 'has-data' : ''} ${isToday ? 'today' : ''}"
             style="height: ${heightPct}%"></div>
      </div>
      <div class="chart-date">${d.date.slice(5)}</div>
      ${rottenHtml}
    `;
    els.chart.appendChild(bar);
  });
}

function computeStreaks(days) {
  const total = days.reduce((s, d) => s + d.completed, 0);
  let current = 0;
  let startIdx = days.length - 1;
  if (startIdx >= 0 && days[startIdx].completed === 0) startIdx--;
  for (let i = startIdx; i >= 0; i--) {
    if (days[i].completed > 0) current++;
    else break;
  }
  let longest = 0;
  let run = 0;
  for (const d of days) {
    if (d.completed > 0) {
      run++;
      longest = Math.max(longest, run);
    } else {
      run = 0;
    }
  }
  return { current, longest, total };
}

function renderSevenDays() {
  const days = snap.last7Days || [];
  renderChart(days);
  const { current, longest, total } = computeStreaks(days);
  els.statCurrent.textContent = current;
  els.statLongest.textContent = longest;
  els.statTotal.textContent = total;
}

// —— 年度热力图(平移旧 refreshHeatmap:53 周 × 7 天,列=周,起点 52 周前的周日)——

function heatmapLevel(count) {
  if (count <= 0) return 0;
  if (count <= 6) return 1;
  if (count <= 12) return 2;
  return 3;
}

function isoDateOf(d) {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

function renderHeatmap() {
  const stats = bundle.stats || {};
  const today = new Date();
  today.setHours(0, 0, 0, 0);

  const end = new Date(today);
  end.setDate(end.getDate() + (6 - end.getDay()));
  const start = new Date(end);
  start.setDate(start.getDate() - (53 * 7 - 1));

  const cells = [];
  const monthMarks = [];
  let lastMonth = -1;
  let totalPomodoros = 0;
  let activeDays = 0;

  for (let col = 0; col < 53; col++) {
    for (let row = 0; row < 7; row++) {
      const d = new Date(start);
      d.setDate(start.getDate() + col * 7 + row);
      const iso = isoDateOf(d);
      const count = stats[iso]?.completed || 0;
      const future = d > today;
      cells.push({ col, row, date: iso, count, future });
      if (!future && count > 0) {
        totalPomodoros += count;
        activeDays += 1;
      }
    }
    const colStart = new Date(start);
    colStart.setDate(start.getDate() + col * 7);
    const m = colStart.getMonth();
    if (m !== lastMonth) {
      monthMarks.push({ col, label: `${m + 1}月` });
      lastMonth = m;
    }
  }

  els.heatmapGrid.innerHTML = cells
    .map((c) => {
      if (c.future) {
        return `<i class="heatmap-cell is-future" style="grid-column:${c.col + 1};grid-row:${c.row + 1};"></i>`;
      }
      const lv = heatmapLevel(c.count);
      const title = c.count === 0 ? `${c.date}:没有番茄` : `${c.date}:${c.count} 🍅`;
      return `<i class="heatmap-cell lv-${lv}" style="grid-column:${c.col + 1};grid-row:${c.row + 1};" title="${title}"></i>`;
    })
    .join('');

  els.heatmapMonths.innerHTML = monthMarks
    .map((m) => `<span style="grid-column:${m.col + 1};">${m.label}</span>`)
    .join('');

  els.heatmapMeta.textContent = `累计 ${totalPomodoros} 颗 · ${activeDays} 个活跃日`;
}

// —— 徽章墙(平移旧 refreshBadges/renderBadgeSlots:近 365 天解锁数点亮 52 格)——

function countUnlockedThisYear(unlockedDates) {
  if (!Array.isArray(unlockedDates)) return 0;
  const cutoff = new Date();
  cutoff.setDate(cutoff.getDate() - 365);
  const cutoffIso = isoDateOf(cutoff);
  return unlockedDates.filter((d) => d >= cutoffIso).length;
}

function renderBadges() {
  const b = bundle.badges;
  const goal = b.goal || 7;
  const cur = Math.max(0, Math.min(goal, b.currentStreak || 0));
  const unlockedThisYear = countUnlockedThisYear(b.unlockedDates);
  els.badgesCount.textContent = unlockedThisYear;
  els.badgesStreak.textContent = cur;
  const filled = Math.min(BADGES_SLOT_TOTAL, unlockedThisYear);
  const parts = [];
  for (let i = 0; i < BADGES_SLOT_TOTAL; i++) {
    const on = i < filled;
    parts.push(
      `<span class="badge-slot${on ? ' is-on' : ''}" title="第 ${i + 1} 枚 · ${on ? '已解锁' : '未解锁'}"><img src="${LOVE_MONSTER_URL}" alt="" /></span>`
    );
  }
  els.badgesSlots.innerHTML = parts.join('');
}

// —— 历史明细(平移旧 renderHistoryDay/renderHistoryTask/effectiveDoneState)——

function effectiveDoneState(task) {
  const planned = Number(task.planned) || 0;
  const used = Number(task.used) || 0;
  if (typeof task.doneOverride === 'boolean') {
    return { done: task.doneOverride, isAuto: false };
  }
  if (task.done === true) {
    return { done: true, isAuto: false };
  }
  if (planned > 0 && used >= planned) {
    return { done: true, isAuto: true };
  }
  return { done: false, isAuto: false };
}

function escapeHtml(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

function renderHistoryTask(task, date) {
  const planned = Number(task.planned) || 0;
  const used = Number(task.used) || 0;
  const over = used > planned;
  const { done, isAuto } = effectiveDoneState(task);
  const statusClass = 'history-task-status ' + (done ? 'done' : 'undone') + (isAuto ? ' is-auto' : '');
  const statusTitle = isAuto ? '自动推断为完成(实际 ≥ 计划)' : done ? '已完成' : '未完成';
  const numsHtml = over
    ? `计划 ${planned} · 实际 <span class="used over">${used}</span>(超 ${used - planned})`
    : `计划 ${planned} · 实际 <span class="used">${used}</span>`;
  const doneActive = done && !isAuto;
  const idAttr = escapeHtml(JSON.stringify(task.id));
  return `
    <div class="history-task${done ? ' is-done' : ''}">
      <span class="${statusClass}" aria-hidden="true" title="${statusTitle}">${done ? '✓' : '✗'}</span>
      <span class="history-task-title">${escapeHtml(task.title || '(未命名)')}</span>
      <span class="history-task-nums">${numsHtml}</span>
      <div class="history-task-actions">
        <button type="button" class="history-task-btn${doneActive ? ' is-active done' : ''}"
                data-date="${date}" data-task-id="${idAttr}" data-action="done">完成</button>
        <button type="button" class="history-task-btn${!done ? ' is-active undone' : ''}"
                data-date="${date}" data-task-id="${idAttr}" data-action="undone">未完成</button>
      </div>
    </div>
  `;
}

function renderHistoryDay(date, tasks, totalToday, isToday) {
  const plannedTotal = tasks.reduce((s, t) => s + (t.planned || 0), 0);
  const usedTotal = tasks.reduce((s, t) => s + (t.used || 0), 0);
  const doneCount = tasks.filter((t) => effectiveDoneState(t).done).length;
  const extra = Math.max(0, (totalToday || 0) - usedTotal);
  const exportLog = (bundle.notionExportLog || {})[date];
  const exportedClass = exportLog && exportLog.ok ? ' is-exported' : '';
  const exportLabel = exportLog && exportLog.ok ? `已导入 ${exportLog.created}` : '导入到 Notion';
  const exportBtnHtml = tasks.length
    ? `<button type="button" class="history-day-export${exportedClass}" data-date="${date}" disabled title="Notion 功能即将接入(phase 06)">${exportLabel}</button>`
    : '';
  const tasksHtml = tasks.length
    ? tasks.map((t) => renderHistoryTask(t, date)).join('')
    : '<div class="history-empty" style="padding:6px 0;">这一天没有记录任务。</div>';
  const extraHtml = extra > 0
    ? `<div class="history-extra">计划外番茄 ${extra} 个(未归到任何任务)</div>`
    : '';
  return `
    <div class="history-day${isToday ? ' is-open' : ''}" data-date="${date}">
      <div class="history-day-header">
        <div class="history-day-date">${date}${isToday ? '<span class="today-tag">今天</span>' : ''}</div>
        <div class="history-day-summary">
          <span>${doneCount}/${tasks.length} 完成</span>
          <span class="sep">·</span>
          <span>计划 ${plannedTotal} · 实际 <span class="${usedTotal > plannedTotal ? 'overflow' : ''}">${usedTotal}</span></span>
          ${exportBtnHtml}
        </div>
      </div>
      <div class="history-day-body">
        ${tasksHtml}
        ${extraHtml}
      </div>
    </div>
  `;
}

function renderHistory() {
  const today = bundle.today;
  const byDate = { ...bundle.tasksArchive };
  byDate[today] = bundle.tasksToday.tasks || [];

  const dates = Object.keys(byDate).sort().reverse().slice(0, HISTORY_MAX_DAYS);
  const stats = bundle.stats || {};

  if (dates.every((d) => (byDate[d] || []).length === 0 && !stats[d])) {
    els.historyList.innerHTML =
      '<div class="history-empty">还没有数据,规划一下今天的三件事,开始第一个番茄吧。</div>';
    els.historyMeta.textContent = '';
    return;
  }

  els.historyMeta.textContent = `共 ${dates.length} 天`;
  els.historyList.innerHTML = dates
    .map((d) => renderHistoryDay(d, byDate[d] || [], stats[d]?.completed || 0, d === today))
    .join('');
}

els.historyList.addEventListener('click', async (e) => {
  const btn = e.target.closest('.history-task-btn');
  if (btn) {
    e.stopPropagation();
    const { date, taskId, action } = btn.dataset;
    if (date && taskId && action) {
      await invoke('set_history_task_done', {
        date,
        id: JSON.parse(taskId),
        done: action === 'done',
      });
      await refreshBundle();
    }
    return;
  }
  const header = e.target.closest('.history-day-header');
  if (header?.parentElement) header.parentElement.classList.toggle('is-open');
});

function setHistoryCollapsed(collapsed) {
  els.historyCard.classList.toggle('is-collapsed', collapsed);
  els.historyPanel.hidden = collapsed;
  els.btnToggleHistory.textContent = collapsed ? '展开' : '收起';
  els.btnToggleHistory.setAttribute('aria-expanded', String(!collapsed));
  localStorage.setItem(HISTORY_COLLAPSED_KEY, collapsed ? '1' : '0');
}

els.btnToggleHistory.addEventListener('click', () => {
  setHistoryCollapsed(!els.historyCard.classList.contains('is-collapsed'));
});

// —— 清零今日(两步确认,平移旧实现)——

let clearConfirmTimer = null;

els.btnClearToday.addEventListener('click', async () => {
  if (els.btnClearToday.classList.contains('confirming')) {
    clearTimeout(clearConfirmTimer);
    els.btnClearToday.classList.remove('confirming');
    els.btnClearToday.textContent = '清零今日';
    await invoke('clear_today_stats');
    await refreshAll();
    return;
  }
  els.btnClearToday.classList.add('confirming');
  els.btnClearToday.textContent = '再点一次确认';
  clearConfirmTimer = setTimeout(() => {
    els.btnClearToday.classList.remove('confirming');
    els.btnClearToday.textContent = '清零今日';
  }, 3000);
});

// —— 刷新编排 ——

async function refreshBundle() {
  bundle = (await invoke('get_stats_bundle')) ?? MOCK_BUNDLE;
  renderBadges();
  renderHeatmap();
  renderHistory();
}

async function refreshAll() {
  snap = (await invoke('get_state')) ?? MOCK_SNAP;
  renderSettings(snap.settings);
  renderSevenDays();
  await refreshBundle();
}

// —— mock(浏览器直开预览)——

const MOCK_SNAP = {
  settings: { chimeEnabled: true, whiteNoiseEnabled: true, lastMinuteEnabled: true,
    longBreakEnabled: true, longBreakEvery: 4, longBreakMinutes: 20,
    theme: 'monster', autoStartNextFocus: true },
  last7Days: [
    { date: '2026-01-01', completed: 3, rotten: 0 }, { date: '2026-01-02', completed: 6, rotten: 1 },
    { date: '2026-01-03', completed: 0, rotten: 0 }, { date: '2026-01-04', completed: 8, rotten: 0 },
    { date: '2026-01-05', completed: 4, rotten: 2 }, { date: '2026-01-06', completed: 7, rotten: 0 },
    { date: '2026-01-07', completed: 2, rotten: 0 },
  ],
};
const MOCK_BUNDLE = {
  today: '2026-01-07',
  stats: { '2026-01-02': { completed: 6, rotten: 1 }, '2026-01-04': { completed: 8, rotten: 0 },
    '2026-01-05': { completed: 14, rotten: 2 }, '2026-01-06': { completed: 7, rotten: 0 } },
  tasksArchive: { '2026-01-06': [
    { id: 'a1', title: '内参阅读', planned: 4, used: 5, done: false, category: '学习' },
    { id: 'a2', title: '回邮件', planned: 1, used: 1, done: true, doneOverride: true, category: '工作' },
  ] },
  tasksToday: { date: '2026-01-07', tasks: [
    { id: 't1', title: '写周报', planned: 2, used: 1, done: false, category: '工作' },
  ] },
  badges: { badges: 3, currentStreak: 5, goal: 7,
    unlockedDates: ['2025-11-02', '2025-12-14', '2026-01-01'] },
  notionExportLog: {},
};

// —— 启动 ——

if (tauri) {
  // 快照每秒广播;只有今日数据/设置变化时才重刷,避免热力图频繁重排
  tauri.event.listen('state-update', (e) => {
    const next = e.payload;
    const key = JSON.stringify([next.todayStats, next.settings, next.tasksToday]);
    if (key !== lastTodayStatsJson) {
      lastTodayStatsJson = key;
      snap = next;
      renderSettings(snap.settings);
      renderSevenDays();
      refreshBundle();
    }
  });
}

setHistoryCollapsed(localStorage.getItem(HISTORY_COLLAPSED_KEY) === '1');
refreshAll();
