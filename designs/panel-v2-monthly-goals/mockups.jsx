// Tomato Monster 面板加宽 + 月度目标位置探索
// 面板 mock 复刻自 src/panel/panel.{html,css} + theme.css(浅色档),
// 尺寸与 flex 布局按真实 CSS 等比还原,现状板的"难看"即真实渲染行为。

const { useState } = React;

// —— 数据 ——

// planned = 每日默认番茄数;monthTarget/monthDone = 把设置里的数字读作"本月总量"时用
const AUG_GOALS = [
  { id: 'g1', title: '内参阅读', cat: '学习', planned: 1, monthTarget: 30, monthDone: 12 },
  { id: 'g2', title: '内驱式学习', cat: '学习', planned: 2, monthTarget: 40, monthDone: 18 },
  { id: 'g3', title: '网站建设完成', cat: '工作', planned: 2, monthTarget: 20, monthDone: 5 },
];

const CAT_CLASS = { 工作: 'work', 学习: 'study', 生活: 'life', 兴趣爱好: 'hobby' };

// —— 基础零件 ——

function TMScene({ height, children }) {
  return <div className="tm-desktop" style={{ height }}>{children}</div>;
}

function TMWindow({ width, children }) {
  return (
    <div className="tm-window" style={{ width }}>
      <div className="tm-container">{children}</div>
    </div>
  );
}

function TMTimer() {
  return (
    <section className="tm-card tm-timer-card">
      <img className="tm-monster" src="happy.svg" alt="" />
      <div className="tm-phase">准备开始</div>
      <div className="tm-time">25:00</div>
      <button className="tm-btn-primary">开始专注</button>
      <div className="tm-btn-row">
        <button className="tm-btn-secondary" disabled>放弃</button>
        <button className="tm-btn-secondary">重置</button>
      </div>
      <div className="tm-quota">今日剩 3/3</div>
    </section>
  );
}

function TMHarvest() {
  return (
    <section className="tm-card tm-harvest">
      <div className="tm-harvest-label">今日收获</div>
      <div className="tm-harvest-icons">🍅🍅🍅</div>
      <div className="tm-streak">🔥 已连续 6 天</div>
    </section>
  );
}

function TMFooter() {
  return (
    <React.Fragment>
      <div className="tm-footer"><button className="tm-ghost">统计与设置</button></div>
      <div className="tm-hint">按时停下来,比多做一轮重要。</div>
    </React.Fragment>
  );
}

function CatBadge({ cat }) {
  return <span className={'tm-cat ' + CAT_CLASS[cat]}>{cat}</span>;
}

const MAX_TOM_ICONS = 8; // 与 panel.js MAX_TOMATO_ICONS 一致

// 计划数 <= 8 画图标;更大改数值,否则 30 个番茄会铺满整行
function Toms({ planned, used }) {
  if (planned > MAX_TOM_ICONS) {
    return <span className="tm-toms-num">🍅 <b>{used}</b>/{planned}</span>;
  }
  const items = [];
  for (let i = 0; i < planned; i++) {
    items.push(<span key={i} className={'tm-tom' + (i < used ? ' used' : '')}>🍅</span>);
  }
  return <div className="tm-toms">{items}</div>;
}

// 可增减的番茄数:−/+ 夹住 Toms。always=常驻显示,否则悬停任务行才显形
function TomsEditable({ planned, used, onChange, always }) {
  return (
    <div className={'tm-toms-edit' + (always ? ' always' : '')}>
      <button className="tm-step-btn" disabled={planned <= Math.max(1, used)}
              title="减少今天的番茄数"
              onClick={() => onChange(planned - 1)}>−</button>
      <Toms planned={planned} used={used} />
      <button className="tm-step-btn" disabled={planned >= 99}
              title="增加今天的番茄数"
              onClick={() => onChange(planned + 1)}>+</button>
    </div>
  );
}

function TMAddForm() {
  return (
    <div className="tm-add">
      <input className="tm-input" placeholder="比如:内参阅读" readOnly />
      <select className="tm-select" defaultValue="学习">
        <option>工作</option><option>学习</option><option>生活</option><option>兴趣爱好</option>
      </select>
      <input className="tm-num" defaultValue="1" readOnly />
      <button className="tm-btn-add">添加</button>
    </div>
  );
}

// 单行任务行 = 现状布局(panel.css .task-item)
function TaskRowSingle({ t }) {
  return (
    <li className={'tm-task' + (t.current ? ' cur' : '')}>
      <input type="checkbox" className="tm-check" readOnly checked={!!t.done} />
      <span className="tm-title">{t.title}<CatBadge cat={t.cat} /></span>
      <Toms planned={t.planned} used={t.used || 0} />
      <button className={'tm-mini-btn' + (t.current ? ' cur-on' : '')}>
        {t.current ? '当前任务' : '设为当前'}
      </button>
      <button className="tm-del">×</button>
    </li>
  );
}

// 两行任务行 = W2 改良:标题独占一行,进度与操作沉到第二行。
// onPlanned 传入时第二行的番茄数可增减(面板上直接改今天的计划数)。
function TaskRowTwoLine({ t, onPlanned, alwaysStep }) {
  return (
    <li className={'tm-task2' + (t.current ? ' cur' : '')}>
      <div className="tm-task2-top">
        <input type="checkbox" className="tm-check" readOnly checked={!!t.done} />
        <span className="tm-title">{t.title}<CatBadge cat={t.cat} /></span>
        <button className="tm-del">×</button>
      </div>
      <div className="tm-task2-bottom">
        {onPlanned
          ? <TomsEditable planned={t.planned} used={t.used || 0} always={alwaysStep}
                          onChange={(n) => onPlanned(t.id, n)} />
          : <Toms planned={t.planned} used={t.used || 0} />}
        <button className={'tm-mini-btn' + (t.current ? ' cur-on' : '')}>
          {t.current ? '当前任务' : '设为当前'}
        </button>
      </div>
    </li>
  );
}

function TMTasksCard({ tasks, twoLine, children, onPlanned, alwaysStep }) {
  const Row = twoLine ? TaskRowTwoLine : TaskRowSingle;
  return (
    <section className="tm-card">
      <header className="tm-tasks-header">
        <span className="tm-tasks-title">今日任务</span>
        <span className="tm-badge">0 个完成</span>
      </header>
      {children}
      <TMAddForm />
      {tasks.length > 0
        ? <ul className="tm-list">
            {tasks.map((t) => (
              <Row key={t.id} t={t} onPlanned={onPlanned} alwaysStep={alwaysStep} />
            ))}
          </ul>
        : <div className="tm-empty">还没有任务,先加一个吧。</div>}
    </section>
  );
}

// 宽度对比用的固定任务组:一个短名 + 一个长名
const WIDTH_DEMO_TASKS = [
  { id: 't1', title: '内参阅读', cat: '学习', planned: 2, used: 1, current: true },
  { id: 't2', title: '完成网站建设的首页设计稿', cat: '工作', planned: 3, used: 0 },
];

function PanelWidthDemo({ width, twoLine }) {
  return (
    <TMWindow width={width}>
      <TMTimer />
      <TMHarvest />
      <TMTasksCard tasks={WIDTH_DEMO_TASKS} twoLine={twoLine} />
      <TMFooter />
    </TMWindow>
  );
}

// —— 月度目标交互逻辑(方案 A/B 共用)——
// 勾选 = 按目标预设加入今日任务;已投入番茄(used>0)后锁定不可取消。

function useGoalState(seedPlanned) {
  // g1 已勾且投入 1 番茄(锁定示例);g2 已勾未投入(可取消);g3 未勾
  const [checked, setChecked] = useState({ g1: true, g2: true, g3: false });
  // 今天的计划番茄数:勾选时取目标预设,之后在面板上可改——只影响今天,不回写月度目标
  const [planned, setPlanned] = useState(
    seedPlanned || { 'task-g1': 1, 'task-g2': 2, 'task-g3': 2, manual1: 1 }
  );
  const usedByGoal = { g1: 1, g2: 0, g3: 0 };

  const toggle = (g) => {
    if (checked[g.id] && usedByGoal[g.id] > 0) return; // 已投入,锁定
    const on = !checked[g.id];
    setChecked((prev) => ({ ...prev, [g.id]: on }));
    // 重新勾上时回到目标预设值(上次的临时改动不粘住)
    if (on) setPlanned((prev) => ({ ...prev, ['task-' + g.id]: g.planned }));
  };

  const onPlanned = (id, n) => {
    if (n < 1 || n > 99) return;
    setPlanned((prev) => ({ ...prev, [id]: n }));
  };

  const goalTasks = AUG_GOALS.filter((g) => checked[g.id]).map((g) => ({
    id: 'task-' + g.id,
    title: g.title,
    cat: g.cat,
    planned: planned['task-' + g.id] ?? g.planned,
    used: usedByGoal[g.id],
    current: g.id === 'g1',
  }));
  const manualTask = {
    id: 'manual1', title: '回访客户邮件', cat: '工作',
    planned: planned.manual1 ?? 1, used: 0,
  };
  return { checked, usedByGoal, toggle, onPlanned, tasks: [...goalTasks, manualTask] };
}

// 方案 A:chips 行内嵌在「今日任务」卡顶部
function PanelGoalChips({ width }) {
  const s = useGoalState();
  return (
    <TMWindow width={width}>
      <TMTimer />
      <TMHarvest />
      <TMTasksCard tasks={s.tasks} twoLine onPlanned={s.onPlanned}>
        <div className="tm-goal-strip">
          <div className="tm-goal-strip-label">
            <span>8 月目标 · 勾选加入今天</span>
            <a className="tm-goal-edit" title="在统计与设置中管理">管理 ›</a>
          </div>
          <div className="tm-chips">
            {AUG_GOALS.map((g) => {
              const on = s.checked[g.id];
              const locked = on && s.usedByGoal[g.id] > 0;
              return (
                <button key={g.id}
                        className={'tm-chip' + (on ? ' on' : '') + (locked ? ' locked' : '')}
                        title={locked ? '已投入番茄,今天不可移除' : ''}
                        onClick={() => s.toggle(g)}>
                  {on && <span className="tick">✓</span>}
                  {g.title}
                  <span className="toms">×{g.planned}</span>
                </button>
              );
            })}
          </div>
        </div>
      </TMTasksCard>
      <TMFooter />
    </TMWindow>
  );
}

// 方案 B:独立「本月目标」卡片,位于收获与今日任务之间
function PanelGoalCard({ width }) {
  const s = useGoalState();
  return (
    <TMWindow width={width}>
      <TMTimer />
      <TMHarvest />
      <section className="tm-card">
        <header className="tm-tasks-header">
          <span className="tm-tasks-title">本月目标</span>
          <a className="tm-goal-edit" title="在统计与设置中管理">管理 ›</a>
        </header>
        {AUG_GOALS.map((g) => {
          const on = s.checked[g.id];
          const locked = on && s.usedByGoal[g.id] > 0;
          return (
            <label key={g.id} className={'tm-goal-row' + (locked ? ' locked' : '')}
                   title={locked ? '已投入番茄,今天不可移除' : ''}>
              <input type="checkbox" className="tm-check" checked={on}
                     onChange={() => s.toggle(g)} />
              <span className="tm-title">{g.title}<CatBadge cat={g.cat} /></span>
              <span className="tm-goal-toms">🍅×{g.planned}</span>
            </label>
          );
        })}
        <div className="tm-goal-hint">勾选后按预设分类与番茄数加入「今日任务」。</div>
      </section>
      <TMTasksCard tasks={s.tasks} twoLine onPlanned={s.onPlanned} />
      <TMFooter />
    </TMWindow>
  );
}

// ——————————————————————————————————————————————
// 定稿方案:360px + 两行任务行 + A chips 内嵌 + 数字输入框
// 设置里的数字 = 本月总量(chip 上显示 12/30);今天做几个在任务行敲数字。
// ——————————————————————————————————————————————

// 今日计划番茄数:直接敲数字(比 −/+ 步进器快)。
// 聚焦全选,回车/失焦提交;不能小于今天已投入的数量。
function PlanField({ planned, used, onChange }) {
  const [draft, setDraft] = useState(String(planned));
  React.useEffect(() => { setDraft(String(planned)); }, [planned]);

  const commit = () => {
    const floor = Math.max(1, used || 0);
    const n = Math.min(99, Math.max(floor, parseInt(draft, 10) || floor));
    onChange(n);
    setDraft(String(n));
  };

  return (
    <span className="tm-plan-field">
      <input className="tm-plan-input" type="number" min="1" max="99"
             aria-label="今天计划几个番茄" value={draft}
             onFocus={(e) => e.target.select()}
             onChange={(e) => setDraft(e.target.value.replace(/[^0-9]/g, '').slice(0, 2))}
             onBlur={commit}
             onKeyDown={(e) => { if (e.key === 'Enter') { commit(); e.target.blur(); } }} />
      <span className="tm-plan-unit">🍅</span>
      {used > 0 && <span className="tm-plan-done">已做 <b>{used}</b></span>}
    </span>
  );
}

function TaskRowFinal({ t, onPlanned }) {
  return (
    <li className={'tm-task2' + (t.current ? ' cur' : '')}>
      <div className="tm-task2-top">
        <input type="checkbox" className="tm-check" readOnly checked={!!t.done} />
        <span className="tm-title">{t.title}<CatBadge cat={t.cat} /></span>
        <button className="tm-del">×</button>
      </div>
      <div className="tm-task2-bottom">
        <PlanField planned={t.planned} used={t.used || 0}
                   onChange={(n) => onPlanned(t.id, n)} />
        <button className={'tm-mini-btn' + (t.current ? ' cur-on' : '')}>
          {t.current ? '当前任务' : '设为当前'}
        </button>
      </div>
    </li>
  );
}

function PanelFinal({ width, notionLinked }) {
  const s = useGoalState();
  const monthDone = AUG_GOALS.reduce((a, g) => a + g.monthDone, 0);
  const monthTarget = AUG_GOALS.reduce((a, g) => a + g.monthTarget, 0);
  return (
    <TMWindow width={width}>
      <TMTimer />
      <TMHarvest />
      <section className="tm-card">
        <header className="tm-tasks-header">
          <span className="tm-tasks-title">今日任务</span>
          <span className="tm-badge">0 个完成</span>
        </header>

        <div className="tm-goal-strip">
          <div className="tm-goal-strip-label">
            <span>8 月目标 · 勾选加入今天{notionLinked ? ' · 来自 Notion' : ''}</span>
            <a className="tm-goal-edit">管理 ›</a>
          </div>
          <div className="tm-chips">
            {AUG_GOALS.map((g) => {
              const on = s.checked[g.id];
              const locked = on && s.usedByGoal[g.id] > 0;
              return (
                <button key={g.id}
                        className={'tm-chip' + (on ? ' on' : '') + (locked ? ' locked' : '')}
                        title={locked ? '已投入番茄,今天不可移除' : ''}
                        onClick={() => s.toggle(g)}>
                  {on && <span className="tick">✓</span>}
                  {g.title}
                  <span className="prog">{g.monthDone}/{g.monthTarget}</span>
                </button>
              );
            })}
          </div>
          <div className="tm-goal-prog">
            <i style={{ width: Math.round((monthDone / monthTarget) * 100) + '%' }}></i>
          </div>
        </div>

        <TMAddForm />
        <ul className="tm-list">
          {s.tasks.map((t) => (
            <TaskRowFinal key={t.id} t={t} onPlanned={s.onPlanned} />
          ))}
        </ul>
      </section>
      <TMFooter />
    </TMWindow>
  );
}

// —— 解读二:设置里的数字是「本月总量」(30),chip 上显示月进度 ——
// 勾选后今天的任务从 1 个番茄起步,在任务行上按需加到今天想做的量。

function PanelGoalMonthly({ width }) {
  const s = useGoalState({ 'task-g1': 1, 'task-g2': 2, 'task-g3': 2, manual1: 1 });
  return (
    <TMWindow width={width}>
      <TMTimer />
      <TMHarvest />
      <TMTasksCard tasks={s.tasks} twoLine onPlanned={s.onPlanned} alwaysStep>
        <div className="tm-goal-strip">
          <div className="tm-goal-strip-label">
            <span>8 月目标 · 勾选加入今天</span>
            <a className="tm-goal-edit" title="在统计与设置中管理">管理 ›</a>
          </div>
          <div className="tm-chips">
            {AUG_GOALS.map((g) => {
              const on = s.checked[g.id];
              const locked = on && s.usedByGoal[g.id] > 0;
              return (
                <button key={g.id}
                        className={'tm-chip' + (on ? ' on' : '') + (locked ? ' locked' : '')}
                        title={locked ? '已投入番茄,今天不可移除' : ''}
                        onClick={() => s.toggle(g)}>
                  {on && <span className="tick">✓</span>}
                  {g.title}
                  <span className="prog">{g.monthDone}/{g.monthTarget}</span>
                </button>
              );
            })}
          </div>
          <div className="tm-goal-prog">
            <i style={{ width: '38%' }}></i>
          </div>
        </div>
      </TMTasksCard>
      <TMFooter />
    </TMWindow>
  );
}

// 「30」这个数落到任务行上的两种画法
function BigCountDemo() {
  const [planned, setPlanned] = useState(30);
  const rows = [
    { key: 'icons', label: '现状画法 · 8 个图标 + 折叠', node: (
      <div className="tm-toms">
        {Array.from({ length: 8 }, (_, i) => (
          <span key={i} className={'tm-tom' + (i < 3 ? ' used' : '')}>🍅</span>
        ))}
        <span className="tm-tom overflow" style={{ color: 'var(--warning)', fontSize: 10, fontWeight: 600, width: 'auto', padding: '0 2px' }}>+22</span>
      </div>
    ) },
    { key: 'num', label: '建议画法 · 超过 8 个改数值', node: (
      <span className="tm-toms-num">🍅 <b>3</b>/30</span>
    ) },
  ];
  return (
    <div style={{
      height: '100%', boxSizing: 'border-box', padding: '18px 16px',
      background: 'var(--window-wash)', display: 'flex', flexDirection: 'column', gap: 12,
      font: '400 13px/1.5 var(--font-body)', color: 'var(--on-neutral)',
    }}>
      <div style={{ font: '600 12px/1.4 var(--font-body)', color: 'var(--on-neutral-muted)' }}>
        计划 30 个番茄时,任务行长这样
      </div>
      {rows.map((r) => (
        <div key={r.key}>
          <div style={{ font: '400 10px/1.4 var(--font-body)', color: 'var(--on-neutral-muted)', marginBottom: 4 }}>
            {r.label}
          </div>
          <ul className="tm-list">
            <li className="tm-task2">
              <div className="tm-task2-top">
                <input type="checkbox" className="tm-check" readOnly />
                <span className="tm-title">内参阅读<CatBadge cat="学习" /></span>
                <button className="tm-del">×</button>
              </div>
              <div className="tm-task2-bottom">
                {r.node}
                <button className="tm-mini-btn">设为当前</button>
              </div>
            </li>
          </ul>
        </div>
      ))}
      <div style={{ font: '400 10px/1.4 var(--font-body)', color: 'var(--on-neutral-muted)', marginBottom: 4, marginTop: 4 }}>
        可增减(常驻步进器,点 −/+ 试试)
      </div>
      <ul className="tm-list">
        <li className="tm-task2">
          <div className="tm-task2-top">
            <input type="checkbox" className="tm-check" readOnly />
            <span className="tm-title">内参阅读<CatBadge cat="学习" /></span>
            <button className="tm-del">×</button>
          </div>
          <div className="tm-task2-bottom">
            <TomsEditable planned={planned} used={3} always
                          onChange={(n) => n >= 1 && n <= 99 && setPlanned(n)} />
            <button className="tm-mini-btn">设为当前</button>
          </div>
        </li>
      </ul>
    </div>
  );
}

// —— 统计与设置窗口:月度目标管理卡 ——

function StatsGoalManage({ withMonth }) {
  return (
    <div className="tm-stats-window" style={{ width: withMonth ? 700 : 620 }}>
      <div className="tm-titlebar">
        <span className="tm-dot" style={{ background: '#f5645c' }}></span>
        <span className="tm-dot" style={{ background: '#f5bd4f' }}></span>
        <span className="tm-dot" style={{ background: '#63c554' }}></span>
        <span className="tm-titlebar-text">Tomato Monster · 统计与设置</span>
      </div>
      <div className="tm-stats-body">
        <div className="tm-stats-card ghosted"><h2>📊 最近 7 天</h2></div>

        <div className="tm-stats-card">
          <h2>🎯 月度目标</h2>
          <div className="tm-stats-sub">
            {withMonth
              ? '2026 年 8 月 · 左边的数字是「每天默认几个番茄」,右边是「这个月一共想做多少个」。'
              : '2026 年 8 月 · 每天在面板勾选即可加入今日任务,分类与番茄数用这里的预设。'}
          </div>
          <div className="tm-manage-list">
            {AUG_GOALS.map((g) => (
              <div key={g.id} className="tm-manage-row">
                <span className="tm-manage-title">{g.title}</span>
                <select className="tm-select" defaultValue={g.cat}>
                  <option>工作</option><option>学习</option><option>生活</option><option>兴趣爱好</option>
                </select>
                <div className="tm-stepper" title="每天勾选后默认几个番茄(面板上可改)">
                  <button>−</button><span>{g.planned} 🍅</span><button>+</button>
                </div>
                {withMonth && (
                  <div className="tm-stepper" title="这个月一共想做多少个番茄">
                    <button>−</button><span>{g.monthTarget}</span><button>+</button>
                  </div>
                )}
                <button className="tm-del">×</button>
              </div>
            ))}
          </div>
          <div className="tm-manage-add">
            <input className="tm-input" placeholder="新增目标,比如:每周复盘" readOnly />
            <select className="tm-select" defaultValue="学习">
              <option>工作</option><option>学习</option><option>生活</option><option>兴趣爱好</option>
            </select>
            <input className="tm-num" defaultValue="1" readOnly />
            <button className="tm-btn-add">添加</button>
          </div>
        </div>

        <div className="tm-stats-card ghosted"><h2>📖 历史明细</h2></div>
        <div className="tm-stats-card ghosted"><h2>💾 本地备份</h2></div>
      </div>
    </div>
  );
}

// ——————————————————————————————————————————————
// Notion 导入:app → Notion 月度页。三种落点的对比
// ——————————————————————————————————————————————

const SEP_GOALS = [
  { title: '内参阅读', cat: '学习', target: 30 },
  { title: '内驱式学习', cat: '学习', target: 40 },
  { title: '网站建设完成', cat: '工作', target: 20 },
];

const CAT_TAG = { 学习: 'study', 工作: 'work' };

// 设置窗口:「月度番茄目标」卡 + 导入 Notion
function StatsMonthGoalCard() {
  const [state, setState] = useState('idle'); // idle | done
  return (
    <div className="tm-stats-window" style={{ width: 640 }}>
      <div className="tm-titlebar">
        <span className="tm-dot" style={{ background: '#f5645c' }}></span>
        <span className="tm-dot" style={{ background: '#f5bd4f' }}></span>
        <span className="tm-dot" style={{ background: '#63c554' }}></span>
        <span className="tm-titlebar-text">Tomato Monster · 统计与设置</span>
      </div>
      <div className="tm-stats-body">
        <div className="tm-stats-card ghosted"><h2>📊 最近 7 天</h2></div>

        <div className="tm-stats-card">
          <h2>🎯 月度番茄目标</h2>
          <div className="tm-stats-sub">
            2026 年 9 月 · 面板上勾选即可加入今日任务;这里的数字是本月总量。
          </div>
          <div className="tm-manage-list">
            {SEP_GOALS.map((g) => (
              <div key={g.title} className="tm-manage-row">
                <span className="tm-manage-title">{g.title}</span>
                <select className="tm-select" defaultValue={g.cat}>
                  <option>工作</option><option>学习</option><option>生活</option><option>兴趣爱好</option>
                </select>
                <input className="tm-plan-input" type="number" min="1" max="99"
                       defaultValue={g.target} aria-label="本月总量" />
                <button className="tm-del">×</button>
              </div>
            ))}
          </div>
          <div className="tm-manage-add">
            <input className="tm-input" placeholder="新增目标,比如:每周复盘" readOnly />
            <select className="tm-select" defaultValue="学习">
              <option>工作</option><option>学习</option><option>生活</option><option>兴趣爱好</option>
            </select>
            <input className="tm-plan-input" type="number" min="1" max="99" defaultValue="10" />
            <button className="tm-btn-add">添加</button>
          </div>

          <div className="tm-import-row">
            <button className="tm-btn-import" onClick={() => setState(state === 'idle' ? 'done' : 'idle')}>
              ↗ 导入 Notion
            </button>
            <select className="tm-month-pick" defaultValue="2026-09">
              <option>2026-09</option><option>2026-10</option>
            </select>
            <span className="tm-import-note">
              {state === 'idle'
                ? '写入「时间框架」库的 2026-09 页面。'
                : <React.Fragment><b>✓ 已写入 2026-09</b> · 3 个目标,共 90 🍅</React.Fragment>}
            </span>
          </div>
        </div>

        <div className="tm-stats-card ghosted"><h2>📤 Notion 导出</h2></div>
      </div>
    </div>
  );
}

// —— Notion 2026-09 页面的三种样子 ——

function NotionPageShell({ children, extraProp }) {
  return (
    <div className="nt-page">
      <div className="nt-crumb">时间框架 / 月 / 2026-09</div>
      <div className="nt-title">2026-09</div>
      <div className="nt-props">
        <div className="nt-prop">
          <span className="nt-prop-key">📅 日期</span>
          <span className="nt-prop-val">2026/09/01 → 2026/09/30</span>
        </div>
        <div className="nt-prop">
          <span className="nt-prop-key">🔗 周</span>
          <span className="nt-prop-val nt-meta">2026-W36, W37, W38, W39</span>
        </div>
        {extraProp}
      </div>
      <div className="nt-divider"></div>
      {children}
    </div>
  );
}

// 方案 A:写进页面正文(to-do 块),用标记块界定可重复覆盖的区域
function NotionOptionContent() {
  return (
    <NotionPageShell>
      <div className="nt-h2">本月复盘</div>
      <div className="nt-block nt-meta">(你原有的正文内容……)</div>
      <div className="nt-divider"></div>
      <div className="nt-highlight">
        <div className="nt-marker">— 🍅 番茄目标 · 由 Tomato Monster 写入 —</div>
        {SEP_GOALS.map((g) => (
          <div key={g.title} className="nt-block">
            <span className="nt-todo-box"></span>
            <span>
              {g.title}
              <span className={'nt-tag ' + CAT_TAG[g.cat]}>{g.cat}</span>
              <span className="nt-meta"> · 目标 {g.target} 🍅</span>
            </span>
          </div>
        ))}
        <div className="nt-marker">— 以上由 app 维护,重复导入会整段替换 —</div>
      </div>
    </NotionPageShell>
  );
}

// 方案 B:加一个属性,所有目标挤成一串文本
function NotionOptionProperty() {
  return (
    <NotionPageShell
      extraProp={
        <div className="nt-prop is-new">
          <span className="nt-prop-key">🍅 番茄目标</span>
          <span className="nt-prop-val">
            内参阅读×30;内驱式学习×40;网站建设完成×20
          </span>
        </div>
      }>
      <div className="nt-h2">本月复盘</div>
      <div className="nt-block nt-meta">(你原有的正文内容,不受影响……)</div>
      <div className="nt-divider"></div>
      <div className="nt-block nt-meta">
        属性栏里多一行,一眼能看到;但三个目标只能挤成一串文本,
        没法单独勾选、单独统计。
      </div>
    </NotionPageShell>
  );
}

// 方案 C:独立「月度番茄目标」库,用 relation 挂到月度页
function NotionOptionDatabase() {
  return (
    <NotionPageShell
      extraProp={
        <div className="nt-prop is-new">
          <span className="nt-prop-key">🍅 番茄目标</span>
          <span className="nt-prop-val">内参阅读 · 内驱式学习 · 网站建设完成</span>
        </div>
      }>
      <div className="nt-h2">🍅 本月番茄目标</div>
      <div className="nt-meta" style={{ fontSize: 11, marginBottom: 2 }}>
        月度番茄目标库 · 筛选:所属月 = 2026-09
      </div>
      <div className="nt-db">
        <div className="nt-db-head">
          <span className="nt-db-cell">目标</span>
          <span className="nt-db-cell mid">分类</span>
          <span className="nt-db-cell narrow">目标</span>
          <span className="nt-db-cell narrow">已完成</span>
        </div>
        {SEP_GOALS.map((g, i) => (
          <div key={g.title} className="nt-db-row">
            <span className="nt-db-cell">{g.title}</span>
            <span className="nt-db-cell mid">
              <span className={'nt-tag ' + CAT_TAG[g.cat]} style={{ marginLeft: 0 }}>{g.cat}</span>
            </span>
            <span className="nt-db-cell narrow">{g.target}</span>
            <span className="nt-db-cell narrow nt-meta">{[12, 18, 5][i]}</span>
          </div>
        ))}
      </div>
      <div className="nt-block nt-meta" style={{ marginTop: 10 }}>
        每个目标一行,可单独筛选统计;「已完成」列以后能让 app 每天回写。
      </div>
    </NotionPageShell>
  );
}

// —— 说明板(代替 post-it,可拖动排序)——

function NoteBoard({ title, items }) {
  return (
    <div style={{
      height: '100%', boxSizing: 'border-box', padding: '18px 20px',
      background: '#fdf9ee', fontFamily: "-apple-system, 'PingFang SC', sans-serif",
      color: '#4a4438', overflow: 'hidden',
    }}>
      <div style={{ font: '600 14px/1.4 inherit', marginBottom: 10 }}>{title}</div>
      <ul style={{ margin: 0, padding: '0 0 0 18px', display: 'flex', flexDirection: 'column', gap: 8 }}>
        {items.map((it, i) => (
          <li key={i} style={{ font: '400 12px/1.6 inherit' }}>{it}</li>
        ))}
      </ul>
    </div>
  );
}

// —— 画布 ——

function App() {
  return (
    <DesignCanvas title="Tomato Monster · 面板加宽 + 月度目标" subtitle="三个问题的设计探索 · 2026-08">
      <DCSection id="final" title="★ 定稿方案" subtitle="360px · 两行任务行 · A chips 内嵌 · 本月总量+月进度 · 今日番茄数敲数字(点数字框直接改试试)">
        <DCArtboard id="f-panel" label="定稿面板(全部决策合成)" width={408} height={890}>
          <TMScene height={890}><PanelFinal width={360} /></TMScene>
        </DCArtboard>
        <DCArtboard id="f-notion" label="接上 Notion 后(目标来自月度页)" width={408} height={890}>
          <TMScene height={890}><PanelFinal width={360} notionLinked /></TMScene>
        </DCArtboard>
        <DCArtboard id="f-notes" label="定稿要点" width={300} height={890}>
          <NoteBoard title="这一版定了什么" items={[
            '宽度 360px,窗口高 480 → 约 560;任务行两行排,长任务名一行放得下。',
            '月度目标 = chips 嵌在「今日任务」卡顶部;chip 上的 12/30 是本月进度,下面细进度条是三个目标合计。',
            '勾选 = 按目标的分类加入今日任务,今天做几个在任务行的数字框里敲(默认 1)。',
            '数字框:点一下全选,敲新数字,回车或点别处生效;不能小于今天已投入的数量。',
            '改的只是今天那条任务,不动月度目标本身。',
            '月度目标永不自动重置——换月只是月进度归零重新计,目标条目原样留着,你想改就去「管理 ›」。',
            '已投入番茄的目标当天不能取消勾选(防止把已有记录删掉)。',
          ]} />
        </DCArtboard>
      </DCSection>

      <DCSection id="width" title="① 面板宽度" subtitle="现状 300px vs 加宽方案 · 长任务名「完成网站建设的首页设计稿」做压力测试">
        <DCArtboard id="w-300" label="现状 · 300px(单行任务行)" width={348} height={700}>
          <TMScene height={700}><PanelWidthDemo width={300} /></TMScene>
        </DCArtboard>
        <DCArtboard id="w-360" label="W1 · 360px 纯加宽(布局不动)" width={408} height={700}>
          <TMScene height={700}><PanelWidthDemo width={360} /></TMScene>
        </DCArtboard>
        <DCArtboard id="w-360-2line" label="W2 · 360px + 任务行两行排(推荐)" width={408} height={700}>
          <TMScene height={700}><PanelWidthDemo width={360} twoLine /></TMScene>
        </DCArtboard>
        <DCArtboard id="w-notes" label="宽度备注" width={260} height={700}>
          <NoteBoard title="宽度怎么选" items={[
            '300px 现状:输入框 placeholder 被截断;长任务名挤在按钮之间换行,行高失控。',
            'W1 只把窗口从 300 加到 360:输入行立刻正常,但 10 字以上任务名仍会换行。',
            'W2 在 360 基础上把任务行改成两行:标题独占整行(约 22 字),番茄进度和按钮沉到第二行,任意长度都稳。',
            '340 / 380 也可选:340 仍偏挤,380 开始像小窗口不像菜单栏面板。建议 360。',
            '窗口高度同步 480 → 约 560,配合月度目标区不出滚动条。',
          ]} />
        </DCArtboard>
      </DCSection>

      <DCSection id="goals-panel" title="② 月度目标 · 放在面板哪里" subtitle="两个方案都可点选试玩:勾选/取消看任务列表联动 · 「内参阅读」演示已投入番茄后的锁定态">
        <DCArtboard id="g-chips" label="方案 A · chips 行嵌在「今日任务」卡顶部" width={408} height={890}>
          <TMScene height={890}><PanelGoalChips width={360} /></TMScene>
        </DCArtboard>
        <DCArtboard id="g-card" label="方案 B · 独立「本月目标」卡片" width={408} height={980}>
          <TMScene height={980}><PanelGoalCard width={360} /></TMScene>
        </DCArtboard>
        <DCArtboard id="g-notes" label="A vs B" width={280} height={890}>
          <NoteBoard title="怎么权衡" items={[
            'A(chips 内嵌):任务的两个来源(勾目标/手动输)收在同一张卡里,面板总高最省;chip 上直接带 ×N 番茄预设。',
            'B(独立卡):月度目标有自己的地盘,一眼看到全部目标与勾选态;但面板多一张卡,高度 +90px 左右,信息有一点重复(勾了的目标上下各出现一次)。',
            '共同交互:勾选=按预设加入今日任务;当天已投入番茄的目标锁定,不能取消(防误删记录)。',
            '「管理 ›」跳到统计与设置窗口的月度目标卡,面板上不做增删改。',
            '另一个更省事的思路:每天首次打开自动把所有月度目标加进今日任务,不想做的手动删——完全零操作,但失去「今天我选哪几个」的仪式感。可讨论。',
          ]} />
        </DCArtboard>
      </DCSection>

      <DCSection id="count-edit" title="④ 番茄数怎么改 · 设置里的数字算什么" subtitle="两块面板都可点 −/+ 实测:改的只是今天,不回写月度目标">
        <DCArtboard id="c-daily" label="解读 1 · 数字＝每日默认(悬停任务行显形 −/+)" width={408} height={890}>
          <TMScene height={890}><PanelGoalChips width={360} /></TMScene>
        </DCArtboard>
        <DCArtboard id="c-month" label="解读 2 · 数字＝本月总量(chip 带 12/30 进度,步进器常驻)" width={408} height={890}>
          <TMScene height={890}><PanelGoalMonthly width={360} /></TMScene>
        </DCArtboard>
        <DCArtboard id="c-big" label="「30」落到任务行上长什么样" width={392} height={430}>
          <BigCountDemo />
        </DCArtboard>
        <DCArtboard id="c-notes" label="两种解读的差别" width={300} height={430}>
          <NoteBoard title="设置里那个数字" items={[
            '解读 1「每日默认」:填 1 就是每天默认 1 个番茄,勾上即用这个值,今天想多做就在任务行 +。简单,不多存数据。',
            '解读 2「本月总量」:填 30 = 这个月要做 30 个番茄,chip 上直接显示 12/30 的月进度;勾上后今天从 1 起步,按需要加。多一层月度成就感。',
            '两者不冲突:也可以两个数都存(每日默认 + 本月总量),但设置页每行要放两个数字框,略重。',
            '共同规则:面板上改番茄数只影响今天那条任务,不回写月度目标;取消勾选再勾回来,回到预设值。',
            '减到已投入的数量为止:已经做了 3 个就减不到 2,防止记录对不上。',
          ]} />
        </DCArtboard>
      </DCSection>

      <DCSection id="notion" title="⑤ 导入 Notion · 写到月度页的哪里" subtitle="app 里写目标 → 点「导入 Notion」→ 落到 2026-09 页面。三种落点各有取舍">
        <DCArtboard id="n-card" label="设置里的「月度番茄目标」卡(点「导入 Notion」看反馈)" width={688} height={560}>
          <div className="tm-desktop" style={{ height: 560, alignItems: 'center' }}>
            <StatsMonthGoalCard />
          </div>
        </DCArtboard>
        <DCArtboard id="n-a" label="落点 A · 写进页面正文(to-do 块)" width={420} height={520}>
          <NotionOptionContent />
        </DCArtboard>
        <DCArtboard id="n-b" label="落点 B · 加一个属性" width={420} height={520}>
          <NotionOptionProperty />
        </DCArtboard>
        <DCArtboard id="n-c" label="落点 C · 独立目标库 + relation(推荐)" width={420} height={520}>
          <NotionOptionDatabase />
        </DCArtboard>
        <DCArtboard id="n-notes" label="三种落点怎么选" width={320} height={520}>
          <NoteBoard title="取舍" items={[
            'A 写正文:最像你手写的样子,能勾 to-do。但不结构化,月末想统计拿不到数;重复导入要靠标记块界定区域再整段替换,你自己在区域内加的字会被覆盖。',
            'B 加属性:改动最小,属性栏一眼可见。但一个属性只能塞一串文本(内参阅读×30;…),没法单独勾选或统计,目标一多就很挤。',
            'C 独立目标库:每个目标一行(名称/分类/本月总量/所属月 relation),在月度页里以 linked view 展示。完全结构化,能筛选统计,以后 app 还能每天回写「已完成」列。代价是要新建一个库。',
            'C 和你现有结构最一致——任务库→日库 已经是这个套路(relation 挂上级),月度目标库→时间框架 是同一个模式往上加一层。',
            '重复导入的处理(三种都要面对):按「目标名 + 所属月」查重,已存在就更新,不存在才新建——和现有 exportDay 的按标题查重逻辑一致。',
          ]} />
        </DCArtboard>
      </DCSection>

      <DCSection id="goals-manage" title="③ 月度目标 · 在哪里管理" subtitle="统计与设置窗口新增一张卡,放在「最近 7 天」之后、「历史明细」之前">
        <DCArtboard id="m-stats" label="统计与设置 · 🎯 月度目标卡(单数字:每日默认)" width={668} height={560}>
          <div className="tm-desktop" style={{ height: 560, alignItems: 'center' }}>
            <StatsGoalManage />
          </div>
        </DCArtboard>
        <DCArtboard id="m-stats-2" label="同上 · 双数字版(每日默认 + 本月总量)" width={748} height={560}>
          <div className="tm-desktop" style={{ height: 560, alignItems: 'center' }}>
            <StatsGoalManage withMonth />
          </div>
        </DCArtboard>
        <DCArtboard id="m-notes" label="决策点清单" width={300} height={620}>
          <NoteBoard title="决策状态" items={[
            '① 宽度 360 ✅',
            '② 两行排 ✅',
            '③ A chips 内嵌 ✅',
            '④ 数字＝本月总量 + 今日敲数字 ✅',
            '⑤ 不自动重置,手动改 ✅',
            '⑥ 待定:已投入番茄的目标当天锁定不可取消勾选?',
            '另:跨天数据残留是后端日切时机 bug(tick 线程漏调 roll_over),纯逻辑修复,不涉及 UI,方案确认后一并修。',
          ]} />
        </DCArtboard>
      </DCSection>
    </DesignCanvas>
  );
}

ReactDOM.createRoot(document.getElementById('root')).render(<App />);
