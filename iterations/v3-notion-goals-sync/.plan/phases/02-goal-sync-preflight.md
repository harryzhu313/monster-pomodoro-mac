# Phase 02 — 月度目标同步预检

**Status**: `completed`
**目标**: 在零写入前提下验证目标库与时间框架契约，并唯一定位 Rust 快照日期所属的月份页面
**前置**: Phase 01 已验收并提交

## 验收判据

- 目标库六个必需属性的名称与类型全部匹配：`名称:title`、`分类:select`、`本月总量:number`、`已完成:number`、`所属月:relation`、`本地目标 ID:rich_text`；额外分析属性不影响通过
- 「分类」选项严格包含工作 / 学习 / 生活 / 兴趣爱好；「所属月」指向可读取的时间框架，并能确认对应双向 relation 属性存在
- 不增加时间框架 DB ID；App 从 relation schema 自动取得查询目标，并验证 `类型:select` 含「月」与 `日期:date`
- 使用 Rust 快照的 `today`，在分页结果中只接受 `类型 = 月` 且日期区间包含今天的唯一页面；0 条或多条均停止并给出不同错误
- 正确配置、缺属性、错类型、错 relation、无月份页、多月份页均有自动测试；所有预检路径都没有 Notion 写请求
- 月度预检失败不改变本地目标、Notion 配置或每日任务导出状态

## Tasks

- [x] 抽出可测试的 Notion 数据库属性解析与错误模型，为每个属性输出名称、预期类型和实际类型（`src/stats/notion.js:101-168`；`src/stats/notion.test.mjs:332-413`）
- [x] 实现目标库六个必需属性、四分类常量、relation 指向与双向属性的只读校验，同时允许不冲突的额外属性存在（`src/stats/notion.js:20-27,145-157,193-294,432-445`；`src/stats/notion.test.mjs:296-359,415-540`）
- [x] 从「所属月」relation 取得时间框架查询目标，并校验 `类型`、`日期`及「月」选项（`src/stats/notion.js:193-294,437-445`；`src/stats/notion.test.mjs:321-534`）
- [x] 分页查询 `类型 = 月` 的时间框架页面，再以 Rust 快照 `today` 在本地判断日期区间包含关系（`src/stats/notion.js:296-353,447-488`；`src/stats/stats.js:949-962`；`src/stats/notion.test.mjs:543-605`）
- [x] 将目标库、时间框架和唯一月份页结果接入月度连接反馈，错误文案明确下一步应修改哪一项（`src/stats/notion.js:360-489,510-529`；`src/stats/stats.js:914-962`）
- [x] 使用 mock HTTP 覆盖成功与各类失败，断言预检期间只出现 GET/query 而没有 create/update 请求（`src/stats/notion.test.mjs:124-133,188-605`；`node --test src/stats/notion.test.mjs` 17 passed）
- [x] 在真实数据库执行一次只读预检，核对前后记录数不变并记录不含敏感信息的 evidence（用户 2026-08-25 提供成功截图：schema、时间框架与唯一 `2026-08` 月页面均通过；随后只读 `COUNT(*)` 为 0，与清理后的空库基线一致）

## Notes

- Notion API relation schema不暴露 UI 的“限制为一个页面 / 无限制”设置。本阶段只验证指向和双向属性；数量限制沿用用户已确认的人工前置条件，App 写入时始终只提交一个「所属月」。
- 只读预检不能证明写权限。写权限只能在 Phase 04 的真实 create/update 中验证。
- 日期判断不得读取前端系统时钟；必须消费 Rust 快照日期，避免跨午夜时前后端口径漂移。
- 自测（2026-08-25）：`cargo test` **44 passed / 0 failed**；`node --test src/stats/notion.test.mjs` **17 passed / 0 failed**；两份 JS module check、`git diff --check` 与 Cargo 依赖零改动检查均通过。
- 自动测试明确只允许 `GET /databases/{id}` 与只读 `POST /databases/{id}/query`，并精确限制预检 IPC 调用；每日任务连接与导出回归测试保持通过。
- 独立代码复审已覆盖 relation 双向 reciprocal ID、Rust `Snapshot.today` 单次捕获、分页、错误映射和每日导出隔离，无 P0 / P1 / P2 遗留。
- 真实验收（2026-08-25）：首次误填时间框架库 ID 时，预检在「分类」缺失处按设计阻止继续；改为实际月度目标库 ID 后，App 成功定位唯一当月页面。只读核对目标库记录数仍为 0，确认预检未创建或更新页面。
- 用户验收（2026-08-25）：**通过**。Phase 02 封存，尚未开始 Phase 03。
- 完成提交：`4427997`（完整 hash：`4427997513003f798a4a3ed9e943483b688655df`）。以上行号按该 revision 解释。
