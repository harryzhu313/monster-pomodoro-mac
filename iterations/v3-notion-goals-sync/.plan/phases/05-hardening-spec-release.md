# Phase 05 — 异常回归、Spec 同步与发布

**Status**: `completed`
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
- `TOMATO_TEST=1 cargo tauri dev --config src-tauri/tauri.dev.conf.json` 跑通专注 → 提醒 → 休息 → 下一轮；发布构建清除三个 `TOMATO_*` 环境变量并成功产出
- ARCHITECTURE、DESIGN、README、v3 PRD 与 plan/phase evidence 和最终实现一致

## Tasks

- [x] 补齐预检零写入、分页、重复键、部分失败、摘要失败与重试幂等的自动化回归测试（`src/stats/notion.test.mjs:719-1469`；`node --test src/stats/notion.test.mjs src/stats/goal-sync-ui.test.mjs`：51/51）
- [x] 验证本地删除不产生远端删除请求，并通过用户认可的测试目标确认 Notion 历史仍在（`src/stats/notion.test.mjs:1195`；2026-08-25 用户真实 Notion 验收正常）
- [x] 回归每日任务连接测试与一次现有导出流程，逐字段核对既有契约未变化（`src/stats/notion.test.mjs:1473,1576,1629,1682,1724`；2026-08-25 用户真实导出验收正常）
- [x] 在 Notion 请求失败回归中确认同步零写入且可恢复，并在隔离 store 跑完整番茄与强制休息周期，联合证明故障隔离（`src/stats/notion.test.mjs:719-798`；`TOMATO_TEST=1 cargo tauri dev --config src-tauri/tauri.dev.conf.json` 全周期通过）
- [x] 审计 store、备份、日志、错误文案、测试夹具和 git diff，确认本 Phase 新增内容没有敏感配置或真实验收数据进入仓库（`src-tauri/src/store.rs:354-389,690-734`；`src-tauri/src/lib.rs:473-545`；隐私 pattern 审计与 `git diff --check` 通过）
- [x] 按最终实现增量更新 ARCHITECTURE、DESIGN、README 与 v3 PRD；若没有对应变化，在 evidence 中写明无需修改的依据（`ARCHITECTURE.md:16-21,38-44,94-119`；`DESIGN.md:160,219`；`README.md:55-78`；`iterations/v3-notion-goals-sync/PRD.md:52-139`）
- [x] 运行完整 Rust/JS/配置/格式/DESIGN 检查与 `TOMATO_TEST=1` 全链路测试，记录命令和结果（Rust 52/52；JS 51/51；四个 ES module、修改过的 Rust 文件、两份 Tauri JSON、DESIGN lint 与 `git diff --check` 通过）
- [x] 在无 `TOMATO_*` 测试变量的发布环境执行 `cargo tauri build`，验证 `.app` / 自用安装包可打开且同步 UI 正常（无测试变量的 arm64 build + `hdiutil verify` 通过；2026-08-25 用户实际打开与同步 UI 验收正常）
- [x] 清理陈旧 release / debug 进程，将最终包固定安装到 `/Applications`，并用独立名称、Bundle ID 与数据目录隔离开发版（`src-tauri/tauri.dev.conf.json:1-5`；`src-tauri/src/lib.rs:24-55,823-826`；added 2026-08-25：用户确认并验收方案 B）
- [x] 对照五个 phase 的验收记录完成最终 spec 审查，并把 plan 总览更新为真实状态（Phase 01–04 完成提交 `0413f02 / 4427997 / 500e3f7 / f3a56d2`；最终跨 phase/spec/发布审查无遗留 blocker）

## Evidence

- 2026-08-25：`node --test src/stats/notion.test.mjs src/stats/goal-sync-ui.test.mjs` 为 **51/51**；覆盖损坏/重复分页游标零写入、错误 DB 恢复、缺失/多个月页、远端重复键、create/PATCH 部分失败、页面身份隔离和重试收口。新增开发身份保护后，`cargo test --manifest-path src-tauri/Cargo.toml` 最终为 **52/52**。
- 删除保护的 mock 契约确认本地目标缺席时不会对远端孤儿页发送 update/delete/archive；生产 `notion.js` 不存在 DELETE、archive 或 in-trash 路径。2026-08-25 用户使用认可的测试目标完成真实验收，确认本地删除并再次同步后 Notion 历史仍保留。
- 每日导出自动化已逐字段覆盖既有 title/select/number/date/relation、分页、同日同名更新、无日页面、部分/全部失败和成功/部分/失败三态文案；2026-08-25 用户完成每日库连接、实际导出与 Notion 字段核对，反馈全部正常。
- 独立测试 identifier + 系统临时隔离 store 下运行 15 秒专注 / 30 秒休息：依次观察到最后一分钟提示、到点提示、面板钉住、白噪音开始，以及休息自然结束后的白噪音停止、提示、解除钉住和下一轮；未读写正式 App 数据。
- Notion 日志在 IPC、旧 store 加载迁移、备份导入/导出四个边界执行安全白名单；浏览器 mock 不打印参数；store 临时文件和用户备份均在写入前设 0600；默认备份与误拷贝的 store 文件名已加入精确 `.gitignore`。审计未发现真实 Token、数据库/页面 ID、请求 payload 或远端删除路径进入本 Phase diff/evidence。既有 v2 演示文案与 Git 历史不属于本次新采集的验收数据；若未来公开仓库，另行决定是否做全历史隐私清理。
- 前端四个 ES module 语法检查、`git diff --check`、Tauri JSON 配置解析和 `npx --no-install @google/design.md lint DESIGN.md` 均通过；`cargo tauri info` 可读取当前配置，完整 Xcode 缺失不构成阻塞，实际 bundle 已由 Command Line Tools 成功产出。
- 清除三个 `TOMATO_*` 变量后显式构建 Apple Silicon 包成功：`.app` 内实际包含月度同步前端资产，Mach-O 为 arm64，Info.plist 的 `LSMinimumSystemVersion` 为 `13.0`；最终 aarch64 DMG 为约 6.5 MB，`hdiutil verify` 校验有效。既有 bundle identifier 的 Tauri 建议警告为保持 app data 路径兼容而暂不迁移；最终包的实际启动与同步 UI 已由用户验收。
- 方案 B 已落地：陈旧 release / debug 进程均正常退出，已验证的最终包按字节一致复制到 `/Applications/Tomato Monster.app`，并切换为唯一正式运行入口；未删除 store 或构建产物。新增 `tauri.dev.conf.json` 将开发版固定为 `Tomato Monster Dev` / `com.tomatomonster.dev`，Rust 在插件初始化和 app data 路径解析前拒绝 debug 使用正式 identifier，托盘提示与退出菜单也读取当前 product name。隔离开发命令实启成功并随后完全退出，正式版始终独立运行。新增身份保护测试后 `cargo test --manifest-path src-tauri/Cargo.toml` 为 **52/52**，月度同步 JS 回归仍为 **51/51**，两个 Tauri 配置 JSON 与 `git diff --check` 均通过。
- 2026-08-25 用户验收：`/Applications` 正式入口、月度目标 Notion 导出功能与原有历史数据均正常，发布包实际打开与同步 UI 验收通过。
- 最终 spec 审查修正了统计页旧隐私文案：Token 仅本机持久化，请求会作为 Authorization 直连 Notion API，但不经过中间服务器；该口径现与 README / ARCHITECTURE / 实现一致。修正后重新从当前工作树构建 `.app` / DMG，安装二进制与构建产物 SHA-256 同为 `423a359b56e893e921aee676797ae7d6a91357b03632334cfaf2db0285e69783`，并已从 `/Applications/Tomato Monster.app` 成功启动。
- 最终工作树复核：Rust **52/52**、JS **51/51**；四个 ES module 语法、修改过的 `lib.rs / store.rs` rustfmt、两份 Tauri JSON、`git diff --check`、spec 隐私 pattern 扫描均通过。DESIGN.md 自上次成功 lint 后未再变化；本次环境缺少本地 lint 包且受限网络无法复跑，因此不虚构新的 lint 结果。全仓 `cargo fmt --check` 仍会命中本迭代未修改的既有 `audio.rs / timer.rs` 排版，Phase 05 只验证并格式化了本次修改的 Rust 文件，未制造无关格式 diff。
- 2026-08-25 最终用户验收：用户检查由当前工作树重建并重新安装的 `/Applications/Tomato Monster.app` 后明确回复“验收通过”；Phase 05 与 v3 迭代正式通过验收，提交状态以 Git 历史为准。

## Notes

- 真实 Notion 数据的删除或清理由用户决定；本 Phase 不擅自删除任何页面。
- 涉及真实月度同步或每日任务导出的回归步骤，执行前再次说明会修改用户现有 Notion 页面并由用户亲自触发；evidence 只记录计数与结论。
- 若实施中发现需要新依赖、Notion API 升级或范围外迁移，先停下与用户讨论并更新 ARCHITECTURE / plan，不在本 Phase 顺手加入。
- 迭代文档不得记录本机网络环境详情；真实 Token、数据库 ID、页面 ID和个人目标标题只用于当前验收，不写入 evidence。
- 三个 `TOMATO_*` 仍是显式运行时开发钩子，不会被构建过程从二进制移除；最终构建命令清除了三者，正常 Finder 启动时也不会自行激活。release 构建并未被误记为“编译期移除测试代码”。
