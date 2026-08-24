# Phase 01 — 跨天自动归档

**Status**: `completed`
**目标**: 跨过 00:00 后，面板上的"今日任务"自动清空并归档，不需要用户点任何地方
**前置**: 无（v1 已上线）

## 背景 / 根因

`timer::roll_over_tasks_if_needed()` 只在两处被调用：

- `get_state`（[lib.rs:169](../../../../src-tauri/src/lib.rs)）——面板 webview 启动时调一次，而面板窗口常驻，之后不再触发
- `get_stats_bundle`（[lib.rs:495](../../../../src-tauri/src/lib.rs)）——打开统计窗口时

每秒的 tick 线程和「点托盘弹出面板」走的 `emit_fresh_snapshot` 都不做日切，快照里 `tasksToday` 原样带着昨天的数据广播出去。用户点一次「统计与设置」恰好触发 `get_stats_bundle` 的日切，回来就干净了——与用户观察到的现象吻合。

配额（`quotaState`）与今日收获（`stats[today]`）是按日期惰性查询的，不受影响；只有任务列表是显式迁移。

## 验收判据

- 跨天后（含睡眠唤醒、app 常驻不重启两种路径），面板"今日任务"自动变空，昨天的任务出现在统计窗口的「📖 历史明细」里
- 归档只发生一次，重复 tick 不产生重复条目（`roll_over_tasks_if_needed` 已按 id 去重，需回归确认）
- 日期未变时 tick 的额外开销为一次字符串比较（提前返回），CPU 预算不受影响（ARCHITECTURE §9）
- `emit_fresh_snapshot`（托盘点击弹出面板）也走过日切，不会广播出隔夜快照

## Sections

### Section A — tick 线程补日切

**Gate**: `self-verified`

**自测判据**：

- `TOMATO_TEST=1` 下用改系统日期或注入假 today 的方式模拟跨天，观察面板自动清空
- `cargo test` 全绿；新增一条覆盖"tick 路径日切"的单测

**Tasks**：

- [x] tick 线程循环内调用 `roll_over_tasks_if_needed`，日期变化时并入 `sync_out` 落盘广播（改在**结算之后**而非之前——与 `get_state` 同序，跨零点的专注仍先记到昨天那条任务上再归档，不改动 v1 结算语义）
- [x] `emit_fresh_snapshot` 补日切（托盘点击弹面板的路径）；归档发生时顺带 `guard.save()`
- [x] `roll_over_tasks_if_needed` 改为返回 `bool`，调用方据此决定是否落盘广播
- [x] 单测 `task_rollover_is_idempotent_within_a_day`：跨天首次归档返回 true，之后 5 次均返回 false 且归档不堆积
- [x] 单测 `task_rollover_across_multiple_days`：隔多天不开机，只产生一条归档
- [x] 单测 `task_rollover_on_fresh_install`：空 date 只补日期，不产生空归档条目

**记录**：

- 自测：2026-08-24 `cargo test` 29 passed / 0 failed（新增 3 条跨天用例）；`cargo build` 无 warning
- **真机端到端（用户授权后用真实数据跑）**：2026-08-24 用户机器天然处于跨天状态——系统日期 2026-08-24，`tasksToday.date` 停在 2026-08-23，挂着 1 条「内驱式学习」(planned 3 / used 3)。退出 release 版 → `cargo tauri dev` 启动 → tick 线程自动归档：
  - `tasksToday.date` 2026-08-23 → **2026-08-24**
  - 今日任务数 1 → **0**
  - `tasksArchive["2026-08-23"]` 新增「内驱式学习」，planned/used/done 字段完整
  - 逐键比对备份：`stats` / `notionExportLog`(81 天) / `badgesState` / `notionConfig` / `settings` / `quotaState` / `timerState` **全部零改动**，只有 `tasksToday` 与 `tasksArchive` 发生了预期内的变化
  - 测试前已备份 store.json 到 scratchpad 并校验 SHA256 一致；测试后未回滚（归档是正确行为，回滚反而会把用户退回 bug 状态），release 版已重新启动
- 用户验收：
- Commit：

---

## Notes

- 归档逻辑本身（`roll_over_tasks_if_needed`）不改，只补调用点——它已经处理了"同一天多次归档按 id 去重"。
- 注意锁的使用：tick 线程已经持有 store 锁，日切在锁内调用即可，**不要**在锁内重入 `emit_fresh_snapshot`（v1 的 PanelPin 自死锁教训，见 [lib.rs:516](../../../../src-tauri/src/lib.rs) 注释）。
