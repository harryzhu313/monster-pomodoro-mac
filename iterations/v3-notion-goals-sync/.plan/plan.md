# 让本月目标一键成为可重复更新的 Notion 月度快照

> 文件位置：`iterations/v3-notion-goals-sync/.plan/plan.md`
> 配套 skill：first-flight-phases
> 本 plan 文档是**稳定航图**，状态跟踪在各 phase 文档（同目录的 `phases/NN-*.md`）里。

## 背景

v2 已让月度目标、今日任务和实际番茄在本地形成闭环，但月度复盘仍要手工抄到 Notion。v3 将 App 当前自然月的目标作为月度快照写入用户已经建立的「月度番茄目标」数据库，并挂到唯一的时间框架月页面。App 继续是唯一事实来源；同步失败必须与本地目标保存、计时和强制休息完全隔离。

## 范围

**做：**

- 为既有 Notion 配置增加月度目标 DB ID，并保持旧配置与每日任务导出兼容
- 只读校验目标库 schema、relation 指向、时间框架 schema 与唯一当月页面
- 将 Rust 快照中的当前月目标按 `所属月 + 本地目标 ID` 幂等创建或更新
- 在月度目标卡提供手动同步、过程状态、结果摘要和可行动错误
- 保存不含敏感信息的最近一次月度同步摘要，并完成异常、回归、文档和发布验收

**不做：**

- 自动或后台同步、历史月份选择与补同步
- Notion → App 导入或双向同步
- 自动创建时间框架页面，或自动删除、归档 Notion 记录
- 改造既有每日任务导出的字段、去重规则或触发方式
- 在本迭代升级 Notion API 版本或引入新 crate / npm 依赖
- 月度分析看板、目标排序或新增激励功能

## 阶段总览

| # | 阶段 slug | 一句话目标 | 状态 |
|---|---|---|---|
| 01 | notion-goal-config | 安全增加目标库配置与独立连接检查，不清空旧配置 | completed |
| 02 | goal-sync-preflight | 只读验证完整 schema，并定位唯一的当前月页面 | completed |
| 03 | goal-sync-core | 用稳定复合键实现可测试、可重试的幂等 upsert 核心 | not started |
| 04 | goal-sync-ui | 接入同步 UI，并在真实 Notion 中完成新增与更新闭环 | not started |
| 05 | hardening-spec-release | 完成异常回归、spec 同步、全链路测试与发布构建 | not started |

> 状态值：`not started` / `in progress` / `completed` / `blocked` / `skipped`
>
> 详细任务、evidence、blocker 在各 phase 文档（`phases/NN-<slug>.md`）里，**不在本表里展开**。

## 关键决策

- **2026-08-25**：只同步当前自然月，且只由用户手动触发；App 是唯一事实来源，Notion 修改不回写本地。
- **2026-08-25**：不新增时间框架 DB ID。App 从目标库「所属月」relation schema 自动取得时间框架查询目标，取得失败即停止。
- **2026-08-25**：Notion API 不暴露 relation 的单页 / 多页 UI 限制，也没有无副作用的写权限探测；App 自动验证 relation 指向、双向属性与读取权限，数量限制由用户在 Notion 中确认，写权限错误归入逐条写入结果。
- **2026-08-25**：Rust 的 `GoalView` 与快照日期是同步 payload 的唯一业务数据源；前端只负责 schema/query/upsert 和显示，不扫描任务或读取系统时间重新计算进度。
- **2026-08-25**：沿用现有前端 `tauri-plugin-http` 通道和 `Notion-Version: 2022-06-28`，先保护每日任务导出；Phase 01/02 做真实只读兼容验证，API 迁移留给独立迭代。
- **2026-08-25**：远端身份使用 `所属月 + 本地目标 ID`。每次同步分页查询远端现状，不保存 page ID 映射；同一复合键已有多条时，该目标失败并提示清理，不猜测更新其中一条。
- **2026-08-25**：读取权限、schema、月份页属于零写入预检；进入写入阶段后允许单条部分失败，重复点击依靠同一复合键安全收口。
- **2026-08-25**：最近一次摘要复用 `notionExportLog`，使用 `monthly:YYYY-MM` 命名空间，只保存时间与新增/更新/失败计数，不保存 Token、DB ID、标题或请求内容。
- **2026-08-25**：五个 Phase 串行推进；每个 Phase 自测后停下，由用户验收并决定是否提交和进入下一阶段。

## Open Questions

- [x] 月度记录粒度、字段映射、同步方向与当前月范围 —— 用户 2026-08-25 确认
- [x] Relation 两侧数量限制与空白记录 —— 用户已在 Notion 调整并清理
- [x] Notion 数据库 schema —— 已只读核对，六个目标属性和时间框架属性就位
- [ ] App 当前使用的 Integration 是否同时具备两个数据库的读取与写入权限 —— Phase 02 已验证两库读取；Phase 04 首次写入验证；Token 只在 App 本机填写

## 关联

> 路径相对当前文件位置 `iterations/v3-notion-goals-sync/.plan/plan.md`：

- 长期文档（项目根）：[BRIEF.md](../../../BRIEF.md) / [DESIGN.md](../../../DESIGN.md) / [ARCHITECTURE.md](../../../ARCHITECTURE.md) / [AGENTS.md](../../../AGENTS.md)
- 当前迭代 PRD：[PRD.md](../PRD.md)
- 首版内容规范：[CONTENT.md](../../v1-launch/CONTENT.md)
- 上一迭代计划：[v2 plan.md](../../v2-monthly-goals/.plan/plan.md)
