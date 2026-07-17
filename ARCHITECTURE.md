# Tomato Monster (macOS) · ARCHITECTURE

> 长期文档。配合 BRIEF.md(定位)、iterations/v1-launch/PRD.md(功能)、DESIGN.md(视觉)阅读。
> 旧 Chrome 插件代码位于本仓库同级目录 `../tomato-timer-v2`(本地私有仓库),是行为与 UI 的参考基准。

## 1. 运行时与语言

- **Tauri 2.x**,目标 macOS 13+。理由:前端资产最大化复用旧插件、产物 ~10MB、常驻内存小,符合"菜单栏常驻不碍眼"的定位。
- **后端 Rust**(Tauri 壳):计时状态机、持久化、音频、托盘、窗口管理、系统集成。
- **前端 vanilla JS(ES modules)+ CSS 变量,无构建工具**——延续旧版"无构建"哲学。`tauri.conf.json` 设 `app.withGlobalTauri: true`,前端直接用 `window.__TAURI__` 调 IPC,不引入 npm bundler。
- **不用 TypeScript**:无构建约定优先;复杂数据结构用 JSDoc 注释表达类型。
- **Rust 依赖基线**:tauri 2.x、serde / serde_json(持久化)、chrono(本地时区日切——配额 / 统计 / 连击都按本地日期结算,对齐旧版 `todayStr()` 行为)、rodio(音频)、window-vibrancy(毛玻璃)+ §6 的官方插件。新增依赖先在此登记再装。

## 2. 职责划分(核心架构决策)

**Rust 是唯一事实来源;前端是纯视图。**

- Rust 负责:状态机(`idle / focusing / breaking / paused`)、基于 `endTime` 时间戳的计时(不累加 tick,零漂移)、加时配额结算、连击 / 统计 / 徽章记账、每秒刷新托盘标题、触发系统通知与音频、持久化。
- 前端负责:渲染与交互。通过 Tauri event 订阅 `state-update`(状态快照广播),通过 command 发指令(`start / pause / resume / reset / abandon / overtime / update_settings / import_backup / export_backup / notion_export`)。
- **为什么不把计时器留在 JS**:WKWebView 对隐藏窗口的 JS 定时器有节流,把计时器放前端会重蹈 Chrome MV3 Service Worker 被休眠的覆辙。托盘标题更新、休息提醒都必须在面板关闭时照常工作。
- 旧 `background/service-worker.js`(1258 行)的状态机逻辑**按行为对齐移植**到 `timer.rs`,以旧代码为行为规范,包括防御性规则(break 阶段 skip 无效、配额日切重置、长休息每 N 个番茄触发等)。

## 3. 窗口体系

| 窗口 | 形态 | 实现要点 |
|---|---|---|
| 托盘 | 菜单栏图标 + `mm:ss` 标题 | Tauri `TrayIcon`,`iconAsTemplate: true`(单色模板剪影,三态);标题文字每秒由 Rust 更新,休息期间加红色提示符;右键原生菜单(统计与设置/退出),左键保持 toggle 面板 |
| 主面板 | 300px 无边框弹出,失焦即隐 | 点托盘图标显示 / 隐藏;`tauri-plugin-positioner` 定位到托盘图标下方;**隐藏而非销毁**(保留 DOM 状态,秒开);**休息期钉住**:自动弹出、失焦不隐、托盘点击不收起、不抢键盘焦点(2026-07-17 替代系统通知的决策,通知插件已移除) |
| 统计与设置 | 常规可调窗口(最小 560px) | 按需创建,关闭销毁 |
| 庆祝弹层 | 全屏无边框透明,点击即关 | love monster 7 天连击奖励;临时窗口,展示完销毁 |

**毛玻璃 vibrancy**:`window-vibrancy` crate(NSVisualEffectView)作用于主面板与统计窗口;窗口设 `transparent: true`,`tauri.conf.json` 开 `app.macOSPrivateApi: true`。CSS 层只画半透明表面色(对齐 DESIGN.md 的 surface-card token),不用 `backdrop-filter` 模拟。

## 4. 数据存储

- **位置**:`~/Library/Application Support/com.tomatomonster.app/store.json`(Tauri app data dir),Rust serde_json 读写。
- **原子写**:先写临时文件再 rename,防崩溃损坏。
- **Schema 顶层键沿用旧插件命名**:`timerState / quotaState / settings / stats / tasksToday / tasksArchive / badgesState / lastCategory / notionExportLog`。
- **旧备份导入**:旧插件备份 JSON(`BACKUP_SCHEMA_VERSION: 1`,含 `stats / tasksToday / tasksArchive / badgesState / settings / lastCategory / notionExportLog` 七键)逐键映射导入,先整体校验后写入,**失败整体回滚**,不半截导入。settings 里浏览器特有项(如悬浮窗设置)导入时丢弃。
- 导出备份保持同一 schema(版本号 +1,并保留对 v1 的读取兼容),路径由用户在保存对话框里选。

## 5. 音频

**Rust 侧 rodio 播放**,彻底绕开 WKWebView 自动播放策略:

- 白噪音:噪声源 + 音量包络(淡入淡出),等价移植旧 `offscreen/offscreen.js` 的 Web Audio 逻辑;休息开始后延迟 1 秒起播(与 chime 错开,旧版行为)。
- 提示音:chime 双音正弦合成,状态转折点播放;最后一分钟提示音同理。
- 音量 / 开关由 settings 控制,Rust 侧读取。

## 6. 系统集成(全部用 Tauri 官方插件)

| 能力 | 插件 | 说明 |
|---|---|---|
| 文件对话框 | `tauri-plugin-dialog` | 备份导出的保存路径选择(§4"保存对话框"的实现载体) |
| 全局快捷键 | `tauri-plugin-global-shortcut` | 默认 `⌃⌥P` 开始 / 暂停;设置里可改可关;注册失败(冲突)时降级提示 |
| 开机自启 | `tauri-plugin-autostart` | 设置里开关,默认关(首启引导询问) |
| Notion API | `tauri-plugin-http` | scope 仅 `https://api.notion.com/*`;前端现有 fetch 导出代码近乎原样移植(换成 plugin-http 的 fetch),无 CORS |

## 7. 代码组织

```
tomato-monster-mac/
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs            # 入口、插件注册、command 注册
│   │   ├── timer.rs          # 状态机(移植自 service-worker.js)
│   │   ├── store.rs          # JSON 持久化 + 备份导入导出
│   │   ├── audio.rs          # rodio 白噪音 / chime
│   │   ├── tray.rs           # 托盘图标三态 + 标题刷新
│   │   └── windows.rs        # 面板 / 统计 / 庆祝窗口管理
│   ├── icons/                # app 图标 + 托盘模板图标(idle/focus/break)
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                      # 前端,无构建,直接作为 frontendDist
│   ├── panel/                # panel.html / panel.css / panel.js
│   ├── stats/                # stats.html / stats.css / stats.js(热力图、历史、设置、Notion)
│   ├── celebration/          # celebration.html / celebration.js
│   └── shared/
│       ├── theme.css         # DESIGN.md token 的 CSS 变量(深浅两套)
│       ├── monsters/*.svg    # 从旧仓库 themes/monster/ 复制
│       ├── backgrounds/*.svg # 从旧仓库复制(庆祝弹层用)
│       └── util.js           # 共享工具(日期、格式化)
├── BRIEF.md / DESIGN.md / ARCHITECTURE.md / AGENTS.md / CLAUDE.md
└── iterations/
    └── v1-launch/            # PRD.md / CONTENT.md / .plan/
```

## 8. 开发约定

- **TEST_MODE 升级**:旧版要同步改两个文件(SW + lockscreen),新版只有 Rust 一处开关(环境变量 `TOMATO_TEST=1` 或 `timer.rs` 常量):专注 15 秒 / 休息 30 秒,时长配置随 `state-update` 事件下发前端——单一开关,不存在不同步。发布前清单只有一条:确认关掉。
- **行为对齐验收**:每移植一块逻辑,以旧插件的实际行为(而非猜测)为准;有疑问回旧代码查。
- 素材直接复制,不重绘:`themes/monster/*.svg`、`backgrounds/*.svg`。

## 9. 性能 / a11y / 安全预算(可验证)

**性能**(测量工具:活动监视器 + 手掐秒表):
- 常驻内存 < 100MB;托盘每秒刷新 CPU 均值 < 0.5%
- 面板点开到可见 < 150ms(隐藏式窗口保证)
- 计时漂移 0(endTime 制);挂机 8 小时后倒计时依然准确
- 安装包 < 15MB

**a11y**(对齐 DESIGN.md):AA 对比度、面板全键盘可达、图标按钮 VoiceOver 标签。

**安全**:
- Notion token 只存本地 `store.json`(文件权限 600),不入 git、不上传任何服务器(BRIEF 本地优先)
- Tauri CSP 保持默认严格配置;IPC command 显式白名单;`plugin-http` scope 只放 Notion 域
- `.gitignore`:`src-tauri/target/`、`*.dmg`、本地数据文件

## 10. 分发

`tauri build` 产出 .app / .dmg。自用不签名不公证;首次打开右键 → 打开绕过 Gatekeeper(README 写清楚)。无 CI/CD、无自动更新、不上架——对齐 BRIEF"不为分发投入"。

## 11. 已知风险与先行验证项

Phase 1 必须先做技术 spike 验证,再进入功能开发:

1. **window-vibrancy 与 Tauri 2.x 当前版本的兼容性**(透明窗口 + vibrancy + 无边框的组合)
2. **托盘标题每秒刷新**的功耗实测;超预算降为 30 秒粒度(面板内仍秒级)
3. **rodio 白噪音**音色与旧版 Web Audio 有差异,可接受;不行则改预渲染音频样本文件
4. `tauri-plugin-positioner` 的托盘相对定位在多显示器下的行为
