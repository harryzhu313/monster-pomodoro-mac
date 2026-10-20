# Phase 02 — 验收后的正式替换

**Status**: `completed`
**目标**: 用户通过开发版验收后，将修复部署到唯一正式入口。
**前置**: 用户于 2026-10-21 明确验收 Phase 01，并授权备份和正式替换。

## 验收判据

- 正式数据已备份，发布构建清除全部 TOMATO 测试变量。
- 已验证产物替换到 `/Applications/Tomato Monster.app`，数据与配置延续。
- 用户确认正式应用菜单栏交互和数据正常。

## Tasks

- [x] 记录用户验收结论并备份正式数据与现有应用。（2026-10-21 用户明确通过；`~/Library/Application Support/Tomato Monster Backups/2026-10-21-071635/`，应用 ZIP CRC、store JSON 和 SHA-256 校验通过，目录 0700 / 文件 0600）
- [x] 构建、核验正式 arm64 产物（不带开发身份与测试环境变量）。（下方正式构建命令成功；Info.plist 为 `com.tomatomonster.app`、名称 `Tomato Monster`；Mach-O arm64；内嵌依赖路径确认 Tauri 2.12.4 / tray-icon 0.25.1）
- [x] 替换并启动唯一正式入口，确认原数据及菜单栏行为。（安装、启动、数据和环境校验完成；2026-10-21 用户确认正式版左键面板和原数据正常，最终验收通过）
- [x] 更新发布与验收记录。（`ARCHITECTURE.md:9`；本文件 Evidence；`.plan/plan.md` 当前进度）

## Notes

2026-10-21：Phase 01 已由用户手动验收通过，本阶段经用户授权完成备份和正式替换。验证采用文件、CLI 与只读日志检查，未启用 computer use；正式应用界面由用户手动复核并验收通过。

## Evidence

- 2026-10-21：先保存 `Tomato Monster-before.zip` 与 `data-before-build/store.json`，备份包含完整本地配置，仅保存在用户私有目录。manifest 记录旧二进制、数据和归档摘要；不复制敏感内容到仓库。
- 正式构建命令：`env -u TOMATO_TEST -u TOMATO_AUTOSTART -u TOMATO_OPEN_STATS -u TAURI_CONFIG /private/tmp/tomato-macos27-tools/node_modules/.bin/tauri build --target aarch64-apple-darwin --bundles app --no-sign -- --locked`。只生成本次本机替换所需的 .app，不运行 DMG 的 Finder 布局流程。
- 2026-10-21：正式构建成功，产物为 `src-tauri/target/aarch64-apple-darwin/release/bundle/macos/Tomato Monster.app`，约 20.18 MiB。既有 identifier 的 `.app` 后缀建议警告保持原状，以延续正式数据目录。
- 2026-10-21：先将产物复制到 `/Applications` 的临时目录并逐文件校验；仅退出核对过可执行路径的旧正式版和 Dev 实例，再保存 `data-before-install/store.json`。旧应用保留在备份目录的 `previous-app/`，新产物替换到 `/Applications/Tomato Monster.app`；替换过程有失败回滚分支，没有覆盖或迁移 store。
- 2026-10-21：安装目录与构建产物的所有文件内容和符号链接一致；正式二进制 SHA-256：`abf9ae2513333c845f23375620589c06cc4999161cbe789c4a62802209bcee1b`。
- 2026-10-21：通过 LaunchServices 从 `/Applications/Tomato Monster.app` 启动，显式设置三个 TOMATO 开关为 `0`；只读进程检查确认唯一 `tomato-monster` 实例来自正式入口，父进程为 1，三个环境开关均为 `0`。启动 stdout/stderr 为空，没有测试模式或 panic 输出。
- 2026-10-21：启动后的正式 store 与 `data-before-install/store.json` 逐字节及 JSON 内容均一致，包括历史、任务、目标、设置和 Notion 配置。原始配置只存在本机数据与受保护备份中，未打印到工具输出或写入迭代文档。
- 2026-10-21：`git diff --check` 通过；正式替换时未创建 Git commit。实际菜单栏点击由用户手动复核，未启用 computer use。最终验收后，用户另行授权将修复和验收记录一并提交。
- 2026-10-21：用户在当前对话明确反馈「正式版验收通过，左键面板和原数据都正常」，完成正式入口的最终人工验收。

## 交付位置

- 正式应用：`/Applications/Tomato Monster.app`
- 备份：`~/Library/Application Support/Tomato Monster Backups/2026-10-21-071635/`
- 完整数据快照：`data-before-install/`（替换前最后快照），`data-before-build/`（构建前快照）
- 旧应用：`Tomato Monster-before.zip` 与 `previous-app/`（原包目录，避免再以同名 .app 暴露入口）
- 备份摘要：`manifest.json`

## 最终验收

2026-10-21 验收通过。开发版的左键面板和休息流程已获用户确认；正式应用替换、启动与数据连续性检查均完成，用户进一步确认正式版左键面板和原数据正常。本阶段与 v4 迭代均标记为 completed。
