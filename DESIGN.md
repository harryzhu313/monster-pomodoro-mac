---
name: Tomato Monster (macOS)
description: 温暖手绘的番茄红小怪兽,住进克制的 macOS 毛玻璃面板——紧凑、留白、系统原生质感。

colors:
  primary: "#d94f3a"
  primary-strong: "#c24330"
  primary-soft: "#e77b68"
  on-primary: "#ffffff"
  success: "#4a8a5c"
  warning: "#b08a3e"
  error: "#b0524a"
  neutral: "#f6f5f0"
  on-neutral: "#2a2a2a"
  on-neutral-muted: "#8a8a85"
  surface-card: "rgba(255,255,255,0.55)"
  border-hairline: "rgba(0,0,0,0.08)"
  neutral-dark: "#1f1e1c"
  on-neutral-dark: "#e8e6e1"
  on-neutral-dark-muted: "#9a9892"
  surface-card-dark: "rgba(255,255,255,0.07)"
  border-hairline-dark: "rgba(255,255,255,0.10)"

typography:
  display:
    fontFamily: "-apple-system, 'SF Pro Display', 'PingFang SC', sans-serif"
    fontSize: "48px"
    fontWeight: 300
    lineHeight: "1.1"
    letterSpacing: "0.02em"
  h1:
    fontFamily: "-apple-system, 'PingFang SC', sans-serif"
    fontSize: "16px"
    fontWeight: 600
    lineHeight: "1.3"
  h2:
    fontFamily: "-apple-system, 'PingFang SC', sans-serif"
    fontSize: "13px"
    fontWeight: 600
    lineHeight: "1.4"
  body:
    fontFamily: "-apple-system, 'PingFang SC', sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: "1.5"
  caption:
    fontFamily: "-apple-system, 'PingFang SC', sans-serif"
    fontSize: "11px"
    fontWeight: 400
    lineHeight: "1.4"

spacing:
  xs: 4px
  sm: 8px
  md: 12px
  lg: 16px
  xl: 24px
  2xl: 32px

rounded:
  none: 0
  sm: 6px
  md: 8px
  lg: 10px
  xl: 14px
  full: 9999px

elevation:
  none: "none"
  sm: "0 1px 2px rgba(0,0,0,0.06)"
  md: "0 4px 12px rgba(0,0,0,0.10)"
  lg: "0 12px 32px rgba(0,0,0,0.18)"

components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.on-primary}"
    rounded: "{rounded.md}"
    padding: "8px 16px"
    typography: "{typography.body}"
  button-primary-hover:
    backgroundColor: "{colors.primary-strong}"
  button-secondary:
    backgroundColor: "{colors.surface-card}"
    textColor: "{colors.on-neutral}"
    border: "1px solid {colors.border-hairline}"
    rounded: "{rounded.md}"
    padding: "8px 16px"
    typography: "{typography.body}"
  card:
    backgroundColor: "{colors.surface-card}"
    border: "1px solid {colors.border-hairline}"
    rounded: "{rounded.lg}"
    padding: "{spacing.md}"
  input-text:
    backgroundColor: "{colors.surface-card}"
    border: "1px solid {colors.border-hairline}"
    rounded: "{rounded.sm}"
    typography: "{typography.body}"
  badge:
    backgroundColor: "{colors.surface-card}"
    textColor: "{colors.on-neutral-muted}"
    rounded: "{rounded.full}"
    typography: "{typography.caption}"
---

## Overview

一只温暖的手绘小怪兽,住进克制的系统原生面板。

这是两种气质的融合:**内容层**延续旧 Chrome 插件版的"温暖纸感"——番茄红、蜡笔质感的情绪小怪兽、🍅 收获行;**容器层**是 macOS 原生的毛玻璃 vibrancy——半透明面板、发丝线边框、跟随系统深浅外观。原则:个性全部交给内容,容器保持沉默。

核心 vibe 关键词:温暖、克制、紧凑、原生。

## Colors

- **Primary (#d94f3a):** 番茄红,品牌主色,继承旧版。只做强调——主按钮、专注状态标签、当前任务标记,永不大面积铺色。
- **Primary-strong (#c24330):** hover 态;需要小字白底且对比度要求高的场景用它(5.1:1)。
- **Primary-soft (#e77b68):** 深色模式下的文字级番茄红强调(深底上直接用 #d94f3a 对比不足)。
- **Success (#4a8a5c) / Warning (#b08a3e) / Error (#b0524a):** 状态三色继承旧版——绿=休息,琥珀=暂停,红字系=专注/放弃。
- **Neutral 系:** 表面色全部半透明(surface-card),让窗口 vibrancy 透出来。浅色模式暖白基调(#f6f5f0 罩),深色模式暖黑(#1f1e1c 罩)。深浅两套 token 同构一一对应,切换只换值不换结构。

## Typography

全系统字体栈,零外部字体加载。倒计时数字用 48px 细体(300)并强制 `font-variant-numeric: tabular-nums`,保证每秒跳动时不左右抖动。界面字阶只有三档:11px(caption/元信息)、13px(正文/按钮)、16px(窗口区块标题),紧凑但不拥挤。中文回退 PingFang SC。

## Layout

- **主面板:** 固定宽 300px,单列卡片流,从上到下:计时卡 → 今日收获 → 今日任务 → 底栏。
- **统计与设置窗口:** 默认 680px,可调(最小 560px),卡片栅格排布,窄时单列。
- **间距:** 4/8/12/16/24/32 六档;面板内以 sm/md 为主,窗口区块间用 lg/xl。

## Elevation & Depth

**soft 档。** 面板自身的投影交给系统窗口,CSS 不重复画;面板内部靠发丝线边框 + 半透明填充分层,不用阴影;`elevation.sm` 仅用于 hover 浮起提示;唯一的 `elevation.lg` 留给庆祝弹层(love monster)。

## Shapes

圆角三档语言:输入框 6px / 按钮 8px / 卡片 10px,面板外框 14px(贴近 macOS 原生面板)。发丝线边框(1px 半透明)取代旧版实色边框。无装饰性形状、无分割线堆砌——小怪兽是界面里唯一的插画元素。

## Components

### button-primary
页面内唯一最高优先级动作(开始专注/添加任务)。番茄红底白字,hover 加深为 primary-strong。一个视图里最多出现一个。

### button-secondary
次级动作(重置/导出/测试)。半透明表面 + 发丝线边框,不抢主按钮的视觉权重。

### card
主面板与统计窗口的基础容器。半透明填充让 vibrancy 透光,发丝线描边。

### input-text
任务名输入、Notion 配置等。聚焦时边框变 primary。

### badge
任务分类标签(工作/学习/生活/兴趣爱好)、配额指示(剩 N/3)。胶囊形,caption 字号。

### 原生件(无 CSS token)
- **菜单栏图标:** 单色模板剪影(template icon),idle/专注/休息三态用不同剪影;旁挂 mm:ss 倒计时文字,休息期间文字变红醒目。
- **系统通知:** 原生样式,不定制。

## Do's and Don'ts

### Do's
- ✅ 只用系统字体;倒计时用 tabular-nums
- ✅ 深浅双模式同步设计,所有颜色经 CSS 变量 token,禁止硬编码 hex 散落
- ✅ 小怪兽是唯一插画元素,保留蜡笔毛绒质感(SVG turbulence + displacement filter)
- ✅ 🍅/🥫/🔥 emoji 仅用于收获数据行(功能性数据可视化,继承旧版)
- ✅ 菜单栏图标用单色模板剪影,自动适配深浅菜单栏

### Don'ts
- ❌ 不要渐变大色块
- ❌ 不要加载外部字体
- ❌ 不要在毛玻璃上叠加重阴影(拟物堆叠)
- ❌ 不要彩色菜单栏图标
- ❌ 不要用 emoji 当功能图标(按钮、导航、设置项)
- ❌ 不要超过 200ms 的阻塞式动画

---

## Motion & Animation

**Level:** 微。

**Typical scenes:**
- 小怪兽表情切换:150ms 淡入淡出
- 按钮 hover:100ms 颜色过渡
- 最后一分钟:倒计时数字轻微颜色脉动(提示不打扰)
- love monster 庆祝入场:400ms 弹性缩放——全应用唯一的"丰富"动效时刻
- 全部动效尊重 `prefers-reduced-motion`,减弱为直接切换

## Responsiveness

非响应式网页场景:主面板固定 300px 不缩放;统计与设置窗口可调宽(最小 560px),内部卡片栅格自动换行;无移动端、无断点体系。

## Accessibility

**WCAG target:** AA

- **对比度:** 白字 on #d94f3a = 4.1:1(临界,决定保留旧版配色;按钮文字保持 13px semibold,如后续 lint 报警整体切换 primary-strong 底 5.1:1)。深浅两套 token 分别校验,深色模式文字级番茄红一律用 primary-soft。
- **键盘:** 面板内全部按钮/输入可 Tab 聚焦,聚焦环不隐藏。
- **VoiceOver:** 图标按钮带 aria-label;倒计时状态不依赖颜色单独传达(始终有文字标签)。

## UI Framework Considerations

**Requirements:**
- 无重型框架,延续旧版 vanilla HTML/CSS/JS 的简单性
- CSS 变量主题体系,支撑深浅模式切换(`prefers-color-scheme` + 手动覆盖)
- SVG 直嵌(小怪兽 filter 需要 inline SVG)
- 毛玻璃由窗口层提供(Tauri 侧 vibrancy),CSS 只负责半透明表面,不用 backdrop-filter 模拟

**Candidate libraries:**
- **Vanilla JS + CSS variables**(推荐)—— 最大化复用旧代码,无构建工具的简单性延续
- **Svelte** —— 仅当状态管理复杂化时考虑

(最终选型在 ARCHITECTURE.md 定论)

## References & Inspiration

**Reference sites / works:**
- Anna Llenas *The Color Monster* —— 情绪小怪兽的隐喻来源,致敬不复刻(版权)
- macOS 原生菜单栏面板(日历、电池、控制中心)—— 毛玻璃质感、发丝线、紧凑布局的基准

**User-provided inspiration:**
- 旧插件 `/Users/apple/web-projects/tomato-timer-v2` 的 popup.css / options.css —— 布局、层级与状态色的事实参考
