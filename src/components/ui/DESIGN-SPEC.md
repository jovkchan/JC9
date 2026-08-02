# JC9 UI 设计规范（对齐 Ant Design）

> 本文件是 `src/components/ui/` 的**设计规范来源与执行清单**。所有组件、Token、交互行为都应对齐本文件。
> 规范原文：Ant Design 设计规范（`ant.design/docs/spec/*`），每个页面都有机器可读的 `LLMs.md` 版本。

## 规范来源索引

| 类别 | 页面 | 状态 |
|---|---|---|
| 设计价值观 | [values-cn.md](https://ant.design/docs/spec/values-cn.md) | 原则层，已内化为本文件 |
| 色彩 | [colors-cn.md](https://ant.design/docs/spec/colors-cn.md) | ✅ 已落地 `tokens.scss`（色板/中性色/功能色） |
| 布局 | [layout-cn.md](https://ant.design/docs/spec/layout-cn.md) | ✅ 8px 网格 → `--jc-space-*` |
| 字体 | [font-cn.md](https://ant.design/docs/spec/font-cn.md) | ✅ 字阶/行高/字重/等宽数字 → `--jc-font-*` |
| 暗黑模式 | [dark-cn.md](https://ant.design/docs/spec/dark-cn.md) | ✅ `data-theme` + 暗色默认 |
| 阴影 | [shadow-cn.md](https://ant.design/docs/spec/shadow-cn.md) | ✅ 三层阴影 → `--jc-shadow-1/2/3` |
| 按钮 | [buttons-cn.md](https://ant.design/docs/spec/buttons-cn.md) | ✅ `JcButton`（type/danger/ghost/…） |
| 反馈 | [feedback-cn.md](https://ant.design/docs/spec/feedback-cn.md) | ✅ `JcToast`/`JcModal`/`JcBadge` |
| 数据录入 | [data-entry-cn.md](https://ant.design/docs/spec/data-entry-cn.md) | ✅ `JcInput/Textarea/Select/Segmented/Switch/Radio/Checkbox` |
| 数据展示 | [data-display-cn.md](https://ant.design/docs/spec/data-display-cn.md) | ✅ `JcTable`/`JcCard`/`JcTree` |
| 数据格式 | [data-format-cn.md](https://ant.design/docs/spec/data-format-cn.md) | 部分（`--` 空值、数值右对齐） |
| 文案 | [copywriting-cn.md](https://ant.design/docs/spec/copywriting-cn.md) | 文案书写规范（下附） |
| 四项原则 | [proximity/alignment/contrast/repetition-cn.md](https://ant.design/docs/spec/proximity-cn.md) | ✅ 亲密性/对齐/对比/重复 |
| 动效 | [motion-cn.md](https://ant.design/docs/spec/motion-cn.md) | ✅ `--jc-motion-*` |
| 过渡 | [transition-cn.md](https://ant.design/docs/spec/transition-cn.md) | 列表增删改高亮（待补） |
| 即时反应 | [reaction-cn.md](https://ant.design/docs/spec/reaction-cn.md) | ✅ 即时反馈原则 |
| 空状态 | [research-empty-cn.md](https://ant.design/docs/spec/research-empty-cn.md) | ✅ `JcEmpty` |
| 消息与反馈 | [research-message-and-feedback-cn.md](https://ant.design/docs/spec/research-message-and-feedback-cn.md) | ✅ `JcToast`/`JcModal` |
| 异常页 | [research-exception-cn.md](https://ant.design/docs/spec/research-exception-cn.md) | 待补 |
| 结果页 | [research-result-cn.md](https://ant.design/docs/spec/research-result-cn.md) | 待补 |

---

## 一、设计价值观（总纲）

1. **自然**：界面符合直觉、低认知成本；80% 信息靠视觉。
2. **确定性**：**保持克制**——能用最少元素表达就不多加；**模块化**——重复出现的局部封装成组件（这正是 ui/ 层存在的意义）。
3. **意义感**：明确目标、即时反馈。
4. **生长性**：组件/功能可发现、可扩展。

## 二、色彩（已落地 tokens.scss）

- 品牌主色取**色板第 6 档**：JC9 `--jc-color-accent: #8a58ff`
- 功能色：成功/错误/警告/信息，整套产品内**保持一致**
- 中性色：文本按层级（primary/secondary/tertiary/disabled），正文与背景保持 **WCAG AAA（≥7:1）** 对比
- 数字等宽：`font-variant-numeric: tabular-nums`（对齐、表格纵向对比）

## 三、布局（已落地）

- **8px 网格**：`--jc-space-xxs/xs/sm/base/lg/xl/xxl`（4/8/12/16/24/32/48）
- **纵向亲密性**：小 8px / 中 16px / 大 24px；间距公式 `y = 8 + 8n`
- 按钮区靠右、操作按主次排列

## 四、字体（已落地）

- 主字体 13（antd 14），字阶 xs~4xl；字重只用 400/500/600
- 数字/代码用等宽；`--jc-font-family` 系统字体优先

## 五、暗黑模式（已落地）

- `data-theme="dark|light"` 唯一数据源，暗色默认
- 原则：避免高对比刺激、保持与浅色模式**信息层级一致**
- 支持 `prefers-color-scheme` 自动跟随

## 六、阴影（已落地）

- 三层表达 `--jc-shadow-1/2/3`（离地越远颜色越淡、模糊越大）
- 菜单/下拉 = 2 层；弹窗 = 3 层

## 七、按钮（JcButton 已对齐）

- 一个按钮区**最多一个主按钮**；不确定用次按钮（最安全）
- 危险按钮用红色警示；**系统不推荐**该操作时，可把「取消」设为主按钮
- 文案**必须用动词**且简练（发布/登录/删除）；默认「确定/取消」
- 顺序：优先询问→最后风险操作；返回类放左侧
- 分组用**间距**区隔，不用短竖线
- 纯图标按钮必须配 Tooltip

## 八、反馈（JcToast/JcModal/JcBadge 已对齐）

- **避免过度反馈**：能即时看到效果的简单操作省略提示
- 加载 >2s 给进度提示；长时间加载提供取消
- **重要失败**用 `JcModal` 对话框，不用轻量 Toast（Toast 默认 3s 会错过）
- 录入错误说明紧跟区块、不自动消失
- Badge：重要高关联用数字，低权重用红点

## 九、数据录入（已对齐）

- label 默认放输入框左侧，系统内统一
- 用 placeholder 暗提示帮助输入；短提示放输入框下方
- 下拉选项 >5 项时才用；选项按逻辑排序

## 十、数据展示（已实现）

- 表格：时间/状态/操作栏词语完整不过行；空数据用 `-`
- 卡片：一行 ≤4 个；信息过长截断

## 十一、数据格式（部分落地）

- 数值：千分位、单位小写、表格右对齐
- 日期 `yyyy-mm-dd`；时间 24h `HH:mm:ss`；相对时间「刚刚/N 分钟前/N 小时前/mm-dd HH:mm」
- 无数据 `--`；加载用骨架屏
- **数字用阿拉伯数字**

## 十二、文案（书写规范，写文案前必读）

- 以用户为主体（「你可以…」而非「我们为你…」）
- 用「你/我」，避免「您」；同一句式不混用
- 报错说「无法完成」而非冷冰冰「失败」；给出下一步
- 精简：省略用户已知事实
- 标签/标题/输入框下提示**省略句号**
- 全角半角搭配加空格（如「2 个」）；连接号用半角 `-`；省略号用半角 `…`
- 避免绝对化表述（「绝不」）

## 十三、四项原则

- **亲密性**：关联越高间距越近；用 8/16/24 分级
- **对齐**：文案统一视觉起点；表单标签冒号右对齐；数值右对齐+相同有效位数
- **对比**：主次分明；需慎重决策场景保持中立（不诱导用户）
- **重复**：相同元素复用，降低学习成本

## 十四、动效（已落地 --jc-motion-*）

- 三原则：**自然 / 高效 / 克制**
- 进场可稍慢、出场要快；过渡时长 0.1~0.3s
- 增/删/改对象给「高亮几秒后消失」反馈（列表场景，待补）

## 十五、空状态（JcEmpty 已对齐）

- 三要素：**明确原因 + 提供邀请（建议操作）**
- 场景：新手引导 / 完成清空 / 无数据

---

## 待办（按价值排序）

- [ ] 收编 32 个工具页 → `ToolShell` + `Jc*`（已样板：Base64Tool）
- [x] 补选择/反馈/展示组件：`JcDropdown` `JcTooltip` `JcSwitch` `JcRadio`(+Group) `JcCheckbox`(+Group) `JcSkeleton` `JcCard` `JcTable` `JcTree`
- [ ] 列表增删改高亮动效（transition 规范）
- [ ] 异常页 / 结果页模板组件
- [ ] 独立 npm 包 `@jc9/ui`（调试满意后）
