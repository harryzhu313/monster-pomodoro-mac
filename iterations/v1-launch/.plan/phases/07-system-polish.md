# Phase 07 — 系统集成与发布收尾

**Status**: `not started`
**目标**: 快捷键 / 自启 / 首启引导 / 庆祝弹层落地，性能预算实测达标，产出可安装的 .dmg
**前置**: Phase 06（功能全部就位）

## 验收判据

- 首次启动引导完整走通：欢迎与导入 → 通知权限 → 自启询问 → 快捷键提示（文案对齐 CONTENT.md）
- 全局快捷键 ⌃⌥P 在任意 app 内开始 / 暂停生效；设置里可改可关；注册冲突时降级提示
- 7 天连击庆祝可触发（TOMATO_TEST 模拟）：love monster 全屏、点击即关
- 性能预算全达标（ARCHITECTURE §9）：常驻内存 < 100MB / 托盘刷新 CPU < 0.5% / 面板 < 150ms / 计时漂移 0 / 包 < 15MB
- TEST_MODE 确认关闭；`cargo tauri build` 产出 .dmg 可安装运行；README 写明右键打开绕 Gatekeeper + 迁移步骤

## Tasks

- [ ] `tauri-plugin-global-shortcut`：默认 ⌃⌥P 开始 / 暂停，设置可改可关，冲突降级提示
- [ ] `tauri-plugin-autostart` 接入设置开关（默认关）
- [ ] 首启引导流：欢迎与导入 / 通知权限 / 自启询问 / 快捷键提示（CONTENT.md"首次启动引导"节）
- [ ] 庆祝弹层：7 天连击第一个番茄结束进入休息时触发，love monster 全屏透明窗口，点击即关（平移旧 celebration.js，窗口用后销毁）
- [ ] 托盘正式模板图标 ×3（从 happy / calm / angry SVG 提取单色剪影，纯黑 + alpha）+ app 图标各尺寸
- [ ] a11y 抽查：AA 对比度、面板全键盘可达、图标按钮 VoiceOver 标签
- [ ] 性能验收实测并记录数字（活动监视器 + 手掐秒表）
- [ ] 确认 TEST_MODE 关闭；`cargo tauri build` 产出 .app / .dmg
- [ ] README：安装（右键打开）、从旧插件迁移步骤、常用命令

## Notes

（执行中的临时记录写这里）
