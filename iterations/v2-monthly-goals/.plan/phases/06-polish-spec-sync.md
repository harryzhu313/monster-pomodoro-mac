# Phase 06 — 全链路验收与 spec 同步

**Status**: `completed`
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

**Gate**: `user-approved`

**Tasks**：

- [x] 全链路手工验收：设置新增目标 → 面板勾选 → 今日数改为 2 → 设为当前 → 15s 专注结算 → chip `1/3` 锁定 → 30s 休息结束进入下一轮 → 删除目标后任务保留（用户 2026-08-24 逐步真机确认；隔离 store 复核 `goals: []` 且任务 `planned: 2 / used: 1 / goalId` 保留）
- [x] `ARCHITECTURE.md` §4 schema 补 `monthlyGoals` 与 `Task.goalId`（`ARCHITECTURE.md:18-42`）
- [x] `DESIGN.md` 同步固定 360×700、goal chip token 与实际统计窗口布局；`npx @google/design.md lint DESIGN.md` 为 **0 errors**（`DESIGN.md:18-26,109-114,132,140-141,169-170,208`）
- [x] `iterations/v2-monthly-goals/PRD.md` 回填固定高度、任务进度阈值、管理默认值、即时同步/删除保护与 v3 前置状态（`PRD.md:5,23,34,48,59-85,105-106`）
- [x] plan.md 阶段总览状态全部置 completed（`iterations/v2-monthly-goals/.plan/plan.md:30-36`）
- [x] release `.app` 构建完成；使用 Tauri 生成的 DMG 脚本产出并校验 arm64 自用包（`src-tauri/target/release/bundle/dmg/Tomato Monster_0.1.0_aarch64.dmg`）

**记录**：

- 自测（2026-08-24）：
  - `cargo test`：**40 passed / 0 failed**；panel / stats / shared JS module 语法检查、Tauri JSON 校验与 `git diff --check` 均通过
  - 隔离测试配置运行 `TOMATO_TEST=1 TOMATO_AUTOSTART=1 TOMATO_OPEN_STATS=1 cargo tauri dev --no-watch`，日志完整经过「专注 → 最后一分钟提醒 → 休息 → 下一轮专注」；隔离 store 与正式数据分开
  - 发布向环境未设置任何 `TOMATO_*` 测试开关；release 默认仍为专注 25 分钟 / 休息 5 分钟
  - v2 相对基线未修改 `Cargo.toml` / `Cargo.lock`，无新增依赖
  - DESIGN lint：0 errors / 18 warnings；warnings 为既有 token 结构、既有对比度及半透明 goal chip 的静态计算提示，不阻断构建
  - release `.app` 与 arm64 DMG 生成成功；`hdiutil verify` 为 VALID，SHA-256：`68e96fcdce25404580924edd1d612fe90bf6bb23bb15039b1c123aced7d0996d`
  - ARCHITECTURE / DESIGN / PRD 分别经过独立只读复查，无阻断问题；复查建议已回填
- 用户验收：2026-08-24 通过。用户在全新隔离实例中逐步确认目标新增、chip 入今日、计划数修改、专注结算、`1/3` 锁定、休息切轮与删除保留任务；Phase 01 / 02 / 04 / 05 亦已分别验收通过
- Commit：待 Phase 06 验收通过后提交

---

## Notes

- v3（Notion 联动）的前置条件写在 plan.md 的 Open Questions 里，本 phase 不动。
- 当前环境中 `cargo tauri build` 的 Finder 美化步骤未能完成；改用同一构建生成的 `bundle_dmg.sh --skip-jenkins` 跳过装饰布局，产物内容、架构与校验均有效，适合作为自用安装包。
