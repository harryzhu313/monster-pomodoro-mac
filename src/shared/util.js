// 共享工具(面板 / 统计 / 庆祝共用)。前端是纯视图:时间与状态一律来自
// Rust 的 state-update 快照,这里只做格式化,不做任何计时。

/** ms → "mm:ss"(平移旧 popup.js formatMs) */
export function formatMs(ms) {
  const total = Math.max(0, Math.round(ms / 1000));
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
}

/** 快照 → 当前阶段剩余 ms(以快照携带的 now 为基准,避免前端时钟参与计时) */
export function computeRemaining(snap) {
  const t = snap.timer;
  if (t.state === 'FOCUSING' || t.state === 'BREAKING') {
    return Math.max(0, (t.endTime ?? 0) - snap.now);
  }
  if (t.state === 'PAUSED') return t.pausedRemaining ?? 0;
  return snap.durations.focusMs;
}

/** 分类常量(Notion select 依赖,不可改动——CONTENT.md 全局节) */
export const CATEGORY_VALUES = ['工作', '学习', '生活', '兴趣爱好'];
export const CATEGORY_DEFAULT = '工作';

export function normalizeCategory(c) {
  return CATEGORY_VALUES.includes(c) ? c : CATEGORY_DEFAULT;
}

/** 状态 → 小怪兽表情(平移旧 popup.js MONSTER_BY_STATE) */
export const MONSTER_BY_STATE = {
  IDLE: 'happy',
  FOCUSING: 'calm',
  BREAKING: 'angry',
  PAUSED: 'calm',
};
