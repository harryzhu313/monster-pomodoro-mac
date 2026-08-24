# Phase 06 — 全链路验收与 spec 同步

**Status**: `not started`
**目标**: 三个问题都真的解决了，文档与代码一致，可发布
**前置**: Phase 05

## 验收判据

- 跨天：模拟跨天后面板自动清空，昨天任务在历史明细里
- 加宽：长任务名不换行，各面板状态布局正常
- 月度目标：设置里加目标 → 面板勾选 → 数字框改数量 → 专注消耗 → chip 进度增长，全链路闭环
- `TOMATO_TEST=1 cargo tauri dev` 跑通一轮完整番茄周期（专注 → 提醒 → 休息 → 下一轮）
- 发布向提交确认 TEST_MODE 关闭
- 无新增依赖；`cargo test` 全绿

## Sections

### Section A — 验收与文档同步

**Gate**: `pending`

**Tasks**：

- [ ] 全链路手工验收（上述 5 条）
- [ ] `ARCHITECTURE.md` §4 schema 补 `monthlyGoals` 与 `Task.goalId`
- [ ] `DESIGN.md` 如记录了面板尺寸则同步 360×560；跑 `npx @google/design.md lint DESIGN.md`
- [ ] `iterations/v2-monthly-goals/PRD.md` 回填实施中的偏差（若有）
- [ ] plan.md 阶段总览状态全部置 completed
- [ ] `cargo tauri build` 出 .dmg 自用

**记录**：

- 自测：
- 用户验收：
- Commit：

---

## Notes

- v3（Notion 联动）的前置条件写在 plan.md 的 Open Questions 里，本 phase 不动。
