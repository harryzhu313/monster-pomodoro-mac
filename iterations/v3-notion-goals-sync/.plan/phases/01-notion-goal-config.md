# Phase 01 — 月度目标 Notion 配置

**Status**: `completed`
**目标**: 安全增加月度目标 DB ID 与独立连接检查，升级后任何旧配置和每日任务导出都不受损
**前置**: v3 PRD 与 plan 已确认

## 验收判据

- 旧 `store.json` 没有 `goalDbId` 时可正常加载，界面显示为空，不报数据损坏
- 填入月度目标 DB ID 后，关闭并重开 App 仍保留；以后修改任一 Notion 配置字段都不会清空其它字段
- 每日任务配置与月度目标配置独立给出结果；目标 DB ID 为空或错误不会阻断既有每日任务连接测试和导出入口
- 月度目标基础连接检查能在现有 API 版本下只读取得数据库信息，错误 Token / DB ID 显示可行动提示，Notion 中没有记录变化
- 配置与迁移相关 Rust 测试、前端语法检查和 `git diff --check` 全部通过

## Tasks

- [x] 为 Rust `NotionConfig` 增加默认空值的 `goalDbId`，并补旧配置加载、序列化 round-trip 与敏感配置不进入备份的测试（`src-tauri/src/store.rs:197-234,466-475,500-583,659-768`；`cargo test` 44 passed）
- [x] 将配置保存改为不会因旧调用缺少新字段而覆盖其它值的合并式契约，并覆盖清空单个字段与保留其余字段的测试（`src-tauri/src/store.rs:208-233,301-310,681-768`；`src-tauri/src/lib.rs:360-368`）
- [x] 在「Notion 导出」卡增加月度目标 DB ID 输入框，沿用现有密码/ID 输入、自动保存和反馈模式（`src/stats/stats.html:133-170`；`src/stats/stats.js:65-72,861-912`）
- [x] 拆分每日任务与月度目标的连接结果，使两套配置可独立检查、独立失败（`src/stats/stats.html:158-167`；`src/stats/stats.js:833-957`；`src/stats/notion.test.mjs:74-118`）
- [x] 用现有 raw HTTP 通道实现月度目标库的基础只读连接检查，并冻结一次检查所使用的配置快照（`src/stats/notion.js:45-128`；`src/stats/notion.test.mjs:74-147`）
- [x] 运行 Rust 测试、前端模块语法检查和格式检查，记录具体 evidence（`cargo test` 44 passed；`node --test src/stats/notion.test.mjs` 5 passed；两份 JS module check 与 `git diff --check` 通过）

## Notes

- 本阶段不校验六属性完整 schema，不查询月份页面；这些属于 Phase 02。
- 本阶段禁止向 Notion 创建或更新页面；基础检查只允许读取数据库元数据。
- 暂不升级 `Notion-Version: 2022-06-28`。官方说明旧单数据源连接可继续工作，但相关旧接口已弃用；迁移必须放入独立迭代，避免与每日任务导出回归混在一起。
- 配置、测试输出和 phase evidence 不得记录真实 Token、数据库 ID 或用户本机网络信息。
- 自测（2026-08-25）：
  - `cargo test`：**44 passed / 0 failed**；新增覆盖旧三字段配置、四字段 round-trip、Patch 省略保留 / 空串清除、真实 save→reload、保存失败内存回滚、备份不含四项敏感配置且导入不覆盖本机配置。
  - `node --test src/stats/notion.test.mjs`：**5 passed / 0 failed**；覆盖两套连接独立、连接请求只读、空配置零请求、配置快照冻结和每日导出不访问目标库。
  - `node --input-type=module --check`（`notion.js` / `stats.js`）、`git diff --check`、Cargo 依赖零改动检查均通过。
  - 以统计窗口实际 **680×760** 视口检查 Notion 卡：四个输入框、两个 secondary 测试按钮与两个 live status 均可见；空配置分别测试时结果互不覆盖，浏览器控制台无错误。
  - `cargo tauri dev --no-watch` 构建并启动成功。全库 `cargo fmt --check` 会触发既有 Rust 文件的大范围重排，因此未据此制造无关 diff；本次 Rust 变更已通过编译、测试、diff 格式检查与独立审查。
- 用户验收（2026-08-25）：**通过**。用户完成本机检查后明确确认验收通过；Phase 01 封存，尚未开始 Phase 02。
