// 主面板:渲染 + 把点击翻译成 command。行为平移旧 popup.js。
// 前端零计时逻辑:一切状态来自 Rust 的 state-update 快照(每秒一拍),
// 剩余时间用快照自带的 now 计算,本地时钟不参与(AGENTS 规则 6)。

import {
  formatMs,
  computeRemaining,
  normalizeCategory,
  MONSTER_BY_STATE,
} from '../shared/util.js';

const MAX_HARVEST_ICONS = 12; // 今日收获超过就用 +N 折叠(旧 popup.js)
const MAX_TOMATO_ICONS = 8;   // 单任务计划超过就用 +N 折叠

// —— Tauri IPC(浏览器直开 panel.html 时降级为 mock,方便开发预览)——

const tauri = window.__TAURI__ ?? null;

async function invoke(cmd, args) {
  if (!tauri) {
    console.log('[mock invoke]', cmd, args ?? '');
    return null;
  }
  return tauri.core.invoke(cmd, args);
}

const MOCK_SNAPSHOT = {
  timer: { state: 'IDLE', phase: null, breakKind: null, endTime: null,
    pausedRemaining: null, focusStartedAt: null },
  quota: { used: 0, limit: 3, remaining: 3 },
  settings: { theme: 'monster', autoStartNextFocus: true },
  durations: { focusMs: 25 * 60 * 1000, breakMs: 5 * 60 * 1000, testMode: false },
  today: '2026-01-01',
  todayStats: { completed: 3, rotten: 1 },
  badges: { badges: 0, currentStreak: 2, goal: 7 },
  tasksToday: { date: '2026-01-01', tasks: [
    { id: 'mock1', title: '内参阅读', category: '学习', planned: 4, used: 2, rotten: 0, done: false, isCurrent: true },
    { id: 'mock2', title: '回邮件', category: '工作', planned: 1, used: 1, rotten: 0, done: true, isCurrent: false },
  ] },
  last7Days: [],
  lastCategory: '学习',
  now: Date.now(),
};

const els = {
  phaseLabel: document.getElementById('phase-label'),
  timer: document.getElementById('timer'),
  btnPrimary: document.getElementById('btn-primary'),
  btnAbandon: document.getElementById('btn-abandon'),
  btnReset: document.getElementById('btn-reset'),
  breakSubtitle: document.getElementById('break-subtitle'),
  extend: document.getElementById('extend'),
  extTitle: document.getElementById('ext-title'),
  extOptions: document.getElementById('ext-options'),
  extCustom: document.getElementById('ext-custom'),
  extCustomBtn: document.getElementById('ext-custom-btn'),
  extInput: document.getElementById('ext-input'),
  extConfirm: document.getElementById('ext-confirm'),
  extCancel: document.getElementById('ext-cancel'),
  extNote: document.getElementById('ext-note'),
  quota: document.getElementById('quota'),
  hint: document.getElementById('hint'),
  btnStats: document.getElementById('btn-stats'),
  monster: document.getElementById('monster'),
  harvestIcons: document.getElementById('harvest-icons'),
  streak: document.getElementById('streak'),
  tasksCount: document.getElementById('tasks-count'),
  taskAddForm: document.getElementById('task-add-form'),
  taskInput: document.getElementById('task-input'),
  taskCategorySelect: document.getElementById('task-category'),
  taskPlannedInput: document.getElementById('task-planned'),
  taskList: document.getElementById('task-list'),
  taskEmpty: document.getElementById('task-empty'),
  onboarding: document.getElementById('onboarding'),
  obNote: document.getElementById('ob-note'),
  obFile: document.getElementById('ob-file'),
};

let snap = null;
let lastTasksJson = '';   // 任务列表只在内容变化时重建,避免每秒快照打断输入/焦点
let lastHarvestJson = '';
let categoryInitialized = false;

// —— 计时卡(平移旧 renderTimer)——

function renderTimer() {
  const t = snap.timer;
  const { state, phase } = t;
  const isLongBreak = t.breakKind === 'long';
  const isBreakPhase = state === 'BREAKING' || (state === 'PAUSED' && phase === 'break');

  els.timer.textContent = formatMs(computeRemaining(snap));
  els.phaseLabel.className = 'phase-label';

  // 加时区块与副标题只在休息中出现;离开休息态时收起自定义输入、清提示
  const breaking = state === 'BREAKING';
  els.extend.classList.toggle('is-hidden', !breaking);
  els.breakSubtitle.classList.toggle('is-hidden', !breaking);
  if (!breaking && !els.extCustom.classList.contains('is-hidden')) {
    els.extCustom.classList.add('is-hidden');
    els.extOptions.classList.remove('is-hidden');
    els.extNote.textContent = '';
  }
  if (breaking) {
    // monster 主题用后端挑好的随机句(与系统通知同一句);default 主题用静态文案
    els.breakSubtitle.textContent =
      snap.settings.theme === 'monster'
        ? (isLongBreak ? '这是长休息,真的离开屏幕一会儿。' : t.breakSubtitle || '')
        : (isLongBreak
            ? '这是长休息。站起来,走远一点,让眼睛和身体都缓过来。'
            : '站起来,离开屏幕。看看窗外,喝口水。');
    const exhausted = snap.quota.remaining <= 0;
    els.extTitle.textContent = exhausted ? '今日配额已用完' : '还没做完？用一次配额再专注一会';
    els.extOptions.querySelectorAll('.ext-btn').forEach((b) => (b.disabled = exhausted));
    els.extConfirm.disabled = exhausted;
  }

  if (state === 'IDLE') {
    els.phaseLabel.textContent = '准备开始';
    els.btnPrimary.textContent = '开始专注';
    els.btnPrimary.dataset.action = 'start';
    els.btnPrimary.disabled = false;
    els.btnAbandon.disabled = true;
    els.btnReset.disabled = false;
    els.hint.textContent = '按时停下来,比多做一轮重要。';
  } else if (state === 'FOCUSING') {
    els.phaseLabel.textContent = '专注中';
    els.phaseLabel.classList.add('focusing');
    els.btnPrimary.textContent = '暂停';
    els.btnPrimary.dataset.action = 'pause';
    els.btnPrimary.disabled = false;
    els.btnAbandon.disabled = false;
    els.btnReset.disabled = false;
    const inGrace = t.focusStartedAt && snap.now - t.focusStartedAt < 10 * 1000;
    els.hint.textContent = inGrace
      ? '前 10 秒可反悔:放弃不计入烂番茄。'
      : '一次只做一件事。';
  } else if (state === 'BREAKING') {
    els.phaseLabel.textContent = isLongBreak ? '长休息中' : '休息中';
    els.phaseLabel.classList.add('breaking');
    els.btnPrimary.textContent = '休息锁定中';
    els.btnPrimary.dataset.action = '';
    els.btnPrimary.disabled = true;
    els.btnAbandon.disabled = true;
    els.btnReset.disabled = true;
    // 旧文案指向锁屏,macOS 版无锁屏,改述(待用户验收确认)
    els.hint.textContent = isLongBreak
      ? '这是长休息,真的离开屏幕一会儿。'
      : '起身休息一下;要继续就加时(扣配额)。';
  } else if (state === 'PAUSED') {
    els.phaseLabel.textContent =
      phase === 'focus' ? '专注已暂停' : isLongBreak ? '长休息已暂停' : '休息已暂停';
    els.phaseLabel.classList.add('paused');
    els.btnPrimary.textContent = '继续';
    els.btnPrimary.dataset.action = 'resume';
    els.btnPrimary.disabled = false;
    els.btnAbandon.disabled = isBreakPhase;
    els.btnReset.disabled = isBreakPhase;
    els.hint.textContent = '暂停时间不计入计时。';
  }

  const { remaining, limit } = snap.quota;
  els.quota.textContent = `今日剩 ${remaining}/${limit}`;
  els.quota.classList.toggle('exhausted', remaining <= 0);

  document.body.classList.toggle('theme-monster', snap.settings.theme === 'monster');
  if (snap.settings.theme === 'monster') {
    const kind = MONSTER_BY_STATE[state] || 'happy';
    const src = `../shared/monsters/${kind}.svg`;
    if (!els.monster.src.endsWith(src.slice(2))) els.monster.src = src;
  } else {
    els.monster.removeAttribute('src');
  }
}

// —— 今日收获 + 连续天数(平移旧 renderHarvestAndStreak)——

function renderHarvest() {
  const key = JSON.stringify([snap.todayStats, snap.last7Days]);
  if (key === lastHarvestJson) return;
  lastHarvestJson = key;

  const { completed, rotten } = snap.todayStats;
  els.harvestIcons.innerHTML = '';
  if (completed === 0 && rotten === 0) {
    const empty = document.createElement('span');
    empty.className = 'harvest-empty';
    empty.textContent = '还没收获,先来一个吧。';
    els.harvestIcons.appendChild(empty);
  } else {
    const addIcons = (count, className, title) => {
      const shown = Math.min(count, MAX_HARVEST_ICONS);
      for (let i = 0; i < shown; i++) {
        const span = document.createElement('span');
        if (className) span.className = className;
        if (title) span.title = title;
        span.textContent = '🍅';
        els.harvestIcons.appendChild(span);
      }
      if (count > MAX_HARVEST_ICONS) {
        const more = document.createElement('span');
        more.className = 'harvest-empty';
        more.style.marginLeft = '4px';
        more.textContent = `+${count - MAX_HARVEST_ICONS}`;
        els.harvestIcons.appendChild(more);
      }
    };
    addIcons(completed, '', '');
    if (rotten > 0) addIcons(rotten, 'rotten', '烂番茄(放弃的专注)');
  }

  // 连续天数只算"有完成"的天;今天若还空着则按"截至昨天"展示(旧 popup.js:242-252)
  const days = snap.last7Days || [];
  let current = 0;
  let i = days.length - 1;
  if (i >= 0 && days[i].completed === 0) i--;
  for (; i >= 0; i--) {
    if (days[i].completed > 0) current++;
    else break;
  }
  els.streak.textContent = `🔥 已连续 ${current} 天`;
}

// —— 今日任务(平移旧 renderTasks/renderTaskTomatoes)——

function renderTaskTomatoes(task) {
  const wrap = document.createElement('div');
  wrap.className = 'task-tomatoes';
  const plannedDisplay = Math.min(task.planned, MAX_TOMATO_ICONS);
  for (let i = 0; i < plannedDisplay; i++) {
    const s = document.createElement('span');
    s.className = 'tomato' + (i < task.used ? ' used' : '');
    s.textContent = '🍅';
    wrap.appendChild(s);
  }
  if (task.planned > MAX_TOMATO_ICONS) {
    const more = document.createElement('span');
    more.className = 'tomato overflow';
    more.textContent = `+${task.planned - MAX_TOMATO_ICONS}`;
    wrap.appendChild(more);
  }
  if (task.used > task.planned) {
    const over = document.createElement('span');
    over.className = 'tomato overflow';
    over.textContent = `超${task.used - task.planned}`;
    wrap.appendChild(over);
  }
  return wrap;
}

function renderTasks() {
  const tasks = snap.tasksToday.tasks || [];
  const key = JSON.stringify(tasks);
  if (key === lastTasksJson) return;
  lastTasksJson = key;

  els.taskList.innerHTML = '';
  const done = tasks.filter((t) => t.done).length;
  els.tasksCount.textContent = `${done} 个完成`;
  els.taskEmpty.classList.toggle('is-hidden', tasks.length > 0);

  for (const t of tasks) {
    const li = document.createElement('li');
    li.className = 'task-item';
    if (t.isCurrent && !t.done) li.classList.add('is-current');
    if (t.done) li.classList.add('is-done');

    const cb = document.createElement('input');
    cb.type = 'checkbox';
    cb.className = 'task-checkbox';
    cb.checked = !!t.done;
    cb.setAttribute('aria-label', `完成 ${t.title}`);
    cb.addEventListener('change', () => invoke('set_task_done', { id: t.id, done: cb.checked }));

    const title = document.createElement('span');
    title.className = 'task-title';
    title.textContent = t.title;

    const cat = normalizeCategory(t.category);
    const badge = document.createElement('span');
    const catClass =
      cat === '工作' ? 'cat-work' :
      cat === '学习' ? 'cat-study' :
      cat === '生活' ? 'cat-life' : 'cat-hobby';
    badge.className = `task-category-badge ${catClass}`;
    badge.textContent = cat;
    title.appendChild(badge);

    const current = document.createElement('button');
    current.type = 'button';
    current.className = 'task-btn-current';
    current.textContent = t.isCurrent && !t.done ? '当前任务' : '设为当前';
    current.disabled = !!t.done;
    if (!(t.isCurrent && !t.done)) {
      current.addEventListener('click', () => invoke('set_current_task', { id: t.id }));
    }

    const del = document.createElement('button');
    del.type = 'button';
    del.className = 'task-btn-del';
    del.textContent = '×';
    del.title = '删除';
    del.setAttribute('aria-label', `删除 ${t.title}`);
    del.addEventListener('click', () => invoke('delete_task', { id: t.id }));

    li.append(cb, title, renderTaskTomatoes(t), current, del);
    els.taskList.appendChild(li);
  }
}

function render(next) {
  snap = next;
  renderTimer();
  renderHarvest();
  renderTasks();
  els.onboarding.classList.toggle('is-hidden', snap.onboardingDone !== false);
  if (!categoryInitialized && snap.lastCategory) {
    els.taskCategorySelect.value = normalizeCategory(snap.lastCategory);
    categoryInitialized = true;
  }
}

// —— 首次启动引导(三步:导入 → 自启 → 快捷键) ——

function showObStep(n) {
  for (const i of [1, 2, 3]) {
    document.getElementById(`ob-step-${i}`).classList.toggle('is-hidden', i !== n);
  }
}

document.getElementById('ob-import').addEventListener('click', () => els.obFile.click());

els.obFile.addEventListener('change', async () => {
  const file = els.obFile.files?.[0];
  els.obFile.value = '';
  if (!file) return;
  const r = await invoke('import_backup', { json: await file.text() });
  if (r?.ok) {
    els.obNote.textContent = `导入完成,${r.days} 天的历史已接上,连击和热力图都在。`;
    setTimeout(() => showObStep(2), 900);
  } else {
    els.obNote.textContent = `导入失败,现有数据没有被改动。原因:${r?.error || '未知'}。`;
  }
});

document.getElementById('ob-fresh').addEventListener('click', () => showObStep(2));
document.getElementById('ob-autostart-yes').addEventListener('click', async () => {
  await invoke('set_autostart', { enabled: true });
  showObStep(3);
});
document.getElementById('ob-autostart-no').addEventListener('click', () => showObStep(3));
document.getElementById('ob-done').addEventListener('click', () => invoke('finish_onboarding'));

// —— 事件绑定 ——

els.btnPrimary.addEventListener('click', () => {
  const action = els.btnPrimary.dataset.action;
  if (action) invoke(action);
});
els.btnAbandon.addEventListener('click', () => invoke('abandon'));
els.btnReset.addEventListener('click', () => invoke('reset'));
els.btnStats.addEventListener('click', () => invoke('open_stats'));
els.quota.addEventListener('dblclick', () => invoke('reset_quota'));

// —— 加时(平移旧 lockscreen.js claim 逻辑)——
// 预设按钮:TEST_MODE 下分钟当秒算,方便快测;自定义输入始终按真·分钟换算。

function minutesToMs(min) {
  return snap?.durations?.testMode ? min * 1000 : min * 60 * 1000;
}

async function claim(ms) {
  if (!Number.isFinite(ms) || ms <= 0) {
    els.extNote.textContent = '无效的时长。';
    return;
  }
  const resp = await invoke('overtime', { ms });
  if (resp && !resp.ok) {
    els.extNote.textContent =
      resp.reason === 'quota-exhausted' ? '今日配额已用完。' : '无法加时。';
  }
  // 成功时 state-update 广播会把面板切回专注态,无需手动处理
}

els.extOptions.querySelectorAll('[data-min]').forEach((btn) => {
  btn.addEventListener('click', () => {
    if (btn.disabled) return;
    claim(minutesToMs(Number(btn.dataset.min)));
  });
});

els.extCustomBtn.addEventListener('click', () => {
  if (els.extCustomBtn.disabled) return;
  els.extOptions.classList.add('is-hidden');
  els.extCustom.classList.remove('is-hidden');
  els.extInput.focus();
});

els.extCancel.addEventListener('click', () => {
  els.extCustom.classList.add('is-hidden');
  els.extOptions.classList.remove('is-hidden');
  els.extNote.textContent = '';
});

els.extConfirm.addEventListener('click', () => {
  if (els.extConfirm.disabled) return;
  claim(Number(els.extInput.value) * 60 * 1000);
});

els.extInput.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') {
    e.preventDefault();
    els.extConfirm.click();
  }
});

els.taskAddForm.addEventListener('submit', (e) => {
  e.preventDefault();
  const title = els.taskInput.value;
  if (!title.trim()) return;
  invoke('add_task', {
    title,
    category: els.taskCategorySelect.value,
    planned: Number(els.taskPlannedInput.value) || 1,
  });
  els.taskInput.value = '';
  els.taskPlannedInput.value = '1';
  // 分类保持不变,方便连续添加同类任务(旧行为)
  els.taskInput.focus();
});

// —— 启动 ——

if (tauri) {
  tauri.event.listen('state-update', (e) => render(e.payload));
  invoke('get_state').then((s) => s && render(s));
} else {
  render(MOCK_SNAPSHOT);
}
