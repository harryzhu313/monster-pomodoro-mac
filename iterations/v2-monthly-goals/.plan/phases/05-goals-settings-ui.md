# Phase 05 — 月度番茄目标管理卡

**Status**: `completed`
**目标**: 在统计与设置窗口增删改月度目标
**前置**: Phase 04（面板侧已能消费目标数据）

## 验收判据

- 新卡「🎯 月度番茄目标」位于「📊 最近 7 天」之后、「📖 历史明细」之前
- 每行：名称 + 分类下拉（四个常量）+ 本月总量数字框 + 删除
- 底部一行新增；改动即时持久化，面板 chips 立刻同步（走 `state-update` 广播）
- 删除有确认（对齐「清零今日」的两步确认惯例），且删除目标不影响已经产生的历史任务记录
- 空状态有引导文案

## Sections

### Section A — 管理卡

**Gate**: `committed`

**自测判据**：增删改后重启 app 数据仍在；面板同步无需重开

**Tasks**：

- [x] `stats.html` 新增卡片结构，位置在「最近 7 天」与「历史明细」之间（`src/stats/stats.html:78-100`）
- [x] `stats.css` 四列行布局（颜色、边框、圆角、字号全部走现有 token）（`src/stats/stats.css:372-441`）
- [x] `stats.js` 渲染 + 绑定 `add_goal` / `update_goal` / `delete_goal`（`src/stats/stats.js:250-472`）
- [x] 复用 `CATEGORY_VALUES` 四个分类常量；新增默认「学习 / 10」，本月总量前后端均夹紧到 `1..999`（`src/shared/util.js:27-33`，`src/stats/stats.js:252-260,448-471`，`src-tauri/src/timer.rs:566-616`）
- [x] 删除两步确认（3 秒复位），提示已产生的今日/历史任务会保留（`src/stats/stats.js:263-369`，`src-tauri/src/timer.rs:618-619`）
- [x] 空状态文案：「还没有月度目标，先加一件这个月想持续做的事吧。」（`src/stats/stats.html:85-87`，`src/stats/stats.js:425`）
- [x] 按 `goal.id` 复用表单行，排除进度/勾选态的快照扰动，不丢输入草稿与焦点（`src/stats/stats.js:372-443`）

**记录**：

- 自测（2026-08-24）：
  - `node --input-type=module --check < src/stats/stats.js` 通过
  - `git diff --check` 通过
  - `cargo test` **40 passed / 0 failed**，已有目标增改删、进度、删除保留任务、备份 round-trip 覆盖
  - 独立静态审查通过：Tauri 参数映射、`state-update` 同步、四分类常量、两步删除、560px 最小窗口布局均无阻断问题
  - 审查发现并修复一个焦点竞态：不再整列清空重建，进度变化或面板勾选不会打断名称/数字输入
- 用户验收：2026-08-24 通过（用户反馈：「验收通过」）
- Commit：`af9fea6` (`Phase 05: 加入月度番茄目标管理卡`)

---

## Notes

- 管理卡最终复用 `get_state` / `state-update` 的 `monthlyGoals` 快照，不再另维护一份前端状态；`get_stats_bundle` 中依然携带同字段，但本卡不从那里消费。
- Notion 数据库已由用户创建；本 phase 严格保持本地目标管理范围，不提前混入 v3 联动。
