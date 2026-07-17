# Phase 01 — 脚手架与技术 spike

**Status**: `completed`
**目标**: Tauri 2 骨架在本机跑起来（托盘 + 空面板 + 测试音），4 项技术风险 spike 得出 go / 降级结论
**前置**: 无

## 验收判据

phase 完成 = **下面所有判据全部满足**：

- `cargo tauri dev` 启动无报错，菜单栏出现图标与每秒跳动的 `mm:ss` 标题
- 点托盘图标弹出 / 隐藏带 vibrancy 毛玻璃效果的空面板，失焦自动隐藏
- 能听到 rodio 播放的测试音（白噪音数秒 + chime 双音）
- Notes 区有 4 项 spike 的书面结论（go / 降级方案），plan.md Open Questions 对应项勾掉

## Tasks

- [x] 检查工具链：rustup / rustc / Xcode CLT / cargo tauri-cli（缺则装）(rustc 1.97.1 + cargo 1.97.1 经官方 rustup 安装；tauri-cli 2.11.4 经 cargo-binstall 装预编译二进制；Xcode CLT 已存在 `/Library/Developer/CommandLineTools`)
- [x] 生成 `src-tauri/` 骨架，按 ARCHITECTURE §7 布局前端目录 `src/`（panel / stats / celebration / shared）(src-tauri/src/lib.rs:1-107, src-tauri/src/audio.rs:1-90, src/panel/panel.html; `cargo build` 一次通过 1m08s)
- [x] `tauri.conf.json`：`withGlobalTauri: true`、`macOSPrivateApi: true`、`frontendDist` 指向 `src/` (src-tauri/tauri.conf.json:9-17, 另加严格 CSP 与 capabilities/default.json)
- [x] 托盘：TrayIcon + `iconAsTemplate: true`，占位模板图标，每秒刷新 `mm:ss` 标题（Rust 侧计时循环）(src-tauri/src/lib.rs:47-83; 占位图标 icons/tray-idle.png 由脚本生成)
- [x] 主面板窗口：无边框 300px、`transparent: true`、失焦即隐、点托盘 toggle、`tauri-plugin-positioner` 定位到托盘下方 (tauri.conf.json windows[0], lib.rs:15-26 toggle_panel, lib.rs:87-93 失焦即隐)
- [x] Spike 1：`window-vibrancy` 施加 NSVisualEffectView，验证透明 + 无边框 + vibrancy 组合 (lib.rs:36-45 用 expect 做 no-go 探针,进程启动无 panic = apply_vibrancy 成功;视觉效果待用户确认)
- [x] Spike 2：托盘秒刷功耗实测（CPU 均值 < 0.5%），写结论 (debug 构建 PID 2624,30 秒 ps 采样 10 次:0.0–0.3%,均值 ~0.1% —— **GO,保留秒级刷新**)
- [x] Spike 3：rodio 合成测试音（白噪音 + chime 双音），主观判断音色，写结论 (首版纯白噪音被用户否决;对齐旧版棕噪音算法后 2026-07-17 用户验收通过 —— GO)
- [x] Spike 4：positioner 托盘相对定位行为（如有多显示器则测多显示器），写结论 (2026-07-17 用户实测面板弹出位置正确 —— GO)
- [x] 4 项 spike 结论汇总写入本文档 Notes，给出 go / 降级决定 (见 Notes,4 项全部 GO,无降级)
- [x] 白噪音算法对齐旧版棕噪音(一阶低通 + ×3.5 补偿),chime 对齐 E5→B5 指数衰减重叠双音 (added 2026-07-17: 用户反馈白噪音不如旧版;src-tauri/src/audio.rs:17-63 BrownNoise / :66-115 EnvSine,逐行对齐旧 offscreen.js:19-96)

> **状态符号**（直接修改 `[ ]` 内的字符）：
>
> - `[ ]` 待办 · `[~]` 进行中 · `[x]` 已完成（行尾必须附 evidence）· `[-]` 跳过（仅用户明确同意）· `[!]` 受阻（Notes 写清 blocker）

## Notes

### Spike 结论（2026-07-17，AI 自测部分）

| Spike | 结论 | 依据 |
|---|---|---|
| 1. vibrancy 兼容性 | **GO** | tauri 2.11.5 + window-vibrancy 0.6 + 透明无边框窗口组合,`apply_vibrancy` 无报错;2026-07-17 用户确认毛玻璃效果 |
| 2. 托盘秒刷功耗 | **GO，保留秒级刷新** | debug 构建 30 秒采样 CPU 0.0–0.3%（均值 ~0.1%）< 0.5% 预算;方案:线程每秒 `run_on_main_thread` + `tray.set_title` |
| 3. rodio 音色 | **GO（有教训）** | 首版纯白噪音刺耳被否决;旧版实为**棕噪音**(offscreen.js:31 一阶低通),逐行对齐算法 + chime E5→B5 重叠双音后用户验收通过。教训:音频移植必须先读旧实现,"白噪音"这种名字不可信 |
| 4. positioner 定位 | **GO** | `Position::TrayBottomCenter`,tray 事件先喂 `on_tray_event`;用户实测弹出位置正确 |

### 其他记录

- 工具链安装路径:rustup 官方脚本 → `~/.cargo`;tauri-cli 用 cargo-binstall 装预编译二进制（13 秒 vs 源码编译 ~10 分钟）
- debug 构建 RSS ~110MB,略超 100MB 预算——**release 构建实测留到 phase 07**,预期显著下降;不作为本 phase 阻塞项
- 本机截屏无屏幕录制权限,托盘/面板/音频的视觉听觉验证全部归入用户验收步骤
