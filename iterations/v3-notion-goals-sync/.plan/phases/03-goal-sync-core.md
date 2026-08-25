# Phase 03 — 月度目标幂等同步核心

**Status**: `completed`
**目标**: 用稳定复合键实现可测试、可重试的当前月目标 upsert，不依赖真实 Notion 写入即可证明核心正确
**前置**: Phase 02 已验收并提交

## 验收判据

- 同步 payload 只消费 Rust 提供的 `today` 与 `GoalView.id/title/category/monthTarget/monthDone`，前端不扫描任务、不重新计算进度
- 当前月无本地目标时返回明确空结果，不发送任何 Notion query/create/update 请求
- 分页读取当月远端记录并以 `所属月 + 本地目标 ID` 建索引：不存在则创建一页，恰有一页则更新，存在多页则该目标失败且不修改任一重复页
- 创建与更新都写入六个目标属性；改名、改分类、改计划量或进度后仍更新原复合键记录
- Rust 快照若意外包含四个产品常量之外的分类，该目标明确失败且不写入，不得回退或猜测为「工作」
- 单条失败不阻断其余目标；结果稳定汇总为新增 N / 更新 N / 失败 N，修复后重复执行不会新增重复页
- 一次同步只读取一次配置并冻结；执行中修改设置不会混用 Token 或数据库
- 最近一次摘要以 `monthly:YYYY-MM` 保存时间和计数，不含 Token、DB ID、标题、page ID 或原始错误响应
- Node 内置测试器覆盖新建、更新、改名、跨月、分页、重复键、空目标、部分失败与安全重试；`cargo test` 全绿

## Tasks

- [x] 定义纯函数形式的月份键、标题、六属性 payload 与远端记录解析，分类只接受项目四个常量，无效值返回该目标错误（`src/stats/notion.js:537-652`）
- [x] 实现按唯一月份 relation 分页查询远端记录，并按本地目标 ID 建立 `0 / 1 / many` 索引（`src/stats/notion.js:609-704`）
- [x] 实现 create/update 编排、逐条错误隔离、重复键冲突和确定性的新增/更新/失败摘要（`src/stats/notion.js:741-836`）
- [x] 在同步入口冻结 Notion 配置和 Rust 目标快照，禁止循环中再次读取可变配置或系统日期（`src/stats/notion.js:549-574,741-761`）
- [x] 将最近一次月度摘要写入 `notionExportLog` 的月份命名空间，并保证摘要记录失败不会把已完成的远端写入误报为整体失败（`src/stats/notion.js:717-735`）
- [x] 使用可注入 mock fetch 编写无新依赖的 JS 单元测试，覆盖验收判据中的所有分支（`src/stats/notion.test.mjs:689-1165`）
- [x] 运行 JS 测试、Rust 测试、前端语法检查和 `git diff --check`，记录具体 evidence（`node --test src/stats/notion.test.mjs`：31/31；`cargo test`：44/44；完成提交 `500e3f7`）

## Notes

- 本阶段只在 mock HTTP 上验证写入，不向真实 Notion 创建或更新页面；真实闭环留到 Phase 04。
- 不保存 `{month, goalId} → pageId` 本地映射。远端查询结果是每次同步的事实来源，可避免 page 被用户移动或删除后产生陈旧映射。
- 本地删除目标时，目标不会进入后续 payload；核心不得生成删除或归档远端页面的请求。
- 不复用标题去重。本地目标允许改名，标题只用于展示。
- 自测（2026-08-25）：`node --test src/stats/notion.test.mjs` **31 passed / 0 failed**；`cargo test` **44 passed / 0 failed**；ES module 语法检查、`git diff --check` 与 Cargo 依赖零改动检查均通过。
- mock 覆盖首次新建、同月更新与改名、跨月同 ID、新建响应丢失后的安全重试、远端与本地重复 ID、跨页重复、畸形后续分页零写入、空目标、非法分类、部分失败、孤儿页不删除、固定 2022 请求头及失败摘要脱敏。
- 配置与 Rust payload 在首次 await 前后分别复制冻结；测试在执行中篡改原对象，全部请求仍使用同一 Token、目标库、月份和五个 GoalView 字段，且不扫描任务数据。
- 最近摘要只向 `monthly:YYYY-MM` 写入 `created / updated / failed`；Rust 继续负责追加时间。日志为 best-effort，不对外承诺磁盘落盘状态，也不改变已发生的远端计数。
- 独立代码、测试与安全复审均无 P0 / P1 / P2 遗留。用户于 2026-08-25 验收通过；尚未接入 UI，也未向真实 Notion 写入任何记录。
- 完成提交：`500e3f7`（完整 hash：`500e3f715dee03cd0fd0318525c7d1ecaf0a276f`）。以上行号按该 revision 解释。
