# Phase 05 — 异常回归、Spec 同步与发布

**Status**: `not started`
**目标**: 月度同步在真实故障与既有功能回归下都可靠，文档、测试和发布包与最终行为一致
**前置**: Phase 04 已验收并提交

## 验收判据

- 错误 DB ID、缺属性、无月份页、多月份页和读取失败全部在 mock/隔离测试中零写入；恢复配置后可正常重试
- 重复复合键与单条写入失败不会误更新错误页面；部分成功后重试最终收口且不新增重复记录
- 本地删除已同步目标后，Notion 历史记录保留；App 不发起 delete/archive 请求
- 既有每日任务连接测试和实际导出字段、relation、去重与反馈行为保持 v1 基线
- Notion 故障期间，本地目标增删改、番茄结算、强制休息和数据持久化仍正常
- Token、数据库 ID 和原始请求内容不进入备份、同步摘要、日志、测试夹具或项目文档
- `cargo test`、JS 单元测试、前端语法检查、Tauri 配置校验、`git diff --check` 和相关 DESIGN lint 全部通过
- `TOMATO_TEST=1 cargo tauri dev` 跑通专注 → 提醒 → 休息 → 下一轮，发布构建确认测试开关关闭并成功产出
- ARCHITECTURE、DESIGN、README、v3 PRD 与 plan/phase evidence 和最终实现一致

## Tasks

- [ ] 补齐预检零写入、分页、重复键、部分失败、摘要失败与重试幂等的自动化回归测试
- [ ] 验证本地删除不产生远端删除请求，并通过用户认可的测试目标确认 Notion 历史仍在
- [ ] 回归每日任务连接测试与一次现有导出流程，逐字段核对既有契约未变化
- [ ] 在 Notion 请求失败条件下跑本地目标保存、番茄结算和完整强制休息周期，证明故障隔离
- [ ] 审计 store、备份、日志、错误文案、测试夹具和 git diff，确认没有敏感配置或个人数据进入仓库
- [ ] 按最终实现增量更新 ARCHITECTURE、DESIGN、README 与 v3 PRD；若没有对应变化，在 evidence 中写明无需修改的依据
- [ ] 运行完整 Rust/JS/配置/格式/DESIGN 检查与 `TOMATO_TEST=1` 全链路测试，记录命令和结果
- [ ] 在无 `TOMATO_*` 测试变量的发布环境执行 `cargo tauri build`，验证 `.app` / 自用安装包可打开且同步 UI 正常
- [ ] 对照五个 phase 的验收记录完成最终 spec 审查，并把 plan 总览更新为真实状态

## Notes

- 真实 Notion 数据的删除或清理由用户决定；本 Phase 不擅自删除任何页面。
- 涉及真实月度同步或每日任务导出的回归步骤，执行前再次说明会修改用户现有 Notion 页面并由用户亲自触发；evidence 只记录计数与结论。
- 若实施中发现需要新依赖、Notion API 升级或范围外迁移，先停下与用户讨论并更新 ARCHITECTURE / plan，不在本 Phase 顺手加入。
- 迭代文档不得记录本机网络环境详情；真实 Token、数据库 ID、页面 ID和个人目标标题只用于当前验收，不写入 evidence。
