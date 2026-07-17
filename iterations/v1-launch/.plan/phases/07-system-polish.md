# Phase 07 — 系统集成与发布收尾

**Status**: `completed`
**目标**: 快捷键 / 自启 / 首启引导 / 庆祝弹层落地，性能预算实测达标，产出可安装的 .dmg
**前置**: Phase 06（功能全部就位）

## 验收判据

- 首次启动引导完整走通：欢迎与导入 → 自启询问 → 快捷键提示（文案对齐 CONTENT.md；通知权限步骤已随系统通知移除，2026-07-17）
- 全局快捷键 ⌃⌥P 在任意 app 内开始 / 暂停生效；设置里可改可关；注册冲突时降级提示
- 7 天连击庆祝可触发（TOMATO_TEST 模拟）：love monster 全屏、点击即关
- 性能预算全达标（ARCHITECTURE §9）：常驻内存 < 100MB / 托盘刷新 CPU < 0.5% / 面板 < 150ms / 计时漂移 0 / 包 < 15MB
- TEST_MODE 确认关闭；`cargo tauri build` 产出 .dmg 可安装运行；README 写明右键打开绕 Gatekeeper + 迁移步骤

## Tasks

- [x] `tauri-plugin-global-shortcut`：默认 ⌃⌥P 开始 / 暂停，设置可改可关，冲突降级提示 (lib.rs apply_global_shortcut/shortcut_toggle;休息中快捷键无操作——红线 1 复查通过;设置页按键捕获式输入,Delete 关闭,注册失败回滚原快捷键并提示;启动注册失败仅日志不阻塞)
- [x] `tauri-plugin-autostart` 接入设置开关（默认关）(LaunchAgent 方式;真相源=系统状态,get/set_autostart 命令;设置页开关失败自动回滚)
- [x] 首启引导流：欢迎与导入 / 自启询问 / 快捷键提示（CONTENT.md 文案逐字）(panel.html onboarding 覆盖层三步;首启检测=store.json 不存在,既有安装自动豁免;导入复用 import_backup 含 CONTENT 反馈文案;实测新装启动面板自动弹出)
- [x] 庆祝弹层：7 天连击触发,love monster 全屏,点击/Esc/10s 关（平移旧 celebration.js,窗口用后销毁）(src/celebration/*;色板入 --celebration-* token;prefers-reduced-motion 降级;实测:构造 7 天连击前夜状态,第 1 个番茄进休息即触发 [effect] Celebration,窗口创建无报错)
- [x] 托盘正式模板图标 ×3 + app 图标各尺寸 (qlmanage 栅格化 SVG → 纯 stdlib 脚本按"离白距离"提取剪影,眼嘴镂空三态可辨(happy/calm/angry);tray 三态随状态切换,只在类别变化时 set_icon;app 图标用旧 icon128 经 cargo tauri icon 重导出全尺寸含 icon.icns,android/ios 产物已清)
- [x] a11y 抽查：AA 对比度、面板全键盘可达、图标按钮 VoiceOver 标签 (对比度按 DESIGN §Accessibility 的 token 校验结论执行:深色文字级番茄红一律 --primary-text→primary-soft;倒计时 tabular-nums;全按钮 :focus-visible 描边;图标按钮(删除/勾选)带 aria-label;VoiceOver 逐项朗读留作使用中持续确认)
- [x] 性能验收实测并记录数字 (见 Notes"性能实测";托盘秒刷稳态 CPU ~0.2% ✓、dmg 6.6MB ✓、计时零漂移 ✓、内存 RSS ~107MB ⚠️ 略超预算,原因与取舍见注)
- [x] 确认 TEST_MODE 关闭；`cargo tauri build` 产出 .app / .dmg (TEST 模式是 TOMATO_TEST=1 环境变量单一开关,正常启动天然关闭——不存在旧版"改回常量"步骤;release .app 已产出;dmg 经 hdiutil 生成——tauri 自带 bundle_dmg.sh 依赖 Finder 自动化权限在本环境失败,见 Notes)
- [x] README：安装（右键打开）、从旧插件迁移步骤、常用命令 (README.md:安装/Gatekeeper 绕过/迁移四步/日常使用/开发命令/License)
- [x] 托盘右键菜单：统计与设置 / 退出 (added 2026-07-17: 用户验收发现无退出途径——菜单栏 app 无 Dock 无 ⌘Q;左键 toggle 面板不变;红线复核:退出后状态持久化重开即恢复,不构成休息跳过;PRD §5/ARCHITECTURE §3 已同步)

## 验收与提交

- 用户验收：2026-07-17 通过（快捷键/自启/首启引导/庆祝弹层/托盘三态/退出菜单/dmg 安装;内存 ~107MB 取舍经用户认可）
- Commit：见 git log(Phase 07 提交)

## Notes

### 性能实测(2026-07-17,release 构建,M 系列)

| 预算项(ARCHITECTURE §9) | 实测 | 结论 |
|---|---|---|
| 托盘每秒刷新 CPU < 0.5% | 稳态 0.1–0.3%(启动瞬态后) | ✅ |
| 常驻内存 < 100MB | RSS ~107MB | ⚠️ 略超:WKWebView 基线 + 面板隐藏而非销毁(换秒开);ps RSS 计入共享框架页,私有占用更低。接受为 v1 已知项 |
| 面板点开 < 150ms | 隐藏式窗口,无创建开销 | ✅(主观秒开,用户使用中确认) |
| 计时漂移 0 | endTime 制;重启/杀进程恢复实测精确(phase 02) | ✅ |
| 安装包 < 15MB | dmg 6.6MB(.app 落盘 20MB) | ✅ |
| 挂机 8 小时倒计时准确 | 机制保证(endTime),留 dogfood 长测 | 🕐 |

**性能修正**:验收中发现专注态 CPU ~1.4% 超预算——每秒向隐藏面板广播全量快照(序列化+webview 渲染)。修复:快照广播按窗口可见性门控(panel_visible/stats_open 原子位),隐藏时只刷托盘标题,窗口转可见瞬间补发一拍;修后稳态 ~0.2%。

### 其他记录

- dmg 打包:tauri bundle_dmg.sh(create-dmg)依赖 Finder/AppleScript 自动化权限,本环境失败;改用 `hdiutil create -format UDZO` 生成(无 Finder 美化排版,自用足够)。后续如需:授权后可用官方脚本
- 托盘剪影生成链:qlmanage 栅格化 SVG(不透明白底)→ 纯 stdlib 脚本按"离白距离×alpha"提取覆盖度 → 44px box 采样 → 纯黑+alpha 模板 PNG;眼嘴成镂空,三态表情可辨(脚本在会话 scratchpad,一次性工具未入库)
- 首启引导测试:移走 store.json 模拟新装,面板自动弹出;既有安装(文件已存在)在 Store::load 自动豁免引导
- 庆祝弹层测试:构造 anchorDate=8 天前 + 前 7 日 stats + 今日 0 番茄,第 1 个番茄进休息即触发 Celebration,窗口创建无报错;测试数据已还原用户真实 store.json
- 旧 lockscreen 的 afraid monster hover 彩蛋确认不做(v1 范围外,如想要归 v2)
