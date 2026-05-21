# Skills 合并 & Bug修复任务计划

> 当前分支: `custom-for-cn-user`
> 目标: 从 upstream 分支中合并 Skills 能力、Agent 改进、Bug 修复

---

## 任务总览

| 阶段 | 任务 | 状态 |
|------|------|------|
| 1 | Merge upstream/main → 获取 Skills 基础 + 各类修复 | ✅ Done |
| 2 | Cherry-pick `agent-panel-menu-items` → Skills/Rules 菜单分组 | 🔄 In Progress |
| 3 | Cherry-pick `create-skill-prevent-dupe` → 防重复 create-skill | ⬜ Pending |
| 4 | Cherry-pick `martin/ai-234-lag` → Agent outline 渲染性能修复 | ⬜ Pending |
| 5 | Cherry-pick `link-backticked-file-paths` → Agent 面板文件路径可点击 | ⬜ Pending |
| 6 | Cherry-pick `martin/ai-233` → 空路径代码块渲染为文件引用 | ⬜ Pending |
| 7 | Cherry-pick `kb/multi-buffer-auto-reveal` → 多缓冲导航修复 | ⬜ Pending |
| 8 | Cherry-pick `fix-issue-57039` → dev_container 路径修复 | ⬜ Pending |
| 9 | Cherry-pick `fix-issue-56254` → Vim space-leader 延迟修复 | ⬜ Pending |
| 10 | Cherry-pick `fix-gpui-windows-reentrancy` → Windows GPUI 重入修复 | ⬜ Pending |
| 11 | Cherry-pick `fix-issue-55939` → git_ui 按钮隐藏修复 | ⬜ Pending |
| 12 | Cherry-pick `fix-linux-rounding-sidebar` → Linux 侧栏圆角修复 | ⬜ Pending |
| 13 | Cherry-pick `fix-onboarding-telemetry` → 遥测修复 | ⬜ Pending |

---

## 详细说明

### 1. Merge upstream/main
- **来源**: `remotes/upstream/main`
- **内容**: Skills 基础功能、ACP extensions 迁移、各类崩溃/竞态修复
- **注意事项**: 可能有大量冲突，特别是汉化相关代码（settings, agent_ui 中的用户可见字符串）
- **编译验证**: `cargo check --workspace`

### 2. agent-panel-menu-items
- **提交**: `0e6c5633f7`, `92f9652e5e`, `c5e44cbafb`
- **内容**: 在 agent panel 省略号菜单中添加 Skills 和 Rules 分组
- **文件**: `crates/agent_ui/src/agent_panel.rs`

### 3. create-skill-prevent-dupe
- **提交**: `2718a12697`
- **内容**: 在 create-skill 内置 skill 中添加防重复创建提示
- **文件**: `crates/agent_skills/builtin/create-skill/SKILL.md`

### 4. martin/ai-234-lag
- **提交**: `d128b2302f`, `54f1952537`, `250def8169`
- **内容**: Agent read_file outline 输出跳过语法高亮 + markdown 渲染优化
- **文件**: `crates/agent/src/tools/read_file_tool.rs`, markdown

### 5. link-backticked-file-paths
- **提交**: `f972d13551`, `0f6fc81060`
- **内容**: Agent 面板中反引号内的文件路径变为可点击链接
- **文件**: `crates/agent_ui/src/conversation_view.rs`, `crates/markdown/src/markdown.rs`, `crates/workspace/src/path_link.rs` (新文件)

### 6. martin/ai-233
- **提交**: `369d209879`
- **内容**: 空路径代码块渲染为文件引用 pill
- **文件**: agent 相关

### 7. kb/multi-buffer-auto-reveal
- **提交**: `d1ff13a90a`
- **内容**: 修复多缓冲导航时项目面板不自动跟随
- **文件**: `crates/editor/`, `crates/workspace/`, `crates/project_panel/`

### 8-13. 各类 Bug 修复
- 见上方任务列表，每个任务一个独立提交/branch

---

## 汉化影响注意事项
- Skills 相关的用户可见字符串（菜单项、设置页面、toast 提示）需要本地化
- `agent_panel.rs` 中的菜单标签
- `skill_creator` 中的表单标签和验证消息
- `settings_ui` 中的 Skills 设置页面
- 所有新增的 doc comments / help text
