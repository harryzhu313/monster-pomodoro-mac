# Phase 05 — 月度番茄目标管理卡

**Status**: `not started`
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

**Gate**: `pending`

**自测判据**：增删改后重启 app 数据仍在；面板同步无需重开

**Tasks**：

- [ ] `stats.html` 新增卡片结构
- [ ] `stats.css` 行布局样式（全 token）
- [ ] `stats.js` 渲染 + 绑定 `add_goal` / `update_goal` / `delete_goal`
- [ ] 删除两步确认
- [ ] 空状态文案

**记录**：

- 自测：
- 用户验收：
- Commit：

---

## Notes

- 统计窗口的数据来自 `get_stats_bundle`，需把 `monthlyGoals` 一并带出（或复用 `get_state` 快照，phase 内决定并记录）。
