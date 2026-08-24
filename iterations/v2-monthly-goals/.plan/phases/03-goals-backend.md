# Phase 03 — 月度目标数据层

**Status**: `completed`
**目标**: 月度目标的 schema、持久化、commands 与快照字段全部就位，Rust 是唯一事实来源
**前置**: Phase 02（UI 布局已定，字段设计不会返工）

## 数据设计（待实现，phase 内可微调）

`store.json` 顶层新增 `monthlyGoals`：

```jsonc
{
  "monthlyGoals": [
    { "id": "...", "title": "内参阅读", "category": "学习", "monthTarget": 30 }
  ]
}
```

- **不按月分桶**：目标永不自动重置，一份列表长期有效（用户拍板）
- 今日任务由目标产生时，在 `Task` 上打 `goalId`，用于：① chip 勾选态回显 ② 本月进度统计 ③ 已投入锁定判断
- 本月进度 = 遍历 `tasksToday` + `tasksArchive` 中本月范围内、`goalId` 匹配的任务的 `used` 之和

## 验收判据

- 新字段对旧 store.json 前向兼容（`#[serde(default)]`），老用户升级不丢数据、不报 corrupt
- 备份导出 / 导入覆盖 `monthlyGoals`（schemaVersion 保持 2 或按需升 3，导入仍兼容 v1/v2）
- commands：`add_goal` / `update_goal` / `delete_goal` / `toggle_goal_today` 全部走 `with_store`
- `toggle_goal_today` 实现三条规则：勾选按预设建任务（planned 默认 1）；已投入番茄时拒绝取消；取消勾选删除对应任务
- 快照新增 `monthlyGoals`（含每个目标的本月进度与今日勾选态），前端零计算

## Sections

### Section A — schema 与持久化

**Gate**: `committed`

**自测判据**：`cargo test` 全绿；用 v1 时期的真实 store.json 加载不报错、字段补默认值

**Tasks**：

- [x] `store.rs` 新增 `MonthlyGoal` 结构与 `StoreData.monthly_goals`（camelCase、`default`、`flatten` extra 透传）
- [x] `Task` 增加 `goal_id: Option<String>`（`skip_serializing_if`，旧任务不会凭空多出 `goalId` 键）
- [x] 备份导出 / 导入纳入 `monthlyGoals`；`BACKUP_SCHEMA_VERSION` 升到 **3**，同时把受理版本改为集合 `{1,2,3}`——原先的 `version != 1 && version != BACKUP_SCHEMA_VERSION` 写法在升版后会把 v2 老备份拒之门外
- [x] 单测：v1/v2 老备份仍可导入、未来版本仍拒绝；round-trip 携带目标；`goalId` 可选字段行为

### Section B — commands 与快照

**Gate**: `committed`

**自测判据**：`TOMATO_TEST=1` 下用临时钩子或单测驱动四个 command，观察 store.json 变化正确

**Tasks**：

- [x] `timer.rs`：`add_goal` / `update_goal` / `delete_goal` / `toggle_goal_today` + `goal_month_done` 本月进度计算
- [x] `toggle_goal_today` 三条规则（勾选按预设建任务 planned=1 / 已投入返回 `locked` 拒绝取消 / 取消删任务），外加目标不存在返回 `not-found`
- [x] `update_goal` 改名/改分类时同步今天那条任务，避免面板上两处名字对不上
- [x] `delete_goal` 保留已产生的任务记录（goalId 变悬空引用，只是不再计入进度）
- [x] `snapshot()` 与 `stats_bundle()` 都带 `monthlyGoals` 视图（id/title/category/monthTarget/monthDone/checkedToday/locked），前端零计算
- [x] `lib.rs` 注册 5 个 command（含从 phase 04 前移的 `set_task_planned`）
- [x] **从 phase 04 前移** `set_task_planned`：同属 Rust 数据层，放一起使 phase 03 成为自洽的验收单元；Rust 侧夹紧 `[max(1, used), 99]`
- [x] 单测 9 条覆盖：字段归一化与 clamp、勾选建任务、锁定拒绝、未知目标、跨月进度隔离、改名同步、删目标保记录、planned 夹紧

**记录**：

- 自测（2026-08-24）：`cargo build` 零 warning；`cargo test` **40 passed / 0 failed**（本 phase 新增 12 条）
- **真实数据兼容性验证**：用用户机器上的真实 store.json（86 天 stats / 86 天归档 / 81 天 Notion 导出日志）走新 schema 解析 + round-trip：解析成功，`monthlyGoals` 默认为空数组，`stats` / `tasksArchive` / `notionExportLog` / `badgesState` 逐字节一致，round-trip 后仅新增 `monthlyGoals` 一个顶层键。该验证用临时测试完成，跑完即删——真实数据含个人任务名，仓库是公开的，**不得作为测试夹具提交**
- 用户验收：2026-08-24 通过（无 UI，按测试证据验收）
- Commit：见 git log（Phase 03 提交）

---

## Notes

- 分类常量（工作/学习/生活/兴趣爱好）是产品红线 2，目标的 category 必须复用同一套常量，不得新增或改名。
- 不引入新 crate（AGENTS 规则 4）。
