# Phase 01 — 升级与隔离开发验证

**Status**: `completed`
**目标**: 交付恢复菜单栏左键行为、可由用户验收的隔离开发版。
**前置**: 用户已批准先升级验证、后正式替换的方案。

## 验收判据

- Tauri 2.12 稳定版编译通过，锁文件包含已修复的托盘依赖。
- 现有 Rust / JS 自动化测试通过。
- 开发版使用 `com.tomatomonster.dev`；`TOMATO_TEST=1` 使用临时进程级 store。
- 左键展开/收起、右键菜单、失焦收起及统计窗口正常。
- 专注 → 提醒 → 休息钉住 → 自然结束 → 下一轮经过运行验证。
- 自测结果、未验证项与用户验收方式明确，正式应用尚未替换。

## Tasks

- [x] 检查工具链、开发身份及正式产物基线，准备可运行的官方临时 CLI。（`rustc --version`；`file /Users/apple/.cargo/bin/cargo-tauri`；`npm install --prefix /private/tmp/tomato-macos27-tools --no-save --package-lock=false --ignore-scripts @tauri-apps/cli@2.12.3`）
- [x] 更新 Tauri 版本约束和必要依赖，复核锁文件差异与上游修复。（`src-tauri/Cargo.toml:19`；`cargo update --manifest-path src-tauri/Cargo.toml -p tauri --precise 2.12.4`；锁定 `tray-icon 0.25.1`）
- [x] 执行 Rust / JS 回归，编译隔离开发版。（`cargo test --manifest-path src-tauri/Cargo.toml --locked`：52/52；`node --test src/stats/notion.test.mjs src/stats/goal-sync-ui.test.mjs`：51/51；临时 CLI `tauri build --debug --bundles app --config src-tauri/tauri.dev.conf.json --no-sign -- --locked` 成功）
- [x] 在隔离开发版验证托盘交互与完整计时周期，记录实际观察。（自动运行周期通过；2026-10-21 用户手动验收通过，明确确认左键面板与休息流程正常）
- [x] 同步架构与验证文档，检查正式产物和数据基线，交付用户验收。（`ARCHITECTURE.md:9`；本文件 Evidence / 用户验收步骤；正式二进制和 store SHA-256、修改时间均与开始时一致）

## Evidence

- 2026-10-21：工作树初始干净；已记录正式二进制与 store 的摘要到系统临时目录（不含数据正文）。Rust 1.97.1 / Apple Silicon；既有 cargo-tauri 为 x86_64，系统拒绝执行。临时官方 CLI 只用于本任务构建。
- 2026-10-21：`cargo test --manifest-path src-tauri/Cargo.toml --locked` **52/52**；`node --test src/stats/notion.test.mjs src/stats/goal-sync-ui.test.mjs` **51/51**。
- 2026-10-21：依赖复核：Tauri 2.12.4、tray-icon 0.25.1、tao 0.37.1、wry 0.57.0、muda 0.20.0。五个直接使用的 Tauri 插件及 rodio 版本不变；项目直接使用的 window-vibrancy 仍为 0.6.0，0.8.1 是新 Tauri 自身的间接依赖。未改产品 Rust/JS 代码。
- 2026-10-21：开发产物 `src-tauri/target/debug/bundle/macos/Tomato Monster Dev.app` 为 arm64，Info.plist 确认 `CFBundleIdentifier=com.tomatomonster.dev`、名称 `Tomato Monster Dev`、最低系统版本 13.0。使用临时 CLI 2.12.3 构建；三个 TOMATO 环境变量均在构建命令中清除，运行测试时才显式设定。
- 2026-10-21：以 `TOMATO_TEST=1 TOMATO_AUTOSTART=1` 运行开发包，读取其进程级临时 store，依次观察到 1.0 秒 FOCUSING / 16.2 秒 BREAKING（completed=1）/ 46.1 秒 FOCUSING（completed=1）。日志依次包含 SoftNudge、Chime、PanelPin、WhiteNoiseStart、WhiteNoiseStop、Chime、PanelUnpin。测试结束只停止本次测试进程。这证明运行时结算与副作用指令已执行，不代表已观察到窗口的真实显示或听到音频。
- 2026-10-21：打包随 Tauri 版本更新了三个已跟踪的生成 schema。`acl-manifests.json` 新增 exit / fullscreen-on-monitor 的 allow/deny **定义**，默认授权集合未改变；`src-tauri/capabilities/default.json` 未修改。
- 2026-10-21：`git diff --check` 通过。正式二进制与 store 的内容摘要、修改时间均与任务开始时一致。没有构建或安装正式版，没有提交。
- 2026-10-21：另启动 `TOMATO_TEST=1`、未设置 `TOMATO_AUTOSTART` 的开发包供交互验收；初始为空闲状态，使用新的临时 store，不复制个人任务、目标或 Notion 配置。

## 用户验收步骤

当前测试实例为 `Tomato Monster Dev`；悬停菜单栏图标可查看名称，右键退出项为「退出 Tomato Monster Dev」。正式图标仍独立存在。

1. 左键点击 Dev 图标应展开番茄主面板，再点一次应收起；展开后点外部应收起。
2. 右键应显示「统计与设置 / 退出 Tomato Monster Dev」，统计窗口应能打开。
3. 开始专注，测试时长为 15 秒；到点进入 30 秒休息，面板自动弹出，点外部或点托盘不应收起。
4. 等待休息自然结束，面板应收起并进入下一轮；可以在专注刚开始时重置计时，或用 Dev 退出菜单结束验收。
5. 反馈上述结果后才将 Phase 01 标为通过，并讨论/执行正式替换。

需要重新启动相同的短周期验收环境时，可运行（只有开发版使用此命令）：

```bash
env -u TOMATO_AUTOSTART -u TOMATO_OPEN_STATS TOMATO_TEST=1 \
  'src-tauri/target/debug/bundle/macos/Tomato Monster Dev.app/Contents/MacOS/tomato-monster'
```

直接双击开发包也使用独立开发身份，但会采用普通 25/5 分钟时长、开发数据目录与首次引导。

## Notes

- 不复制正式 store 到开发环境，不向 Notion 发写请求。
- 验证界面前必须收到 computer use 授权，或由用户手动完成；不能把编译通过当作左键行为已验证。
- 用户验收：2026-10-21 用户选择手动验证，随后回复「验收通过，左键面板和休息流程都正常。可以备份数据并替换正式应用了。」本阶段通过；未使用 computer use。
