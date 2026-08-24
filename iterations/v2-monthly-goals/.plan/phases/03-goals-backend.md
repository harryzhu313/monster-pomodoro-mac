# Phase 03 — 月度目标数据层

**Status**: `not started`
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

**Gate**: `pending`

**自测判据**：`cargo test` 全绿；用 v1 时期的真实 store.json 加载不报错、字段补默认值

**Tasks**：

- [ ] `store.rs` 新增 `MonthlyGoal` 结构与 `StoreData.monthly_goals`（camelCase、`default`、`flatten` extra 透传）
- [ ] `Task` 增加 `goal_id: Option<String>`（`skip_serializing_if`，不污染旧数据）
- [ ] 备份导出 / 导入纳入 `monthlyGoals`
- [ ] 单测：旧 store.json 无该字段时加载正常；备份 round-trip 保留目标

### Section B — commands 与快照

**Gate**: `pending`

**自测判据**：`TOMATO_TEST=1` 下用临时钩子或单测驱动四个 command，观察 store.json 变化正确

**Tasks**：

- [ ] `timer.rs`：`add_goal` / `update_goal` / `delete_goal` / `toggle_goal_today` + 本月进度计算函数
- [ ] `toggle_goal_today` 的三条规则（勾选建任务 / 已投入拒绝取消 / 取消删任务）
- [ ] `snapshot()` 新增 `monthlyGoals` 视图字段（title / category / monthTarget / monthDone / checkedToday / locked）
- [ ] `lib.rs` 注册四个 command 到 `invoke_handler`
- [ ] 单测覆盖：勾选后任务出现且带 goalId；已投入时取消被拒；跨月进度只算当月

**记录**：

- 自测：
- 用户验收：
- Commit：

---

## Notes

- 分类常量（工作/学习/生活/兴趣爱好）是产品红线 2，目标的 category 必须复用同一套常量，不得新增或改名。
- 不引入新 crate（AGENTS 规则 4）。
