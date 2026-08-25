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

**Rust 是本地业务状态的唯一事实来源;前端是视图与受限的 Notion 集成适配层。**

- Rust 负责:状态机(`idle / focusing / breaking / paused`)、基于 `endTime` 时间戳的计时(不累加 tick,零漂移)、加时配额结算、连击 / 统计 / 徽章记账、每秒刷新托盘标题、触发系统通知与音频、持久化。
- 前端负责:渲染与交互。通过 Tauri event 订阅 `state-update`(状态快照广播),通过 command 发送用户意图(计时、任务、设置、备份、Notion 导出与月度目标)。Notion 同步只消费并冻结 Rust 快照,不自行维护或推导业务状态。
- **v2 月度目标写操作**:`set_task_planned(id, planned)` 与 `add_goal / update_goal / delete_goal / toggle_goal_today`。它们全部走 `with_store`,由 Rust 统一完成夹紧/规则校验、持久化、`state-update` 广播与托盘刷新。`toggle_goal_today` 返回 `{ ok, reason, checked }`;已投入番茄时以 `locked` 拒绝取消,未知目标返回 `not-found`。
- **v3 月度目标 Notion 同步**:统计窗口在点击时先等待已经触发的 Rust 目标保存,随后只取一次新的 `get_state` 快照并冻结 Notion 配置。`notion.js` 以该快照做只读 schema / 月份页预检和幂等 upsert;前端不得扫描任务、读取浏览器时间或在请求过程中混用后续广播。远端身份固定为 `所属月 + 本地目标 ID`;本地删除不触发远端 delete / archive。
- **为什么不把计时器留在 JS**:WKWebView 对隐藏窗口的 JS 定时器有节流,把计时器放前端会重蹈 Chrome MV3 Service Worker 被休眠的覆辙。托盘标题更新、休息提醒都必须在面板关闭时照常工作。
- 旧 `background/service-worker.js`(1258 行)的状态机逻辑**按行为对齐移植**到 `timer.rs`,以旧代码为行为规范,包括防御性规则(break 阶段 skip 无效、配额日切重置、长休息每 N 个番茄触发等)。

## 3. 窗口体系

| 窗口 | 形态 | 实现要点 |
|---|---|---|
| 托盘 | 菜单栏图标 + `mm:ss` 标题 | Tauri `TrayIcon`,`iconAsTemplate: true`(单色模板剪影,三态);标题文字每秒由 Rust 更新,休息期间加红色提示符;右键原生菜单(统计与设置/退出),左键保持 toggle 面板 |
| 主面板 | 固定 360×700px 无边框弹出,失焦即隐 | 点托盘图标显示 / 隐藏;`tauri-plugin-positioner` 定位到托盘图标下方;**隐藏而非销毁**(保留 DOM 状态,秒开);内容超高时沿用纵向滚动,运行时不动态改窗口高度;**休息期钉住**:自动弹出、失焦不隐、托盘点击不收起、不抢键盘焦点(2026-07-17 替代系统通知的决策,通知插件已移除) |
| 统计与设置 | 常规可调窗口(最小 560px) | 按需创建,关闭销毁 |
| 庆祝弹层 | 全屏无边框透明,点击即关 | love monster 7 天连击奖励;临时窗口,展示完销毁 |

**毛玻璃 vibrancy**:`window-vibrancy` crate(NSVisualEffectView)作用于主面板与统计窗口;窗口设 `transparent: true`,`tauri.conf.json` 开 `app.macOSPrivateApi: true`。CSS 层只画半透明表面色(对齐 DESIGN.md 的 surface-card token),不用 `backdrop-filter` 模拟。

## 4. 数据存储

- **位置**:`~/Library/Application Support/com.tomatomonster.app/store.json`(Tauri app data dir),Rust serde_json 读写。
- **原子写**:临时文件创建时即设 0600,写入并 `sync_all` 后再 rename,既防崩溃损坏也避免敏感内容短暂落入宽权限文件。
- **Schema 顶层键**:旧插件键名保持不变,新能力只追加字段。当前固定键为 `timerState / quotaState / settings / stats / tasksToday / tasksArchive / badgesState / monthlyGoals / lastCategory / notionExportLog / notionConfig / onboardingDone`。未识别的顶层键、任务字段与月度目标字段继续透传,保持前向兼容。
- **月度目标持久化**:`monthlyGoals` 是长期有效、不按月分桶的数组,每项定义字段为 `{ id, title, category, monthTarget }`;跨月保留条目,仅进度按自然月重新派生。`Task.goalId?: string` 只标记从目标勾选产生的任务,手动任务与旧任务不写该键。删除目标不级联删除已有任务,`goalId` 允许成为悬空引用。后加字段使用 serde 默认值:旧 store 缺 `monthlyGoals` 时按空数组加载,旧任务缺 `goalId` 时按 `None` 加载且回写不新增该键。
- **月度目标视图契约**:`get_state` / `state-update` 的 `Snapshot` 和统计窗口的 `StatsBundle` 都返回 `monthlyGoals: GoalView[]`;每项包含 `id / title / category / monthTarget / monthDone / checkedToday / locked`。后三者不持久化:`monthDone` 是当前自然月 `tasksToday + tasksArchive` 中相同 `goalId` 任务的 `used` 之和,`checkedToday / locked` 从今日关联任务派生;前端不自行维护或推导。
- **Notion 配置与摘要**:`notionConfig` 追加 `goalDbId`,仅持久化于权限 600 的本机 store 且不进备份。配置更新使用 merge-patch:省略字段保持原值,显式空串清空该字段;落盘失败时回滚内存配置并向调用方返回错误。月度同步摘要复用 `notionExportLog["monthly:YYYY-MM"]`,只保存 `at / created / updated / failed`;每日导出当下的详细反馈不变,持久化摘要只保存 `at / ok / created / updated / failed / total / existingDuplicates / dayPageLinked`。Rust 在 IPC、旧 store 加载、备份导入与导出边界统一执行白名单;逐项目错误、Token、数据库 / 页面 ID、标题和请求内容不持久化。
- **备份 schema**:当前导出 `schemaVersion: 3`,导入受理 `{1, 2, 3}`。v3 `data` 包含 `stats / tasksToday / tasksArchive / badgesState / monthlyGoals / settings / lastCategory / notionExportLog`;`goalId` 随任务对象自然进入备份。v1 的 `lastTaskCategory` 与 v2/v3 的 `lastCategory` 均可读;v1/v2 缺少 `monthlyGoals` 时按空数组恢复。`timerState / quotaState / notionConfig / onboardingDone` 不进备份,确保 token 不外泄。导入先在副本完成全部解析,全部成功后才一次替换;settings 中浏览器特有项丢弃。导出路径由用户在保存对话框里选择;macOS 新建或覆盖的备份文件收窄为仅当前用户可读写(0600),权限设置失败时不截断同名旧文件。

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
| Notion API | `tauri-plugin-http` | scope 仅 `https://api.notion.com/*`,固定 `Notion-Version: 2022-06-28`;`notion.js` 同时承载既有每日导出与 v3 月度目标预检 / upsert,无 CORS,不引入新服务端 |

## 7. 代码组织

```
tomato-monster-mac/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs           # 桌面二进制入口
│   │   ├── lib.rs            # Tauri 初始化、托盘/窗口、插件与 command 注册
│   │   ├── timer.rs          # 状态机(移植自 service-worker.js)
│   │   ├── store.rs          # JSON 持久化 + 备份导入导出
│   │   ├── audio.rs          # rodio 白噪音 / chime
│   ├── icons/                # app 图标 + 托盘模板图标(idle/focus/break)
│   ├── Cargo.toml
│   ├── tauri.conf.json       # 正式版身份与打包配置
│   └── tauri.dev.conf.json   # 开发版名称 / identifier 覆盖层
├── src/                      # 前端,无构建,直接作为 frontendDist
│   ├── panel/                # panel.html / panel.css / panel.js
│   ├── stats/                # stats UI + notion.js(日任务/月目标同步)+ goal-sync-ui.js + 两组 .test.mjs
│   ├── celebration/          # celebration.html / celebration.js
│   └── shared/
│       ├── theme.css         # DESIGN.md token 的 CSS 变量(深浅两套)
│       ├── monsters/*.svg    # 从旧仓库 themes/monster/ 复制
│       ├── backgrounds/*.svg # 从旧仓库复制(庆祝弹层用)
│       └── util.js           # 共享工具(日期、格式化)
├── BRIEF.md / DESIGN.md / ARCHITECTURE.md / AGENTS.md / CLAUDE.md
└── iterations/
    ├── v1-launch/            # 首版 PRD / CONTENT / plan
    ├── v2-monthly-goals/     # 本地月度目标 PRD / plan
    └── v3-notion-goals-sync/ # Notion 月度同步 PRD / plan
```

## 8. 开发约定

- **TEST_MODE 升级**:旧版要同步改两个文件(SW + lockscreen),新版唯一时长开关是运行时环境变量 `TOMATO_TEST=1`:专注 15 秒 / 休息 30 秒,时长配置随 `state-update` 事件下发前端——单一开关,不存在不同步。测试模式使用系统临时目录中的进程级隔离 store 并跳过首次引导,不会给真实统计增加测试番茄。另有 `TOMATO_AUTOSTART=1`(启动专注)和 `TOMATO_OPEN_STATS=1`(打开统计窗口)两个开发钩子;发布构建必须同时清除三者。
- **开发 / 正式身份隔离**:开发命令必须合并 `tauri.dev.conf.json`,使用 `Tomato Monster Dev` / `com.tomatomonster.dev` 和独立 app data 目录；debug build 若仍加载正式 identifier,会在插件初始化和 store 路径解析前拒绝启动。正式版唯一入口为 `/Applications/Tomato Monster.app`，真实数据与 Notion 验收不在开发版执行。
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
- Notion token 仅持久化于本机 `store.json`(文件权限 600),请求时只直连 `api.notion.com`,不经过自建或第三方中间服务
- Notion 日志在 Rust IPC / 加载 / 备份边界按每日或月度安全统计白名单收窄;`notionConfig` 不进入备份,备份内的 `notionExportLog` 不携带逐项目错误、标题或请求内容(任务与目标本体仍按备份 schema 正常包含标题)
- Tauri CSP 显式限制为本地资源(`default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:`);IPC command 显式白名单;`plugin-http` scope 只放 Notion 域
- `.gitignore`:`src-tauri/target/`、`*.dmg`、本地数据文件

## 10. 分发

`tauri.conf.json` 明确 `bundle.macOS.minimumSystemVersion: 13.0`;当前 Apple Silicon 自用包使用 `env -u TOMATO_TEST -u TOMATO_AUTOSTART -u TOMATO_OPEN_STATS cargo tauri build --target aarch64-apple-darwin --no-sign` 产出 .app / .dmg，最终包固定安装到 `/Applications/Tomato Monster.app` 并只从此入口启动。自用不签名不公证;首次打开右键 → 打开绕过 Gatekeeper(README 写清楚)。既有 bundle identifier 为保持 app data 路径兼容暂不迁移,虽会触发 Tauri 的 `.app` 后缀建议警告,不影响本次构建。无 CI/CD、无自动更新、不上架——对齐 BRIEF"不为分发投入"。

## 11. 已知风险与先行验证项

下列技术 spike 已在首版完成验证,后续版本持续观察:

1. **window-vibrancy 与 Tauri 2.x 当前版本的兼容性**(透明窗口 + vibrancy + 无边框的组合)
2. **托盘标题每秒刷新**的功耗实测;超预算降为 30 秒粒度(面板内仍秒级)
3. **rodio 白噪音**音色与旧版 Web Audio 有差异,可接受;不行则改预渲染音频样本文件
4. `tauri-plugin-positioner` 的托盘相对定位在多显示器下的行为
