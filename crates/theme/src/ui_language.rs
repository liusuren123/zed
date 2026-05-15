use collections::HashMap;
use gpui::{App, Global, SharedString};
use std::sync::OnceLock;

/// The language to use for the UI.
#[derive(Debug, Default, PartialEq, Eq, Hash, Clone, Copy)]
pub enum UiLanguage {
    /// Display the UI in English.
    #[default]
    English,
    /// Display the UI in Simplified Chinese (中文简体).
    ChineseSimplified,
}

/// A GPUI global holding the current UI language. Updated whenever ThemeSettings changes.
#[derive(Default)]
pub struct GlobalUiLanguage(pub UiLanguage);

impl Global for GlobalUiLanguage {}

/// Returns the current UI language from the application context.
pub fn current_ui_language(cx: &App) -> UiLanguage {
    cx.try_global::<GlobalUiLanguage>()
        .map(|g| g.0)
        .unwrap_or_default()
}

/// Translate `key` (an English string) into the currently-active UI language.
///
/// When the language is English the key itself is returned without any allocation.
/// For other languages the translation table is consulted and the result is returned
/// as a `SharedString`. If no translation is found the original key is returned.
pub fn translate(key: &'static str, cx: &App) -> SharedString {
    match current_ui_language(cx) {
        UiLanguage::English => SharedString::from(key),
        UiLanguage::ChineseSimplified => zh_cn_translations()
            .get(key)
            .copied()
            .map(SharedString::from)
            .unwrap_or_else(|| SharedString::from(key)),
    }
}

/// Shorthand: translate a key using the current language.
///
/// Usage: `t("Save", cx)`
#[inline]
pub fn t(key: &'static str, cx: &App) -> SharedString {
    translate(key, cx)
}

static ZH_CN_TRANSLATIONS: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

fn zh_cn_translations() -> &'static HashMap<&'static str, &'static str> {
    ZH_CN_TRANSLATIONS.get_or_init(|| {
        let mut map = HashMap::default();

        // ════════════════════════════════════════════════════════════════════
        // 一、顶部菜单栏
        // ════════════════════════════════════════════════════════════════════
        map.insert("File", "文件");
        map.insert("Edit", "编辑");
        map.insert("Selection", "选择");
        map.insert("View", "视图");
        map.insert("Go", "跳转");
        map.insert("Run", "运行");
        map.insert("Window", "窗口");
        map.insert("Help", "帮助");

        // ── Zed 菜单 ─────────────────────────────────────────────────────────
        map.insert("About Zed", "关于 Zed");
        map.insert("Check for Updates", "检查更新");
        map.insert("Open Settings", "打开设置");
        map.insert("Open Settings File", "打开设置文件");
        map.insert("Open Project Settings", "打开项目设置");
        map.insert("Open Project Settings File", "打开项目设置文件");
        map.insert("Open Default Settings", "打开默认设置");
        map.insert("Open Keymap", "打开键位映射");
        map.insert("Open Keymap File", "打开键位映射文件");
        map.insert("Open Default Key Bindings", "打开默认键位绑定");
        map.insert("Select Theme...", "选择主题\u{2026}");
        map.insert("Select Icon Theme...", "选择图标主题\u{2026}");
        map.insert("Extensions", "扩展");
        map.insert("Quit Zed", "退出 Zed");

        // ── 用户菜单（右上角头像菜单）───────────────────────────────────────
        map.insert("Settings", "设置");
        map.insert("Keymap", "键位映射");
        map.insert("Themes\u{2026}", "主题\u{2026}");
        map.insert("Icon Themes\u{2026}", "图标主题\u{2026}");
        map.insert("Panel Layout", "面板布局");
        map.insert("Classic", "经典");
        map.insert("Agentic", "智能体");
        map.insert("Custom", "自定义");
        map.insert("Sign Out", "退出登录");
        map.insert("Sign In", "登录");

        // ── File 菜单 ─────────────────────────────────────────────────────────
        map.insert("New", "新建");
        map.insert("New Window", "新建窗口");
        map.insert("Open File...", "打开文件\u{2026}");
        map.insert("Open Folder...", "打开文件夹\u{2026}");
        map.insert("Open Recent...", "打开最近\u{2026}");
        map.insert("Open Remote...", "打开远程\u{2026}");
        map.insert("Add Folder to Project...", "将文件夹添加到项目\u{2026}");
        map.insert("Save", "保存");
        map.insert("Save As...", "另存为\u{2026}");
        map.insert("Save All", "全部保存");
        map.insert("Close Editor", "关闭编辑器");
        map.insert("Close Project", "关闭项目");
        map.insert("Close Window", "关闭窗口");

        // ── Edit 菜单 ─────────────────────────────────────────────────────────
        map.insert("Undo", "撤销");
        map.insert("Redo", "重做");
        map.insert("Cut", "剪切");
        map.insert("Copy", "复制");
        map.insert("Copy and Trim", "复制并修剪");
        map.insert("Paste", "粘贴");
        map.insert("Find", "查找");
        map.insert("Find in Project", "在项目中查找");
        map.insert("Toggle Line Comment", "切换行注释");

        // ── Selection 菜单 ────────────────────────────────────────────────────
        map.insert("Select All", "全选");
        map.insert("Expand Selection", "扩展选区");
        map.insert("Shrink Selection", "收缩选区");
        map.insert("Select Next Sibling", "选择下一个同级");
        map.insert("Select Previous Sibling", "选择上一个同级");
        map.insert("Add Cursor Above", "在上方添加光标");
        map.insert("Add Cursor Below", "在下方添加光标");
        map.insert("Select Next Occurrence", "选择下一个匹配项");
        map.insert("Select Previous Occurrence", "选择上一个匹配项");
        map.insert("Select All Occurrences", "选择所有匹配项");
        map.insert("Move Line Up", "上移行");
        map.insert("Move Line Down", "下移行");
        map.insert("Duplicate Selection", "复制选区");

        // ── Go 菜单 ───────────────────────────────────────────────────────────
        map.insert("Back", "后退");
        map.insert("Forward", "前进");
        map.insert("Command Palette...", "命令面板\u{2026}");
        map.insert("Go to File...", "跳转到文件\u{2026}");
        map.insert("Go to Symbol in Editor...", "跳转到编辑器中的符号\u{2026}");
        map.insert("Go to Line/Column...", "跳转到行/列\u{2026}");
        map.insert("Go to Definition", "跳转到定义");
        map.insert("Go to Declaration", "跳转到声明");
        map.insert("Go to Type Definition", "跳转到类型定义");
        map.insert("Find All References", "查找所有引用");
        map.insert("Next Problem", "下一个问题");
        map.insert("Previous Problem", "上一个问题");

        // ── View 菜单 ─────────────────────────────────────────────────────────
        map.insert("Zoom In", "放大");
        map.insert("Zoom Out", "缩小");
        map.insert("Reset Zoom", "重置缩放");
        map.insert("Reset All Zoom", "重置所有缩放");
        map.insert("Toggle Left Dock", "切换左侧停靠栏");
        map.insert("Toggle Right Dock", "切换右侧停靠栏");
        map.insert("Toggle Bottom Dock", "切换底部停靠栏");
        map.insert("Toggle All Docks", "切换所有停靠栏");
        map.insert("Split Up", "向上拆分");
        map.insert("Split Down", "向下拆分");
        map.insert("Split Left", "向左拆分");
        map.insert("Split Right", "向右拆分");
        map.insert("Project Panel", "项目面板");
        map.insert("Outline Panel", "大纲面板");
        map.insert("Collab Panel", "协作面板");
        map.insert("Terminal Panel", "终端面板");
        map.insert("Debugger Panel", "调试器面板");
        map.insert("Diagnostics", "诊断");

        // ── Run 菜单 ──────────────────────────────────────────────────────────
        map.insert("Spawn Task", "创建任务");
        map.insert("Start Debugger", "启动调试器");
        map.insert("Edit tasks.json...", "编辑 tasks.json\u{2026}");
        map.insert("Edit debug.json...", "编辑 debug.json\u{2026}");
        map.insert("Continue", "继续");
        map.insert("Step Over", "单步跳过");
        map.insert("Step Into", "单步进入");
        map.insert("Step Out", "单步退出");
        map.insert("Toggle Breakpoint", "切换断点");
        map.insert("Edit Breakpoint", "编辑断点");
        map.insert("Clear All Breakpoints", "清除所有断点");

        // ── Window 菜单 ───────────────────────────────────────────────────────
        map.insert("Minimize", "最小化");
        map.insert("Zoom", "缩放");

        // ── Help 菜单 ─────────────────────────────────────────────────────────
        map.insert("View Release Notes Locally", "查看本地发行说明");
        map.insert("View Telemetry", "查看遥测数据");
        map.insert("View Dependency Licenses", "查看依赖许可证");
        map.insert("Show Welcome", "显示欢迎页");
        map.insert("File Bug Report...", "提交 Bug 报告\u{2026}");
        map.insert("Request Feature...", "请求功能\u{2026}");
        map.insert("Email Us...", "发送邮件\u{2026}");
        map.insert("Documentation", "文档");
        map.insert("Zed Repository", "Zed 代码仓库");
        map.insert("Zed Twitter", "Zed Twitter");
        map.insert("Join the Team", "加入团队");

        // ── 通话控件 ──────────────────────────────────────────────────────────
        map.insert("Leave Call", "离开通话");
        map.insert("Stop Sharing Screen", "停止共享屏幕");
        map.insert("Share Screen", "共享屏幕");
        map.insert("Unshare", "取消分享");
        map.insert("Share", "分享");
        map.insert("Stop sharing project with call participants", "停止与通话参与者共享项目");
        map.insert("Share project with call participants", "与通话参与者共享项目");
        map.insert("This project may not be shared in a public channel.", "此项目不能在公开频道中共享。");
        map.insert("Unmute Microphone", "取消麦克风静音");
        map.insert("Mute Microphone", "麦克风静音");
        map.insert("Audio will be unmuted", "音频将取消静音");
        map.insert("Microphone will be muted", "麦克风将静音");
        map.insert("Unmute Audio", "取消音频静音");
        map.insert("Mute Audio", "音频静音");
        map.insert("Excellent", "极佳");
        map.insert("Good", "良好");
        map.insert("Poor", "较差");
        map.insert("Lost", "已断开");
        map.insert("Unknown screen", "未知屏幕");
        map.insert("Sharing Screen Failed", "屏幕共享失败");
        map.insert("Screen Share Enabled", "屏幕共享已启用");
        map.insert("Screen Share Disabled", "屏幕共享已禁用");
        map.insert("Microphone Enabled", "麦克风已启用");
        map.insert("Microphone Disabled", "麦克风已禁用");

        // ════════════════════════════════════════════════════════════════════
        // 二、设置页面标题（SettingsPage title）
        // ════════════════════════════════════════════════════════════════════
        map.insert("General", "常规");
        map.insert("Appearance", "外观");
        map.insert("Editor", "编辑器");
        map.insert("Languages & Tools", "语言与工具");
        map.insert("Search & Files", "搜索与文件");
        map.insert("Window & Layout", "窗口与布局");
        map.insert("Panels", "面板");
        map.insert("Debugger", "调试器");
        map.insert("Terminal", "终端");
        map.insert("Version Control", "版本控制");
        map.insert("Collaboration", "协作");
        map.insert("AI", "人工智能");
        map.insert("Network", "网络");
        map.insert("Developer", "开发者");

        // ════════════════════════════════════════════════════════════════════
        // 三、设置分节标题（SectionHeader）
        // ════════════════════════════════════════════════════════════════════
        map.insert("General Settings", "常规设置");
        map.insert("Security", "安全");
        map.insert("Workspace Restoration", "工作区恢复");
        map.insert("Scoped Settings", "作用域设置");
        map.insert("Privacy", "隐私");
        map.insert("Auto Update", "自动更新");
        map.insert("Theme", "主题");
        map.insert("Buffer Font", "缓冲区字体");
        map.insert("UI Font", "界面字体");
        map.insert("Agent Panel Font", "智能体面板字体");
        map.insert("Text Rendering", "文本渲染");
        map.insert("Cursor", "光标");
        map.insert("Highlighting", "高亮");
        map.insert("Guides", "参考线");
        map.insert("Keybindings", "键位绑定");
        map.insert("Base Keymap", "基础键位映射");
        map.insert("Modal Editing", "模态编辑");
        map.insert("Auto Save", "自动保存");
        map.insert("Which-key Menu", "Which-key 菜单");
        map.insert("Multibuffer", "多缓冲区");
        map.insert("Scrolling", "滚动");
        map.insert("Signature Help", "签名帮助");
        map.insert("Hover Popover", "悬停弹出框");
        map.insert("Drag And Drop Selection", "拖拽选择");
        map.insert("Gutter", "装订线");
        map.insert("Scrollbar", "滚动条");
        map.insert("Minimap", "缩略图");
        map.insert("Toolbar", "工具栏");
        map.insert("Vim", "Vim");
        map.insert("Indentation", "缩进");
        map.insert("Wrapping", "换行");
        map.insert("Indent Guides", "缩进参考线");
        map.insert("Formatting", "格式化");
        map.insert("Autoclose", "自动关闭");
        map.insert("Whitespace", "空白字符");
        map.insert("Completions", "补全");
        map.insert("Inlay Hints", "内嵌提示");
        map.insert("Tasks", "任务");
        map.insert("Miscellaneous", "其他");
        map.insert("LSP", "LSP");
        map.insert("LSP Completions", "LSP 补全");
        map.insert("Debuggers", "调试器");
        map.insert("Prettier", "Prettier");
        map.insert("File Types", "文件类型");
        map.insert("Inline Diagnostics", "内联诊断");
        map.insert("LSP Pull Diagnostics", "LSP 拉取诊断");
        map.insert("LSP Highlights", "LSP 高亮");
        map.insert("Languages", "语言");
        map.insert("Search", "搜索");
        map.insert("File Finder", "文件查找器");
        map.insert("File Scan", "文件扫描");
        map.insert("Status Bar", "状态栏");
        map.insert("Title Bar", "标题栏");
        map.insert("Tab Bar", "标签栏");
        map.insert("Tab Settings", "标签设置");
        map.insert("Preview Tabs", "预览标签");
        map.insert("Layout", "布局");
        map.insert("Window", "窗口");
        map.insert("Pane Modifiers", "窗格修饰");
        map.insert("Pane Split Direction", "窗格拆分方向");
        map.insert("Collaboration Panel", "协作面板");
        map.insert("Git Panel", "Git 面板");
        map.insert("Agent Panel", "智能体面板");
        map.insert("Git Integration", "Git 集成");
        map.insert("Git Gutter", "Git 装订线");
        map.insert("Inline Git Blame", "内联 Git 追责");
        map.insert("Git Blame View", "Git 追责视图");
        map.insert("Branch Picker", "分支选择器");
        map.insert("Git Hunks", "Git 变更块");
        map.insert("Calls", "通话");
        map.insert("Agent Configuration", "智能体配置");
        map.insert("Context Servers", "上下文服务器");
        map.insert("Edit Predictions", "编辑预测");
        map.insert("Environment", "环境");
        map.insert("Font", "字体");
        map.insert("Display Settings", "显示设置");
        map.insert("Behavior Settings", "行为设置");
        map.insert("Layout Settings", "布局设置");
        map.insert("Advanced Settings", "高级设置");
        map.insert("Feature Flags", "功能标志");
        map.insert("Instrumentation", "性能监测");

        // ════════════════════════════════════════════════════════════════════
        // 四、Language & UI 选择器（Appearance 页面顶部）
        // ════════════════════════════════════════════════════════════════════
        map.insert("Language", "语言");
        map.insert("UI Language", "界面语言");
        map.insert(
            "Choose the language used for the Zed user interface.",
            "选择 Zed 用户界面所使用的语言。",
        );

        // ════════════════════════════════════════════════════════════════════
        // 五、设置项标题（Setting Item Title）
        // ════════════════════════════════════════════════════════════════════

        // ── General 页 ───────────────────────────────────────────────────────
        map.insert("When Closing With No Tabs", "无标签时关闭行为");
        map.insert("On Last Window Closed", "关闭最后一个窗口时");
        map.insert("Use System Path Prompts", "使用系统路径对话框");
        map.insert("Use System Prompts", "使用系统对话框");
        map.insert("Redact Private Values", "隐藏私密值");
        map.insert("Private Files", "私密文件");
        map.insert("CLI Default Open Behavior", "CLI 默认打开行为");
        map.insert("Trust All Projects By Default", "默认信任所有项目");
        map.insert("Restore Unsaved Buffers", "恢复未保存的缓冲区");
        map.insert("Restore On Startup", "启动时恢复");
        map.insert("Preview Channel", "预览频道");
        map.insert("Settings Profiles", "设置配置文件");
        map.insert("Telemetry Diagnostics", "遥测诊断");
        map.insert("Telemetry Metrics", "遥测指标");
        map.insert("Hide Mouse", "隐藏鼠标");
        map.insert("Focus Follows Mouse", "焦点跟随鼠标");
        map.insert("Focus Follows Mouse Debounce ms", "焦点跟随鼠标防抖毫秒数");

        // ── Appearance 页 ────────────────────────────────────────────────────
        map.insert("Theme Mode", "主题模式");
        map.insert("Theme Name", "主题名称");
        map.insert("Mode", "模式");
        map.insert("Light Theme", "浅色主题");
        map.insert("Dark Theme", "深色主题");
        map.insert("Light Icon Theme", "浅色图标主题");
        map.insert("Dark Icon Theme", "深色图标主题");
        map.insert("Icon Theme", "图标主题");
        map.insert("Icon Theme Name", "图标主题名称");
        map.insert("Icon Theme Mode", "图标主题模式");
        map.insert("Font Family", "字体族");
        map.insert("Font Size", "字号");
        map.insert("Font Weight", "字重");
        map.insert("Font Features", "字体特性");
        map.insert("Font Fallbacks", "字体回退");
        map.insert("Line Height", "行高");
        map.insert("UI Font Size", "界面字体大小");
        map.insert("Buffer Font Size", "缓冲区字体大小");
        map.insert("Text Rendering Mode", "文本渲染模式");
        map.insert("Multi Cursor Modifier", "多光标修饰键");
        map.insert("Unnecessary Code Fade", "不必要代码淡化");
        map.insert("Inactive Opacity", "非活动透明度");
        map.insert("Rounded Selection", "圆角选区");
        map.insert("Border Size", "边框大小");
        map.insert("Zoomed Padding", "缩放填充");

        // ── Keymap 页 ────────────────────────────────────────────────────────
        map.insert("Edit Keybindings", "编辑键位绑定");
        map.insert("Vim Mode", "Vim 模式");
        map.insert("Helix Mode", "Helix 模式");
        map.insert("Vim/Emacs Modeline Support", "Vim/Emacs 模式行支持");
        map.insert("Custom Digraphs", "自定义二合字");
        map.insert("Global Substitution Default", "全局替换默认值");
        map.insert("Regex Search", "正则搜索");
        map.insert("Relative Line Numbers", "相对行号");
        map.insert("Toggle Relative Line Numbers", "切换相对行号");
        map.insert("Highlight on Yank Duration", "复制高亮持续时间");
        map.insert("Use Smartcase Find", "使用智能大小写查找");
        map.insert("Default Mode", "默认模式");
        map.insert("Cursor Shape - Insert Mode", "光标形状 - 插入模式");
        map.insert("Cursor Shape - Normal Mode", "光标形状 - 普通模式");
        map.insert("Cursor Shape - Replace Mode", "光标形状 - 替换模式");
        map.insert("Cursor Shape - Visual Mode", "光标形状 - 可视模式");
        map.insert("Cursors", "光标");
        map.insert("Toggle On Modifiers Press", "按修饰键时切换");

        // ── Editor 页 ────────────────────────────────────────────────────────
        map.insert("Auto Save Mode", "自动保存模式");
        map.insert("Show Which-key Menu", "显示 Which-key 菜单");
        map.insert("Menu Delay", "菜单延迟");
        map.insert("Double Click In Multibuffer", "在多缓冲区中双击");
        map.insert("Expand Excerpt Lines", "展开摘录行数");
        map.insert("Excerpt Context Lines", "摘录上下文行数");
        map.insert("Expand Outlines With Depth", "按深度展开大纲");
        map.insert("Diff View Style", "差异视图样式");
        map.insert("Minimum Split Diff Width", "最小拆分差异宽度");
        map.insert("Word Diff Enabled", "启用词级差异");
        map.insert("Scroll Beyond Last Line", "滚动超出最后一行");
        map.insert("Vertical Scroll Margin", "垂直滚动边距");
        map.insert("Horizontal Scroll Margin", "水平滚动边距");
        map.insert("Scroll Sensitivity", "滚动灵敏度");
        map.insert("Mouse Wheel Zoom", "鼠标滚轮缩放");
        map.insert("Fast Scroll Sensitivity", "快速滚动灵敏度");
        map.insert("Autoscroll On Clicks", "点击时自动滚动");
        map.insert("Sticky Scroll", "粘性滚动");
        map.insert("Scroll Debounce Ms", "滚动防抖毫秒数");
        map.insert("Auto Signature Help", "自动签名帮助");
        map.insert("Show Signature Help After Edits", "编辑后显示签名帮助");
        map.insert("Snippet Sort Order", "代码片段排序方式");
        map.insert("Enabled", "已启用");
        map.insert("Hiding Delay", "隐藏延迟");
        map.insert("Cursor Blink", "光标闪烁");
        map.insert("Cursor Shape", "光标形状");
        map.insert("Active Line Width", "活动行宽度");
        map.insert("Show Line Numbers", "显示行号");
        map.insert("Min Line Number Digits", "最小行号位数");
        map.insert("Show Folds", "显示折叠");
        map.insert("Show Runnables", "显示可运行项");
        map.insert("Show Breakpoints", "显示断点");
        map.insert("Show Bookmarks", "显示书签");
        map.insert("Code Actions", "代码操作");
        map.insert("Diff Stats", "差异统计");
        map.insert("Breadcrumbs", "面包屑导航");
        map.insert("Quick Actions", "快捷操作");
        map.insert("Selections Menu", "选区菜单");
        map.insert("Agent Review", "智能体审查");
        map.insert("Coloring", "着色");
        map.insert("Background Coloring", "背景着色");
        map.insert("Show", "显示");
        map.insert("Thumb", "滑块");
        map.insert("Thumb Border", "滑块边框");
        map.insert("Scroll Bar", "滚动条");
        map.insert("Vertical Scrollbar", "垂直滚动条");
        map.insert("Horizontal Scrollbar", "水平滚动条");
        map.insert("Search Results", "搜索结果");
        map.insert("Selected Text", "选中文本");
        map.insert("Selected Symbol", "选中符号");
        map.insert("Selection Highlight", "选区高亮");
        map.insert("Display In", "显示位置");
        map.insert("Show Wrap Guides", "显示换行参考线");
        map.insert("Wrap Guides", "换行参考线");
        map.insert("Show Completion Documentation", "显示补全文档");
        map.insert("Show Completions On Input", "输入时显示补全");
        map.insert("Completion Detail Alignment", "补全详情对齐方式");
        map.insert("Completion Menu Scrollbar", "补全菜单滚动条");
        map.insert("Words", "单词");
        map.insert("Words Min Length", "最小单词长度");
        map.insert("Code Actions On Format", "格式化时执行代码操作");
        map.insert("Format On Save", "保存时格式化");
        map.insert("Formatter", "格式化器");
        map.insert("Ensure Final Newline On Save", "保存时确保末尾换行");
        map.insert("Remove Trailing Whitespace On Save", "保存时删除行尾空白");
        map.insert("Use Autoclose", "使用自动关闭");
        map.insert("Use Auto Surround", "使用自动包围");
        map.insert("Always Treat Brackets As Autoclosed", "始终将括号视为自动关闭");
        map.insert("JSX Tag Auto Close", "JSX 标签自动关闭");
        map.insert("Space Whitespace Indicator", "空格空白指示符");
        map.insert("Tab Whitespace Indicator", "制表符空白指示符");
        map.insert("Show Other Hints", "显示其他提示");
        map.insert("Show Parameter Hints", "显示参数提示");
        map.insert("Show Type Hints", "显示类型提示");
        map.insert("Show Value Hints", "显示值提示");
        map.insert("Edit Debounce Ms", "编辑防抖毫秒数");
        map.insert("Update Debounce", "更新防抖");
        map.insert("Colorize Brackets", "括号着色");
        map.insert("Code Lens", "代码镜头");
        map.insert("LSP Document Colors", "LSP 文档颜色");
        map.insert("Auto Indent", "自动缩进");
        map.insert("Auto Indent On Paste", "粘贴时自动缩进");
        map.insert("Hard Tabs", "硬制表符");
        map.insert("Tab Size", "制表符大小");
        map.insert("Preferred Line Length", "首选行长度");
        map.insert("Soft Wrap", "软换行");
        map.insert("Custom Line Height", "自定义行高");
        map.insert("Auto Replace Emoji Shortcode", "自动替换 Emoji 短码");
        map.insert("Drag and Drop", "拖放");
        map.insert("Drop Size Target", "拖放大小目标");
        map.insert("Middle Click Paste", "中键粘贴");
        map.insert("Image Viewer", "图片查看器");
        map.insert("Allow Rewrap", "允许重排");
        map.insert("Extend Comment On Newline", "换行时延续注释");
        map.insert("Use On Type Format", "输入时格式化");
        map.insert("Prefer LSP", "优先使用 LSP");

        // ── Languages & Tools 页 ─────────────────────────────────────────────
        map.insert("Enable Language Server", "启用语言服务器");
        map.insert("Language Servers", "语言服务器");
        map.insert("Linked Edits", "关联编辑");
        map.insert("Go To Definition Fallback", "跳转到定义回退策略");
        map.insert("Go To Definition Scroll Strategy", "跳转到定义滚动策略");
        map.insert("Semantic Tokens", "语义标记");
        map.insert("LSP Folding Ranges", "LSP 折叠范围");
        map.insert("LSP Document Symbols", "LSP 文档符号");
        map.insert("Fetch Timeout (milliseconds)", "获取超时（毫秒）");
        map.insert("Insert Mode", "插入模式");
        map.insert("Allowed", "已允许");
        map.insert("Parser", "解析器");
        map.insert("Plugins", "插件");
        map.insert("Options", "选项");
        map.insert("File Type Associations", "文件类型关联");
        map.insert("Max Severity", "最大严重性");
        map.insert("Include Warnings", "包含警告");
        map.insert("Inline Diagnostics", "内联诊断");
        map.insert("Tab Show Diagnostics", "标签显示诊断");
        map.insert("Hidden Files", "隐藏文件");

        // ── Search & Files 页 ────────────────────────────────────────────────
        map.insert("Whole Word", "整词匹配");
        map.insert("Case Sensitive", "区分大小写");
        map.insert("Use Smartcase Search", "使用智能大小写搜索");
        map.insert("Include Ignored", "包含已忽略项");
        map.insert("Regex", "正则表达式");
        map.insert("Search Wrap", "搜索循环");
        map.insert("Center on Match", "以匹配项为中心");
        map.insert("Seed Search Query From Cursor", "从光标初始化搜索查询");
        map.insert("Include Ignored in Search", "搜索中包含已忽略项");
        map.insert("File Icons", "文件图标");
        map.insert("Modal Max Width", "模态窗口最大宽度");
        map.insert("Skip Focus For Active In Search", "搜索中跳过活动文件焦点");
        map.insert("File Scan Exclusions", "文件扫描排除项");
        map.insert("File Scan Inclusions", "文件扫描包含项");
        map.insert("Restore File State", "恢复文件状态");
        map.insert("Close on File Delete", "文件删除时关闭");

        // ── Window & Layout 页 ───────────────────────────────────────────────
        map.insert("Project Panel Button", "项目面板按钮");
        map.insert("Active Language Button", "活动语言按钮");
        map.insert("Active Encoding Button", "活动编码按钮");
        map.insert("Cursor Position Button", "光标位置按钮");
        map.insert("Terminal Button", "终端按钮");
        map.insert("Diagnostics Button", "诊断按钮");
        map.insert("Project Search Button", "项目搜索按钮");
        map.insert("Debugger Button", "调试器按钮");
        map.insert("Active File Name", "活动文件名");
        map.insert("Collaboration Panel Button", "协作面板按钮");
        map.insert("Outline Panel Button", "大纲面板按钮");
        map.insert("Git Panel Button", "Git 面板按钮");
        map.insert("Agent Panel Button", "智能体面板按钮");
        map.insert("Show Branch Status Icon", "显示分支状态图标");
        map.insert("Show Branch Name", "显示分支名称");
        map.insert("Show Project Items", "显示项目条目");
        map.insert("Show Onboarding Banner", "显示引导横幅");
        map.insert("Show Sign In", "显示登录");
        map.insert("Show User Menu", "显示用户菜单");
        map.insert("Show User Picture", "显示用户头像");
        map.insert("Show Menus", "显示菜单");
        map.insert("Button Layout", "按钮布局");
        map.insert("Custom Button Layout", "自定义按钮布局");
        map.insert("Show Tab Bar", "显示标签栏");
        map.insert("Show Git Status In Tabs", "在标签中显示 Git 状态");
        map.insert("Show File Icons In Tabs", "在标签中显示文件图标");
        map.insert("Tab Close Position", "标签关闭按钮位置");
        map.insert("Maximum Tabs", "最大标签数");
        map.insert("Show Navigation History Buttons", "显示导航历史按钮");
        map.insert("Show Tab Bar Buttons", "显示标签栏按钮");
        map.insert("Preview Tabs Enabled", "启用预览标签");
        map.insert("Enable Preview From File Finder", "从文件查找器打开预览");
        map.insert("Enable Preview From Multibuffer", "从多缓冲区打开预览");
        map.insert("Enable Preview From Project Panel", "从项目面板打开预览");
        map.insert("Enable Preview File From Code Navigation", "从代码导航打开预览文件");
        map.insert("Enable Preview Multibuffer From Code Navigation", "从代码导航打开多缓冲区预览");
        map.insert("Enable Keep Preview On Code Navigation", "代码导航时保留预览");
        map.insert("Pinned Tabs Layout", "固定标签布局");
        map.insert("Bottom Dock Layout", "底部停靠栏布局");
        map.insert("Horizontal Split Direction", "水平拆分方向");
        map.insert("Vertical Split Direction", "垂直拆分方向");
        map.insert("Centered Layout Left Padding", "居中布局左填充");
        map.insert("Centered Layout Right Padding", "居中布局右填充");
        map.insert("Limit Content Width", "限制内容宽度");
        map.insert("Max Content Width", "最大内容宽度");
        map.insert("Max Width Columns", "最大列宽");
        map.insert("Window Decorations", "窗口装饰");
        map.insert("Use System Window Tabs", "使用系统窗口标签");
        map.insert("Activate On Close", "关闭时激活");

        // ── Panels 页 ────────────────────────────────────────────────────────
        map.insert("Project Panel Dock", "项目面板停靠位置");
        map.insert("Project Panel Default Width", "项目面板默认宽度");
        map.insert("Hide .gitignore", "隐藏 .gitignore 项");
        map.insert("Entry Spacing", "条目间距");
        map.insert("Folder Icons", "文件夹图标");
        map.insert("Git Status", "Git 状态");
        map.insert("Indent Size", "缩进大小");
        map.insert("Auto Reveal Entries", "自动展示条目");
        map.insert("Starts Open", "启动时打开");
        map.insert("Auto Fold Directories", "自动折叠目录");
        map.insert("Bold Folder Labels", "文件夹标签加粗");
        map.insert("Show Scrollbar", "显示滚动条");
        map.insert("Horizontal Scroll", "水平滚动");
        map.insert("Show Diagnostics", "显示诊断");
        map.insert("Diagnostic Badges", "诊断徽章");
        map.insert("Git Status Indicator", "Git 状态指示器");
        map.insert("Show Indent Guides", "显示缩进参考线");
        map.insert("Hide Root", "隐藏根节点");
        map.insert("Hide Hidden", "隐藏隐藏项");
        map.insert("Sort Mode", "排序模式");
        map.insert("Sort Order", "排序方式");
        map.insert("Sort By Path", "按路径排序");
        map.insert("Auto Open Files On Create", "创建时自动打开文件");
        map.insert("Auto Open Files On Paste", "粘贴时自动打开文件");
        map.insert("Auto Open Files On Drop", "拖入时自动打开文件");
        map.insert("Tree View", "树形视图");
        map.insert("Sticky", "固定");
        map.insert("Collaboration Panel Dock", "协作面板停靠位置");
        map.insert("Collaboration Panel Default Width", "协作面板默认宽度");
        map.insert("Git Panel Dock", "Git 面板停靠位置");
        map.insert("Git Panel Default Width", "Git 面板默认宽度");
        map.insert("Git Panel Status Style", "Git 面板状态样式");
        map.insert("Collapse Untracked Diff", "折叠未跟踪差异");
        map.insert("Commit Title Max Length", "提交标题最大长度");
        map.insert("Diff Stats", "差异统计");
        map.insert("Show Count Badge", "显示数量徽章");
        map.insert("Outline Panel Dock", "大纲面板停靠位置");
        map.insert("Outline Panel Default Width", "大纲面板默认宽度");
        map.insert("Outline Panel Button", "大纲面板按钮");
        map.insert("Agent Panel Dock", "智能体面板停靠位置");
        map.insert("Agent Panel Default Width", "智能体面板默认宽度");
        map.insert("Agent Panel Default Height", "智能体面板默认高度");
        map.insert("Agent Panel Flexible Sizing", "智能体面板弹性大小");
        map.insert("Terminal Dock", "终端停靠位置");
        map.insert("Terminal Panel Flexible Sizing", "终端面板弹性大小");

        // ── Debugger 页 ──────────────────────────────────────────────────────
        map.insert("Stepping Granularity", "单步粒度");
        map.insert("Save Breakpoints", "保存断点");
        map.insert("Timeout", "超时");
        map.insert("Log DAP Communications", "记录 DAP 通信");
        map.insert("Format DAP Log Messages", "格式化 DAP 日志消息");

        // ── Terminal 页 ──────────────────────────────────────────────────────
        map.insert("Shell", "Shell");
        map.insert("Working Directory", "工作目录");
        map.insert("Environment Variables", "环境变量");
        map.insert("Detect Virtual Environment", "检测虚拟环境");
        map.insert("Cursor Blinking", "光标闪烁");
        map.insert("Alternate Scroll", "备用滚动");
        map.insert("Minimum Contrast", "最小对比度");
        map.insert("Option As Meta", "Option 键作为 Meta 键");
        map.insert("Copy On Select", "选中即复制");
        map.insert("Keep Selection On Copy", "复制后保留选区");
        map.insert("Audible Bell", "声音提示");
        map.insert("Default Width", "默认宽度");
        map.insert("Default Height", "默认高度");
        map.insert("Max Scroll History Lines", "最大滚动历史行数");
        map.insert("Scroll Multiplier", "滚动倍数");
        map.insert("Title Override", "标题覆盖");
        map.insert("Program", "程序");
        map.insert("Arguments", "参数");
        map.insert("Directory", "目录");

        // ── Version Control 页 ──────────────────────────────────────────────
        map.insert("Disable Git Integration", "禁用 Git 集成");
        map.insert("Enable Git Status", "启用 Git 状态");
        map.insert("Enable Git Diff", "启用 Git 差异");
        map.insert("Git Diff", "Git 差异");
        map.insert("Visibility", "可见性");
        map.insert("Debounce", "防抖");
        map.insert("Delay", "延迟");
        map.insert("Padding", "填充");
        map.insert("Minimum Column", "最小列数");
        map.insert("Show Commit Summary", "显示提交摘要");
        map.insert("Show Avatar", "显示头像");
        map.insert("Show Author Name", "显示作者名称");
        map.insert("Hunk Style", "变更块样式");
        map.insert("Path Style", "路径样式");
        map.insert("Fallback Branch Name", "默认分支名称");

        // ── Collaboration 页 ─────────────────────────────────────────────────
        map.insert("Mute On Join", "加入时静音");
        map.insert("Share On Join", "加入时分享");
        map.insert("Test Audio", "测试音频");
        map.insert("Output Audio Device", "音频输出设备");
        map.insert("Input Audio Device", "音频输入设备");

        // ── AI 页 ────────────────────────────────────────────────────────────
        map.insert("Disable AI", "禁用 AI");
        map.insert("Threads Sidebar Side", "线程侧边栏位置");
        map.insert("Tool Permissions", "工具权限");
        map.insert("New Thread Location", "新线程位置");
        map.insert("Single File Review", "单文件审查");
        map.insert("Enable Feedback", "启用反馈");
        map.insert("Notify When Agent Waiting", "智能体等待时通知");
        map.insert("Play Sound When Agent Done", "智能体完成时播放声音");
        map.insert("Expand Edit Card", "展开编辑卡片");
        map.insert("Expand Terminal Card", "展开终端卡片");
        map.insert("Thinking Display", "思考过程显示");
        map.insert("Cancel Generation On Terminal Stop", "终端停止时取消生成");
        map.insert("Use Modifier To Send", "使用修饰键发送");
        map.insert("Message Editor Min Lines", "消息编辑器最小行数");
        map.insert("Show Turn Stats", "显示轮次统计");
        map.insert("Show Merge Conflict Indicator", "显示合并冲突指示器");
        map.insert("Context Server Timeout", "上下文服务器超时");
        map.insert("Configure Providers", "配置提供商");
        map.insert("Data Collection", "数据收集");
        map.insert("Show Edit Predictions", "显示编辑预测");
        map.insert("Show Edit Predictions in Normal Mode", "在普通模式下显示编辑预测");
        map.insert("Disable in Language Scopes", "在语言范围内禁用");
        map.insert("Display Mode", "显示模式");

        // ── Network 页 ──────────────────────────────────────────────────────
        map.insert("Proxy", "代理");
        map.insert("Server URL", "服务器 URL");

        // ── Developer 页 ────────────────────────────────────────────────────
        map.insert("Performance Profiler", "性能分析器");

        // ════════════════════════════════════════════════════════════════════
        // 六、设置项描述（Setting Item Description）
        // ════════════════════════════════════════════════════════════════════

        map.insert(
            "What to do when using the 'close active item' action with no tabs.",
            "使用\u{300c}关闭活动项\u{300d}操作但无标签时的行为。",
        );
        map.insert("What to do when the last window is closed.", "关闭最后一个窗口时的行为。");
        map.insert(
            "Use native OS dialogs for 'Open' and 'Save As'.",
            "对\u{300c}打开\u{300d}和\u{300c}另存为\u{300d}使用系统原生对话框。",
        );
        map.insert("Use native OS dialogs for confirmations.", "对确认操作使用系统原生对话框。");
        map.insert("Hide the values of variables in private files.", "在私密文件中隐藏变量的值。");
        map.insert(
            "Globs to match against file paths to determine if a file is private.",
            "用于匹配文件路径以判断文件是否为私密文件的 glob 模式。",
        );
        map.insert(
            "How Zed <path> opens directories when no flag is specified.",
            "未指定标志时 Zed <path> 打开目录的方式。",
        );
        map.insert(
            "When opening Zed, avoid Restricted Mode by auto-trusting all projects, enabling use of all features without having to give permission to each new project.",
            "打开 Zed 时，通过自动信任所有项目来避免受限模式，无需为每个新项目单独授权即可使用全部功能。",
        );
        map.insert(
            "Whether or not to restore unsaved buffers on restart.",
            "重启时是否恢复未保存的缓冲区。",
        );
        map.insert(
            "What to restore from the previous session when opening Zed.",
            "打开 Zed 时从上次会话中恢复的内容。",
        );
        map.insert(
            "Which settings should be activated only in Preview build of Zed.",
            "仅在 Zed 预览版中启用的设置。",
        );
        map.insert(
            "Any number of settings profiles that are temporarily applied on top of your existing user settings.",
            "可临时叠加在现有用户设置之上的任意数量的设置配置文件。",
        );
        map.insert("Send debug information like crash reports.", "发送崩溃报告等调试信息。");
        map.insert(
            "Send anonymized usage data like what languages you're using Zed with.",
            "发送匿名使用数据，例如您在 Zed 中使用的编程语言。",
        );
        map.insert("Whether or not to automatically check for updates.", "是否自动检查更新。");
        map.insert("When to hide the mouse cursor.", "何时隐藏鼠标光标。");
        map.insert("Amount of time to wait before changing focus.", "切换焦点前的等待时间。");
        map.insert(
            "Whether to change focus to a pane when the mouse hovers over it.",
            "鼠标悬停在窗格上时是否切换焦点。",
        );
        map.insert(
            "Choose a static, fixed theme or dynamically select themes based on appearance and light/dark modes.",
            "选择固定主题，或根据系统外观和明暗模式动态切换主题。",
        );
        map.insert("The name of your selected theme.", "所选主题的名称。");
        map.insert(
            "Choose whether to use the selected light or dark theme or to follow your OS appearance configuration.",
            "选择使用浅色或深色主题，或跟随操作系统的外观配置。",
        );
        map.insert(
            "The theme to use when mode is set to light, or when mode is set to system and it is in light mode.",
            "当模式设为浅色时，或模式设为跟随系统且系统处于浅色模式时使用的主题。",
        );
        map.insert(
            "The theme to use when mode is set to dark, or when mode is set to system and it is in dark mode.",
            "当模式设为深色时，或模式设为跟随系统且系统处于深色模式时使用的主题。",
        );
        map.insert(
            "The custom set of icons Zed will associate with files and directories.",
            "Zed 与文件和目录关联的自定义图标集。",
        );
        map.insert("The name of your selected icon theme.", "所选图标主题的名称。");
        map.insert(
            "Choose whether to use the selected light or dark icon theme or to follow your OS appearance configuration.",
            "选择使用浅色或深色图标主题，或跟随操作系统的外观配置。",
        );
        map.insert(
            "The icon theme to use when mode is set to light, or when mode is set to system and it is in light mode.",
            "当模式设为浅色时使用的图标主题。",
        );
        map.insert(
            "The icon theme to use when mode is set to dark, or when mode is set to system and it is in dark mode.",
            "当模式设为深色时使用的图标主题。",
        );
        map.insert("Font family for editor text.", "编辑器文本使用的字体族。");
        map.insert("Font family for UI elements.", "界面元素使用的字体族。");
        map.insert("Font size for editor text.", "编辑器文本的字号。");
        map.insert("Font size for UI elements.", "界面元素的字号。");
        map.insert("Font weight for editor text (100-900).", "编辑器文本的字重（100-900）。");
        map.insert("Font weight for UI elements (100-900).", "界面元素的字重（100-900）。");
        map.insert("Line height for editor text.", "编辑器文本的行高。");
        map.insert(
            "The OpenType features to enable for rendering in text buffers.",
            "在文本缓冲区渲染中启用的 OpenType 特性。",
        );
        map.insert(
            "The OpenType features to enable for rendering in UI elements.",
            "在界面元素渲染中启用的 OpenType 特性。",
        );
        map.insert(
            "The font fallbacks to use for rendering in text buffers.",
            "在文本缓冲区渲染中使用的字体回退列表。",
        );
        map.insert(
            "The font fallbacks to use for rendering in the UI.",
            "在界面渲染中使用的字体回退列表。",
        );
        map.insert(
            "Font size for agent response text in the agent panel. Falls back to the regular UI font size.",
            "智能体面板中智能体响应文本的字号，默认回退到界面字号。"
        );
        map.insert(
            "Font size for user messages text in the agent panel.",
            "智能体面板中用户消息文本的字号。"
        );
        map.insert("The text rendering mode to use.", "要使用的文本渲染模式。");
        map.insert("Modifier key for adding multiple cursors.", "用于添加多个光标的修饰键。");
        map.insert("How much to fade out unused code (0.0 - 0.9).", "不必要代码的淡化程度（0.0 - 0.9）。");
        map.insert("Opacity of inactive panels (0.0 - 1.0).", "非活动面板的不透明度（0.0 - 1.0）。");
        map.insert("Whether the text selection should have rounded corners.", "文本选区是否应有圆角。");
        map.insert("Size of the border surrounding the active pane.", "活动窗格周围边框的大小。");
        map.insert(
            "Customize keybindings in the keymap editor.",
            "在键位映射编辑器中自定义键位绑定。",
        );
        map.insert("The name of a base set of key bindings to use.", "要使用的基础键位绑定集名称。");
        map.insert("Enable Vim mode and key bindings.", "启用 Vim 模式和键位绑定。");
        map.insert("Enable Helix mode and key bindings.", "启用 Helix 模式和键位绑定。");
        map.insert("Number of lines to search for modelines (set to 0 to disable).", "搜索模式行的行数（设为 0 则禁用）。");
        map.insert("Custom digraph mappings for Vim mode.", "Vim 模式的自定义二合字映射。");
        map.insert(
            "When enabled, the :substitute command replaces all matches in a line by default. The 'g' flag then toggles this behavior.",
            "启用后，:substitute 命令默认替换一行中的所有匹配项。'g' 标志可切换此行为。",
        );
        map.insert("Use regex search by default in Vim search.", "在 Vim 搜索中默认使用正则表达式搜索。");
        map.insert("Toggle relative line numbers in Vim mode.", "在 Vim 模式中切换相对行号。");
        map.insert("Duration in milliseconds to highlight yanked text in Vim mode.", "Vim 模式中复制文本高亮的持续时间（毫秒）。");
        map.insert("Enable smartcase searching in Vim mode.", "在 Vim 模式中启用智能大小写搜索。");
        map.insert("Cursor shape for the editor.", "编辑器的光标形状。");
        map.insert("Cursor shape for insert mode. Inherit uses the editor's cursor shape.", "插入模式的光标形状。Inherit 使用编辑器的光标形状。");
        map.insert("Cursor shape for normal mode.", "普通模式的光标形状。");
        map.insert("Cursor shape for replace mode.", "替换模式的光标形状。");
        map.insert("Cursor shape for visual mode.", "可视模式的光标形状。");
        map.insert("Whether the cursor blinks in the editor.", "编辑器中光标是否闪烁。");
        map.insert("Toggles inlay hints (hides or shows) when the user presses the modifiers specified.", "用户按下指定修饰键时切换内嵌提示的显示/隐藏。");
        map.insert("When to auto save buffer changes.", "何时自动保存缓冲区更改。");
        map.insert(
            "Display the which-key menu with matching bindings while a multi-stroke binding is pending.",
            "在等待多键绑定期间显示匹配绑定的 which-key 菜单。",
        );
        map.insert(
            "Delay in milliseconds before the which-key menu appears.",
            "which-key 菜单出现前的延迟时间（毫秒）。",
        );
        map.insert(
            "What to do when multibuffer is double-clicked in some of its excerpts.",
            "在多缓冲区的某些摘录中双击时的行为。",
        );
        map.insert(
            "How many lines to expand the multibuffer excerpts by default.",
            "默认展开多缓冲区摘录的行数。",
        );
        map.insert(
            "How many lines of context to provide in multibuffer excerpts by default.",
            "多缓冲区摘录中默认提供的上下文行数。",
        );
        map.insert(
            "Default depth to expand outline items in the current file.",
            "当前文件中展开大纲条目的默认深度。",
        );
        map.insert("How to display diffs in the editor.", "在编辑器中显示差异的方式。");
        map.insert(
            "The minimum width (in columns) at which the split diff view is used. When the editor is narrower, the diff view automatically switches to unified mode. Set to 0 to disable.",
            "使用拆分差异视图的最小宽度（列数）。当编辑器更窄时，差异视图自动切换为统一模式。设为 0 则禁用。",
        );
        map.insert(
            "Whether to enable word diff highlighting in the editor. When enabled, changed words within modified lines are highlighted to show exactly what changed.",
            "是否在编辑器中启用词级差异高亮。启用后，修改行中已更改的单词将被高亮显示以明确变化内容。",
        );
        map.insert(
            "Whether the editor will scroll beyond the last line.",
            "编辑器是否可以滚动超出最后一行。",
        );
        map.insert(
            "The number of lines to keep above/below the cursor when auto-scrolling.",
            "自动滚动时光标上方/下方保留的行数。",
        );
        map.insert(
            "The number of characters to keep on either side when scrolling with the mouse.",
            "使用鼠标滚动时两侧保留的字符数。",
        );
        map.insert(
            "Scroll sensitivity multiplier for both horizontal and vertical scrolling.",
            "水平和垂直滚动的灵敏度倍数。",
        );
        map.insert(
            "Whether to zoom the editor font size with the mouse wheel while holding the primary modifier key.",
            "按住主修饰键时是否可以使用鼠标滚轮缩放编辑器字号。",
        );
        map.insert(
            "Fast scroll sensitivity multiplier for both horizontal and vertical scrolling.",
            "水平和垂直快速滚动的灵敏度倍数。",
        );
        map.insert(
            "Whether to scroll when clicking near the edge of the visible text area.",
            "在可见文本区域边缘附近点击时是否自动滚动。",
        );
        map.insert("Whether to stick scopes to the top of the editor", "是否将作用域粘在编辑器顶部。");
        map.insert("Automatically show a signature help pop-up.", "自动显示签名帮助弹出框。");
        map.insert(
            "Show the signature help pop-up after completions or bracket pairs are inserted.",
            "在插入补全内容或括号对后显示签名帮助弹出框。",
        );
        map.insert(
            "Determines how snippets are sorted relative to other completion items.",
            "决定代码片段相对于其他补全项的排序方式。",
        );
        map.insert(
            "Show the informational hover box when moving the mouse over symbols in the editor.",
            "在编辑器中将鼠标悬停在符号上时显示信息弹出框。",
        );
        map.insert(
            "Time to wait in milliseconds before showing the informational hover box.",
            "显示信息悬停框前的等待时间（毫秒）。",
        );
        map.insert(
            "Time to wait in milliseconds before hiding the hover popover after the mouse moves away.",
            "鼠标离开后隐藏悬停弹出框的等待时间（毫秒）。",
        );
        map.insert(
            "Whether the hover popover sticks when the mouse moves toward it, allowing interaction with its contents.",
            "鼠标向悬停弹出框移动时是否保持显示以便交互。",
        );
        map.insert("Delay in milliseconds before drag and drop selection starts.", "拖放选择开始前的延迟时间（毫秒）。");
        map.insert("Enable drag and drop selection.", "启用拖放选择。");
        map.insert("Relative size of the drop target in the editor that will open dropped file as a split pane.", "编辑器中拖放目标的相对大小（将拖入文件以拆分窗格打开）。");
        map.insert("Enable middle-click paste on Linux.", "在 Linux 上启用中键粘贴。");
        map.insert("Controls line number display in the editor's gutter. \"disabled\" shows absolute line numbers, \"enabled\" shows relative line numbers for each absolute line, and \"wrapped\" shows relative line numbers for every line, absolute or wrapped.", "控制编辑器装订线中的行号显示方式。");
        map.insert("Minimum number of characters to reserve space for in the gutter.", "装订线中为行号预留空间的最小字符数。");
        map.insert("Show code folding controls in the gutter.", "在装订线中显示代码折叠控件。");
        map.insert("Show runnable buttons in the gutter.", "在装订线中显示可运行按钮。");
        map.insert("Show breakpoints in the gutter.", "在装订线中显示断点。");
        map.insert("Show bookmarks in the gutter.", "在装订线中显示书签。");
        map.insert("Show code action button at start of buffer line.", "在缓冲区行首显示代码操作按钮。");
        map.insert("Show line numbers in the gutter.", "在装订线中显示行号。");
        map.insert("How and when the scrollbar should be displayed.", "滚动条的显示方式和时机。");
        map.insert("Show cursor positions in the scrollbar.", "在滚动条中显示光标位置。");
        map.insert("Show Git diff indicators in the scrollbar.", "在滚动条中显示 Git 差异指示器。");
        map.insert("Show selected text occurrences in the scrollbar.", "在滚动条中显示选中文本的出现位置。");
        map.insert("Show selected symbol occurrences in the scrollbar.", "在滚动条中显示选中符号的出现位置。");
        map.insert("Show buffer search result indicators in the scrollbar.", "在滚动条中显示缓冲区搜索结果指示器。");
        map.insert("Which diagnostic indicators to show in the scrollbar.", "在滚动条中显示哪些诊断指示器。");
        map.insert("Border style for the minimap's scrollbar thumb.", "缩略图滚动条滑块的边框样式。");
        map.insert("When to show the minimap in the editor.", "何时在编辑器中显示缩略图。");
        map.insert("Where to show the minimap in the editor.", "缩略图在编辑器中的显示位置。");
        map.insert("How to highlight the current line in the minimap.", "缩略图中当前行的高亮方式。");
        map.insert("When to show the minimap thumb.", "何时显示缩略图滑块。");
        map.insert("Maximum number of columns to display in the minimap.", "缩略图中显示的最大列数。");
        map.insert("Show breadcrumbs.", "显示面包屑导航。");
        map.insert("Display the terminal title in breadcrumbs inside the terminal pane.", "在终端窗格的面包屑中显示终端标题。");
        map.insert("Show quick action buttons (e.g., search, selection, editor controls, etc.).", "显示快捷操作按钮（如搜索、选区、编辑器控件等）。");
        map.insert("Show the selections menu in the editor toolbar.", "在编辑器工具栏中显示选区菜单。");
        map.insert("Show agent review buttons in the editor toolbar.", "在编辑器工具栏中显示智能体审查按钮。");
        map.insert("Show code action buttons in the editor toolbar.", "在编辑器工具栏中显示代码操作按钮。");
        map.insert("Highlight all occurrences of selected text.", "高亮选中文本的所有出现位置。");
        map.insert("How to highlight the current line.", "当前行的高亮方式。");
        map.insert("Character counts at which to show wrap guides.", "显示换行参考线的字符数位置。");
        map.insert("Character counts at which to show wrap guides in the editor.", "在编辑器中显示换行参考线的字符数位置。");
        map.insert("Show wrap guides (vertical rulers).", "显示换行参考线（垂直标尺）。");
        map.insert("Show wrap guides in the editor.", "在编辑器中显示换行参考线。");
        map.insert("Whether to display inline and alongside documentation for items in the completions menu.", "是否在补全菜单中内联并排显示条目的文档。");
        map.insert("Whether to pop the completions menu while typing in an editor without explicitly requesting it.", "在编辑器中输入时是否自动弹出补全菜单。");
        map.insert("Whether to align detail text in code completions context menus left or right.", "代码补全上下文菜单中详情文本的对齐方式（左或右）。");
        map.insert("When to show the scrollbar in the completion menu.", "何时在补全菜单中显示滚动条。");
        map.insert("Controls how words are completed.", "控制单词补全方式。");
        map.insert("How many characters has to be in the completions query to automatically show the words-based completions.", "自动显示基于单词的补全所需的最少查询字符数。");
        map.insert("Additional code actions to run when formatting.", "格式化时运行的附加代码操作。");
        map.insert("Whether or not to perform a buffer format before saving.", "保存前是否对缓冲区进行格式化。");
        map.insert("How to perform a buffer format.", "缓冲区格式化的执行方式。");
        map.insert("Whether or not to ensure there's a single newline at the end of a buffer when saving it.", "保存缓冲区时是否确保末尾只有一个换行符。");
        map.insert("Whether or not to remove any trailing whitespace from lines of a buffer before saving it.", "保存缓冲区前是否删除各行行尾的空白字符。");
        map.insert("Whether to automatically type closing characters for you. For example, when you type '(', Zed will automatically add a closing ')' at the correct position.", "是否自动补全闭合字符。");
        map.insert("Whether to automatically surround text with characters for you. For example, when you select text and type '(', Zed will automatically surround text with ().", "是否自动为选中文本添加包围字符。");
        map.insert("Controls whether the closing characters are always skipped over and auto-removed no matter how they were inserted.", "控制是否始终跳过并自动删除闭合字符。");
        map.insert("Whether to automatically close JSX tags.", "是否自动关闭 JSX 标签。");
        map.insert("Whether to show tabs and spaces in the editor.", "是否在编辑器中显示制表符和空格。");
        map.insert("Visible character used to render space characters when show_whitespaces is enabled (default: \"•\")", "显示空白字符时用于渲染空格的可见字符（默认：\"•\"）。");
        map.insert("Visible character used to render tab characters when show_whitespaces is enabled (default: \"→\")", "显示空白字符时用于渲染制表符的可见字符（默认：\"→\"）。");
        map.insert("Global switch to toggle hints on and off.", "全局开关，用于打开/关闭内嵌提示。");
        map.insert("Show a background for inlay hints.", "为内嵌提示显示背景。");
        map.insert("Whether parameter hints should be shown.", "是否显示参数提示。");
        map.insert("Whether type hints should be shown.", "是否显示类型提示。");
        map.insert("Whether other hints should be shown.", "是否显示其他提示。");
        map.insert("Whether or not to debounce inlay hints updates after buffer edits (set to 0 to disable debouncing).", "缓冲区编辑后是否对内嵌提示更新进行防抖处理（设为 0 则禁用）。");
        map.insert("Whether or not to debounce inlay hints updates after buffer scrolls (set to 0 to disable debouncing).", "缓冲区滚动后是否对内嵌提示更新进行防抖处理（设为 0 则禁用）。");
        map.insert("Whether tasks are enabled for this language.", "是否为此语言启用任务。");
        map.insert("Extra task variables to set for a particular language.", "为特定语言设置的额外任务变量。");
        map.insert("Whether to colorize brackets in the editor.", "是否在编辑器中为括号着色。");
        map.insert("Whether and how to display code lenses from language servers.", "是否以及如何显示语言服务器的代码镜头。");
        map.insert("How to render LSP color previews in the editor.", "编辑器中 LSP 颜色预览的渲染方式。");
        map.insert("Controls automatic indentation behavior when typing.", "控制输入时的自动缩进行为。");
        map.insert("Whether indentation of pasted content should be adjusted based on the context.", "粘贴内容的缩进是否应根据上下文调整。");
        map.insert("Whether to indent lines using tab characters, as opposed to multiple spaces.", "是否使用制表符而非多个空格来缩进行。");
        map.insert("How many columns a tab should occupy.", "制表符占用的列数。");
        map.insert("The column at which to soft-wrap lines, for buffers where soft-wrap is enabled.", "启用软换行的缓冲区中软换行的列位置。");
        map.insert("How to soft-wrap long lines of text.", "长文本行的软换行方式。");
        map.insert("Custom line height value (must be at least 1.0).", "自定义行高值（必须至少为 1.0）。");
        map.insert("Whether to automatically replace emoji shortcodes with emoji characters.", "是否自动将 emoji 短码替换为 emoji 字符。");
        map.insert("Enable drag and drop selection.", "启用拖放选择。");
        map.insert("Whether to automatically open files dropped from external sources.", "是否自动打开从外部拖入的文件。");
        map.insert("Whether to start a new line with a comment when a previous line is a comment as well.", "当上一行也是注释时，是否在新行开头自动添加注释。");
        map.insert("Whether to use additional LSP queries to format (and amend) the code after every \"trigger\" symbol input, defined by LSP server capabilities", "是否使用额外的 LSP 查询在每个触发符号输入后格式化代码。");
        map.insert("Use LSP tasks over Zed language extension tasks.", "优先使用 LSP 任务而非 Zed 语言扩展任务。");
        map.insert("Controls where the `editor::rewrap` action is allowed for this language.", "控制此语言允许 `editor::rewrap` 操作的位置。");
        map.insert(
            "Whether to use language servers to provide code intelligence.",
            "是否使用语言服务器提供代码智能功能。",
        );
        map.insert(
            "The list of language servers to use (or disable) for this language.",
            "为此语言使用（或禁用）的语言服务器列表。",
        );
        map.insert(
            "Whether to perform linked edits of associated ranges, if the LS supports it. For example, when editing opening <html> tag, the contents of the closing </html> tag will be edited as well.",
            "是否对关联范围执行联动编辑（如果语言服务器支持）。",
        );
        map.insert(
            "Whether to follow-up empty Go to definition responses from the language server.",
            "是否在语言服务器返回空的跳转定义响应时执行后续操作。",
        );
        map.insert(
            "How to scroll the target into view when navigating to a definition or reference.",
            "导航到定义或引用时将目标滚动到视图中的方式。",
        );
        map.insert(
            "When enabled, use folding ranges from the language server instead of indent-based folding.",
            "启用后，使用来自语言服务器的折叠范围而非基于缩进的折叠。",
        );
        map.insert(
            "When enabled, use the language server's document symbols for outlines and breadcrumbs instead of tree-sitter.",
            "启用后，使用语言服务器的文档符号作为大纲和面包屑，而非 tree-sitter。",
        );
        map.insert(
            "When fetching LSP completions, determines how long to wait for a response of a particular server (set to 0 to wait indefinitely).",
            "获取 LSP 补全时，等待特定服务器响应的超时时间（设为 0 表示无限等待）。",
        );
        map.insert("Controls how LSP completions are inserted.", "控制 LSP 补全内容的插入方式。");
        map.insert("Whether to fetch LSP completions or not.", "是否获取 LSP 补全。");
        map.insert("Minimum time to wait before pulling diagnostics from the language server(s).", "从语言服务器拉取诊断信息前的最小等待时间。");
        map.insert("Whether to pull for language server-powered diagnostics or not.", "是否主动拉取语言服务器驱动的诊断信息。");
        map.insert("The debounce delay before querying highlights from the language.", "从语言查询高亮前的防抖延迟。");
        map.insert("Preferred debuggers for this language.", "此语言首选的调试器。");
        map.insert(
            "Enables or disables formatting with Prettier for a given language.",
            "启用或禁用 Prettier 对特定语言的格式化。",
        );
        map.insert(
            "Forces Prettier integration to use a specific parser name when formatting files with the language.",
            "强制 Prettier 集成在格式化该语言文件时使用特定解析器名称。",
        );
        map.insert(
            "Forces Prettier integration to use specific plugins when formatting files with the language.",
            "强制 Prettier 集成在格式化该语言文件时使用特定插件。",
        );
        map.insert(
            "Default Prettier options, in the format as in package.json section for Prettier.",
            "默认 Prettier 选项，格式与 package.json 中的 Prettier 部分相同。",
        );
        map.insert(
            "A mapping from languages to files and file extensions that should be treated as that language.",
            "将文件和文件扩展名映射到对应语言的规则。",
        );
        map.insert(
            "Which level to use to filter out diagnostics displayed in the editor.",
            "用于过滤编辑器中显示的诊断信息的级别。",
        );
        map.insert("Whether to show warnings or not by default.", "默认是否显示警告。");
        map.insert("The delay in milliseconds to show inline diagnostics after the last diagnostic update.", "最后一次诊断更新后显示内联诊断的延迟（毫秒）。");
        map.insert("The minimum column at which to display inline diagnostics.", "显示内联诊断的最小列号。");
        map.insert("Which files containing diagnostic errors/warnings to mark in the tabs.", "在标签中标记哪些包含诊断错误/警告的文件。");
        map.insert(
            "Globs to match files that will be considered \"hidden\" and can be hidden from the project panel.",
            "用于匹配被视为\"隐藏\"文件的 glob 模式，可从项目面板中隐藏。",
        );
        map.insert("Search for whole words by default.", "默认按整词搜索。");
        map.insert("Search case-sensitively by default.", "默认区分大小写搜索。");
        map.insert(
            "Whether to automatically enable case-sensitive search based on the search query.",
            "是否根据搜索查询自动启用区分大小写搜索。",
        );
        map.insert("Include ignored files in search results by default.", "默认在搜索结果中包含已忽略的文件。");
        map.insert("Use regex search by default.", "默认使用正则表达式搜索。");
        map.insert("Whether the editor search results will loop.", "编辑器搜索结果是否循环。");
        map.insert("Whether to center the current match in the editor", "是否在编辑器中将当前匹配项居中。");
        map.insert(
            "When to populate a new search's query based on the text under the cursor.",
            "何时根据光标处的文本填充新搜索的查询。",
        );
        map.insert("Use gitignored files when searching.", "搜索时使用 .gitignore 排除的文件。");
        map.insert("Show file icons in the file finder.", "在文件查找器中显示文件图标。");
        map.insert(
            "Determines how much space the file finder can take up in relation to the available window width.",
            "决定文件查找器相对于可用窗口宽度可占用的空间大小。",
        );
        map.insert(
            "Whether the file finder should skip focus for the active file in search results.",
            "文件查找器是否应在搜索结果中跳过活动文件的焦点。",
        );
        map.insert(
            "Files or globs of files that will be excluded by Zed entirely. They will be skipped during file scans, file searches, and not be displayed in the project file tree. Takes precedence over \"File Scan Inclusions\"",
            "Zed 完全排除的文件或 glob 模式。将在文件扫描和搜索中跳过，也不会显示在项目文件树中。优先级高于\"文件扫描包含项\"。",
        );
        map.insert(
            "Files or globs of files that will be included by Zed, even when ignored by git. This is useful for files that are not tracked by git, but are still important to your project. Note that globs that are overly broad can slow down Zed's file scanning. \"File Scan Exclusions\" takes precedence over these inclusions",
            "即使被 git 忽略也会被 Zed 包含的文件或 glob 模式。适用于未被 git 跟踪但对项目重要的文件。过于宽泛的 glob 可能会降低扫描速度。",
        );
        map.insert("Restore previous file state when reopening.", "重新打开文件时恢复之前的文件状态。");
        map.insert("Automatically close files that have been deleted.", "自动关闭已被删除的文件。");
        map.insert("Show the project panel button in the status bar.", "在状态栏中显示项目面板按钮。");
        map.insert("Show the active language button in the status bar.", "在状态栏中显示活动语言按钮。");
        map.insert("Control when to show the active encoding in the status bar.", "控制何时在状态栏中显示活动编码。");
        map.insert("Show the active line endings button in the status bar.", "在状态栏中显示活动行尾符按钮。");
        map.insert("Show the cursor position button in the status bar.", "在状态栏中显示光标位置按钮。");
        map.insert("Show the terminal button in the status bar.", "在状态栏中显示终端按钮。");
        map.insert("Show the project diagnostics button in the status bar.", "在状态栏中显示项目诊断按钮。");
        map.insert("Show the project search button in the status bar.", "在状态栏中显示项目搜索按钮。");
        map.insert("Show the debugger button in the status bar.", "在状态栏中显示调试器按钮。");
        map.insert("Show the name of the active file in the status bar.", "在状态栏中显示活动文件名。");
        map.insert("Show the collaboration panel button in the status bar.", "在状态栏中显示协作面板按钮。");
        map.insert("Show the Git panel button in the status bar.", "在状态栏中显示 Git 面板按钮。");
        map.insert("Show the outline panel button in the status bar.", "在状态栏中显示大纲面板按钮。");
        map.insert("Whether to show the agent panel button in the status bar.", "是否在状态栏中显示智能体面板按钮。");
        map.insert("Show the agent panel button in the status bar.", "在状态栏中显示智能体面板按钮。");
        map.insert("Show git status indicators on the branch icon in the titlebar.", "在标题栏的分支图标上显示 Git 状态指示器。");
        map.insert("Show the branch name button in the titlebar.", "在标题栏中显示分支名称按钮。");
        map.insert("Show the project host and name in the titlebar.", "在标题栏中显示项目主机和名称。");
        map.insert("Show banners announcing new features in the titlebar.", "在标题栏中显示宣传新功能的横幅。");
        map.insert("Show the sign in button in the titlebar.", "在标题栏中显示登录按钮。");
        map.insert("Show the user menu button in the titlebar.", "在标题栏中显示用户菜单按钮。");
        map.insert("Show user picture in the titlebar.", "在标题栏中显示用户头像。");
        map.insert("Show the menus in the titlebar.", "在标题栏中显示菜单。");
        map.insert(
            "(Linux only) choose how window control buttons are laid out in the titlebar.",
            "（仅限 Linux）选择标题栏中窗口控制按钮的布局方式。",
        );
        map.insert("(Linux only) whether Zed or your compositor should draw window decorations.", "（仅限 Linux）是否由 Zed 或合成器绘制窗口装饰。");
        map.insert("(macOS only) whether to allow Windows to tab together.", "（仅限 macOS）是否允许窗口合并为标签组。");
        map.insert("Show the tab bar in the editor.", "在编辑器中显示标签栏。");
        map.insert("Show the Git file status on a tab item.", "在标签条目上显示 Git 文件状态。");
        map.insert("Show the file icon for a tab.", "显示标签的文件图标。");
        map.insert("Position of the close button in a tab.", "标签中关闭按钮的位置。");
        map.insert("Maximum open tabs in a pane. Will not close an unsaved tab.", "窗格中最多打开的标签数，不会关闭未保存的标签。");
        map.insert("Show the navigation history buttons in the tab bar.", "在标签栏中显示导航历史按钮。");
        map.insert("Show the tab bar buttons (New, Split Pane, Zoom).", "显示标签栏按钮（新建、拆分窗格、缩放）。");
        map.insert("Show opened editors as preview tabs.", "将打开的编辑器显示为预览标签。");
        map.insert("Whether to open tabs in preview mode when selected from the file finder.", "从文件查找器选中时是否以预览模式打开标签。");
        map.insert("Whether to open tabs in preview mode when opened from a multibuffer.", "从多缓冲区打开时是否以预览模式打开标签。");
        map.insert("Whether to open tabs in preview mode when opened from the project panel with a single click.", "在项目面板中单击打开时是否以预览模式打开标签。");
        map.insert("Whether to open tabs in preview mode when code navigation is used to open a single file.", "使用代码导航打开单个文件时是否以预览模式打开标签。");
        map.insert("Whether to open tabs in preview mode when code navigation is used to open a multibuffer.", "使用代码导航打开多缓冲区时是否以预览模式打开标签。");
        map.insert("Whether to keep tabs in preview mode when code navigation is used to navigate away from them. If `enable_preview_file_from_code_navigation` or `enable_preview_multibuffer_from_code_navigation` is also true, the new tab may replace the existing one.", "使用代码导航跳转时是否保持标签的预览模式。");
        map.insert("Show pinned tabs in a separate row above unpinned tabs.", "在未固定标签上方单独一行显示固定标签。");
        map.insert("Layout mode for the bottom dock.", "底部停靠栏的布局模式。");
        map.insert("Direction to split horizontally.", "水平拆分方向。");
        map.insert("Direction to split vertically.", "垂直拆分方向。");
        map.insert("Left padding for centered layout.", "居中布局的左侧填充。");
        map.insert("Right padding for centered layout.", "居中布局的右侧填充。");
        map.insert("Maximum content width in pixels. Content will be centered when the panel is wider than this value.", "内容的最大宽度（像素）。当面板宽于此值时内容将居中。");
        map.insert("Whether to constrain the agent panel content to a maximum width, centering it when the panel is wider, for optimal readability.", "是否将智能体面板内容限制在最大宽度内，宽于时居中显示以提升可读性。");
        map.insert("What to do after closing the current tab.", "关闭当前标签后的行为。");
        map.insert("Where to dock the project panel.", "项目面板的停靠位置。");
        map.insert("Default width of the project panel in pixels.", "项目面板的默认宽度（像素）。");
        map.insert("Whether to hide the gitignore entries in the project panel.", "是否在项目面板中隐藏 .gitignore 中的条目。");
        map.insert("Spacing between worktree entries in the project panel.", "项目面板中工作树条目间的间距。");
        map.insert("Whether to show folder icons or chevrons for directories in the project panel.", "是否在项目面板中显示目录的文件夹图标或箭头。");
        map.insert("Show the Git status in the project panel.", "在项目面板中显示 Git 状态。");
        map.insert("Amount of indentation for nested items.", "嵌套条目的缩进量。");
        map.insert("Whether to reveal entries in the project panel automatically when a corresponding project entry becomes active.", "当相应项目条目变为活动状态时，是否自动在项目面板中展示。");
        map.insert("Whether the project panel should open on startup.", "项目面板是否应在启动时打开。");
        map.insert("Whether to fold directories automatically and show compact folders when a directory has only one subdirectory inside.", "是否自动折叠目录，并在目录仅含一个子目录时显示紧凑文件夹。");
        map.insert("Whether to fold directories automatically when a directory contains only one subdirectory.", "当目录只含一个子目录时是否自动折叠。");
        map.insert("Whether to show folder names with bold text in the project panel.", "是否在项目面板中以粗体显示文件夹名称。");
        map.insert("Show the scrollbar in the project panel.", "在项目面板中显示滚动条。");
        map.insert("Whether to allow horizontal scrolling in the project panel. When disabled, the view is always locked to the leftmost position and long file names are clipped.", "是否允许在项目面板中水平滚动。禁用时视图始终锁定在最左侧，长文件名将被截断。");
        map.insert("Which files containing diagnostic errors/warnings to mark in the project panel.", "在项目面板中标记哪些包含诊断错误/警告的文件。");
        map.insert("Show error and warning count badges next to file names in the project panel.", "在项目面板的文件名旁显示错误和警告数量徽章。");
        map.insert("Show a git status indicator next to file names in the project panel.", "在项目面板文件名旁显示 Git 状态指示器。");
        map.insert("Show indent guides in the project panel.", "在项目面板中显示缩进参考线。");
        map.insert("Whether to enable drag-and-drop operations in the project panel.", "是否在项目面板中启用拖放操作。");
        map.insert("Whether to hide the root entry when only one folder is open in the window.", "当窗口中只打开一个文件夹时是否隐藏根条目。");
        map.insert("Whether to hide the hidden entries in the project panel.", "是否在项目面板中隐藏隐藏条目。");
        map.insert("Sort order for entries in the project panel.", "项目面板中条目的排序顺序。");
        map.insert("Whether to sort file and folder names case-sensitively in the project panel.", "是否在项目面板中区分大小写地排序文件和文件夹名称。");
        map.insert("Enable to sort entries in the panel by path, disable to sort by status.", "启用则按路径排序面板条目，禁用则按状态排序。");
        map.insert("Whether to automatically open newly created files in the editor.", "是否自动在编辑器中打开新建的文件。");
        map.insert("Whether to automatically open files after pasting or duplicating them.", "是否在粘贴或复制文件后自动打开。");
        map.insert("Whether to automatically open files dropped from external sources.", "是否自动打开从外部拖入的文件。");
        map.insert("Enable to show entries in tree view list, disable to show in flat view list.", "启用以树形视图显示条目，禁用则以平铺视图显示。");
        map.insert("Whether to stick parent directories at top of the project panel.", "是否将父目录固定在项目面板顶部。");
        map.insert("Show file icons in the project panel.", "在项目面板中显示文件图标。");
        map.insert("Where to dock the collaboration panel.", "协作面板的停靠位置。");
        map.insert("Default width of the collaboration panel in pixels.", "协作面板的默认宽度（像素）。");
        map.insert("Where to dock the Git panel.", "Git 面板的停靠位置。");
        map.insert("Default width of the Git panel in pixels.", "Git 面板的默认宽度（像素）。");
        map.insert("How entry statuses are displayed.", "条目状态的显示方式。");
        map.insert("Whether to collapse untracked files in the diff panel.", "是否在差异面板中折叠未跟踪文件。");
        map.insert("Maximum length of the commit message title before a warning is shown. Set to 0 to disable.", "显示警告前提交消息标题的最大长度。设为 0 则禁用。");
        map.insert("Show the addition/deletion change count next to each file in the Git panel.", "在 Git 面板中每个文件旁显示增删行数统计。");
        map.insert("Whether to show a badge on the git panel icon with the count of uncommitted changes.", "是否在 Git 面板图标上显示未提交更改数量徽章。");
        map.insert("Show a badge on the terminal panel icon with the count of open terminals.", "在终端面板图标上显示已打开终端数量徽章。");
        map.insert("Show file icons in the outline panel.", "在大纲面板中显示文件图标。");
        map.insert("Show the Git status in the outline panel.", "在大纲面板中显示 Git 状态。");
        map.insert("Whether to show folder icons or chevrons for directories in the outline panel.", "是否在大纲面板中显示目录的文件夹图标或箭头。");
        map.insert("Whether to reveal when a corresponding outline entry becomes active.", "当相应大纲条目变为活动状态时是否自动展示。");
        map.insert("Where to dock the agent panel.", "智能体面板的停靠位置。");
        map.insert("Default width when the agent panel is docked to the left or right.", "智能体面板停靠在左侧或右侧时的默认宽度。");
        map.insert("Default height when the agent panel is docked to the bottom.", "智能体面板停靠在底部时的默认高度。");
        map.insert("Whether the agent panel should use flexible (proportional) sizing when docked to the left or right.", "智能体面板停靠在左侧或右侧时是否使用弹性（比例）大小。");
        map.insert("Where to dock the terminal panel.", "终端面板的停靠位置。");
        map.insert("Whether the terminal panel should use flexible (proportional) sizing when docked to the left or right.", "终端面板停靠在左侧或右侧时是否使用弹性（比例）大小。");
        map.insert("Determines the stepping granularity for debug operations.", "决定调试操作的单步粒度。");
        map.insert("Whether breakpoints should be reused across Zed sessions.", "断点是否应在 Zed 会话之间保留。");
        map.insert("Time in milliseconds until timeout error when connecting to a TCP debug adapter.", "连接 TCP 调试适配器时超时的毫秒数。");
        map.insert("Whether to log messages between active debug adapters and Zed.", "是否记录活动调试适配器与 Zed 之间的消息。");
        map.insert("Whether to format DAP messages when adding them to debug adapter logger.", "是否在将 DAP 消息添加到调试适配器日志时对其格式化。");
        map.insert("What shell to use when opening a terminal.", "打开终端时使用的 shell。");
        map.insert("What working directory to use when launching the terminal.", "启动终端时使用的工作目录。");
        map.insert("Key-value pairs to add to the terminal's environment.", "添加到终端环境的键值对。");
        map.insert("Activates the Python virtual environment, if one is found, in the terminal's working directory.", "在终端工作目录中检测到 Python 虚拟环境时自动激活。");
        map.insert("Font size for terminal text. If not set, defaults to buffer font size.", "终端文本的字号。未设置时默认使用缓冲区字号。");
        map.insert("Font family for terminal text. If not set, defaults to buffer font family.", "终端文本的字体族。未设置时默认使用缓冲区字体族。");
        map.insert("Font fallbacks for terminal text. If not set, defaults to buffer font fallbacks.", "终端文本的字体回退。未设置时默认使用缓冲区字体回退。");
        map.insert("Font weight for terminal text in CSS weight units (100-900).", "终端文本的字重（CSS 单位，100-900）。");
        map.insert("Font features for terminal text.", "终端文本的字体特性。");
        map.insert("Line height for terminal text.", "终端文本的行高。");
        map.insert("Default cursor shape for the terminal (bar, block, underline, or hollow).", "终端的默认光标形状（竖线、方块、下划线或空心）。");
        map.insert("Sets the cursor blinking behavior in the terminal.", "设置终端中光标的闪烁行为。");
        map.insert("Whether alternate scroll mode is active by default (converts mouse scroll to arrow keys in apps like Vim).", "是否默认启用备用滚动模式（在 Vim 等应用中将鼠标滚动转换为方向键）。");
        map.insert("The minimum APCA perceptual contrast between foreground and background colors (0-106).", "前景色与背景色之间的最小 APCA 感知对比度（0-106）。");
        map.insert("The minimum APCA perceptual contrast to maintain when rendering text over highlight backgrounds.", "在高亮背景上渲染文本时要保持的最小 APCA 感知对比度。");
        map.insert("Whether the option key behaves as the meta key.", "Option 键是否作为 Meta 键使用。");
        map.insert("Whether selecting text in the terminal automatically copies to the system clipboard.", "在终端中选中文本是否自动复制到系统剪贴板。");
        map.insert("Whether to keep the text selection after copying it to the clipboard.", "复制到剪贴板后是否保留文本选区。");
        map.insert("Whether to play a sound when the BEL character (\\a, 0x07) is printed", "打印 BEL 字符（\\a, 0x07）时是否播放声音。");
        map.insert("Default width when the terminal is docked to the left or right (in pixels).", "终端停靠在左侧或右侧时的默认宽度（像素）。");
        map.insert("Default height when the terminal is docked to the bottom (in pixels).", "终端停靠在底部时的默认高度（像素）。");
        map.insert("Maximum number of lines to keep in scrollback history (max: 100,000; 0 disables scrolling).", "滚动回溯历史保留的最大行数（最大：100,000；0 表示禁用滚动）。");
        map.insert("The multiplier for scrolling in the terminal with the mouse wheel", "使用鼠标滚轮在终端中滚动的倍数。");
        map.insert("The multiplier for scrolling in the terminal with the mouse wheel.", "使用鼠标滚轮在终端中滚动的倍数。");
        map.insert("An optional string to override the title of the terminal tab.", "可选字符串，用于覆盖终端标签的标题。");
        map.insert("The shell program to use.", "要使用的 Shell 程序。");
        map.insert("The shell program to run.", "要运行的 Shell 程序。");
        map.insert("The arguments to pass to the shell program.", "传递给 Shell 程序的参数。");
        map.insert("The directory path to use (will be shell expanded).", "要使用的目录路径（将进行 shell 展开）。");
        map.insert("Disable all Git integration features in Zed.", "禁用 Zed 中的所有 Git 集成功能。");
        map.insert("Show Git status information in the editor.", "在编辑器中显示 Git 状态信息。");
        map.insert("Show Git diff information in the editor.", "在编辑器中显示 Git 差异信息。");
        map.insert("Control whether Git status is shown in the editor's gutter.", "控制是否在编辑器装订线中显示 Git 状态。");
        map.insert("Debounce threshold in milliseconds after which changes are reflected in the Git gutter.", "更改反映到 Git 装订线的防抖阈值（毫秒）。");
        map.insert("The delay after which the inline blame information is shown.", "内联追责信息显示前的延迟。");
        map.insert("Padding between the end of the source line and the start of the inline blame in columns.", "源码行末与内联追责起始处之间的填充列数。");
        map.insert("The minimum column number at which to show the inline blame information.", "显示内联追责信息的最小列号。");
        map.insert("Show commit summary as part of the inline blame.", "将提交摘要作为内联追责的一部分显示。");
        map.insert("Whether or not to show Git blame data inline in the currently focused line.", "是否在当前聚焦行内联显示 Git 追责数据。");
        map.insert("Show the avatar of the author of the commit.", "显示提交作者的头像。");
        map.insert("Show author name as part of the commit information in branch picker.", "在分支选择器的提交信息中显示作者名称。");
        map.insert("How Git hunks are displayed visually in the editor.", "Git 变更块在编辑器中的视觉显示方式。");
        map.insert("Should the name or path be displayed first in the git view.", "Git 视图中优先显示文件名还是路径。");
        map.insert("Default branch name will be when init.defaultbranch is not set in Git.", "未在 Git 中设置 init.defaultBranch 时使用的默认分支名称。");
        map.insert("Whether to show folder icons or chevrons for directories in the git panel.", "是否在 Git 面板中显示目录的文件夹图标或箭头。");
        map.insert("Show file icons next to the Git status icon.", "在 Git 状态图标旁显示文件图标。");
        map.insert("Whether the microphone should be muted when joining a channel or a call.", "加入频道或通话时麦克风是否静音。");
        map.insert("Whether your current project should be shared when joining an empty channel.", "加入空频道时是否分享当前项目。");
        map.insert("Test your microphone and speaker setup", "测试您的麦克风和扬声器设置。");
        map.insert("Select output audio device", "选择音频输出设备。");
        map.insert("Select input audio device", "选择音频输入设备。");
        map.insert("Whether to disable all AI features in Zed.", "是否禁用 Zed 中的所有 AI 功能。");
        map.insert("Which side of the window the threads sidebar appears on.", "线程侧边栏出现在窗口的哪一侧。");
        map.insert("Set up regex patterns to auto-allow, auto-deny, or always request confirmation, for specific tool inputs.", "为特定工具输入设置正则模式，以自动允许、自动拒绝或始终请求确认。");
        map.insert("Whether to start a new thread in the current local project or in a new Git worktree.", "是否在当前本地项目或新的 Git 工作树中开始新线程。");
        map.insert("When enabled, agent edits will also be displayed in single-file buffers for review.", "启用后，智能体编辑也将在单文件缓冲区中显示以供审查。");
        map.insert("Show voting thumbs up/down icon buttons for feedback on agent edits.", "显示对智能体编辑进行反馈的点赞/踩图标按钮。");
        map.insert("Where to show notifications when the agent has completed its response or needs confirmation before running a tool action.", "当智能体完成响应或在执行工具操作前需要确认时，显示通知的位置。");
        map.insert("When to play a sound when the agent has either completed its response, or needs user input.", "当智能体完成响应或需要用户输入时，何时播放声音。");
        map.insert("Whether to have edit cards in the agent panel expanded, showing a Preview of the diff.", "是否在智能体面板中展开编辑卡片，显示差异预览。");
        map.insert("Whether to have terminal cards in the agent panel expanded, showing the whole command output.", "是否在智能体面板中展开终端卡片，显示完整命令输出。");
        map.insert("How 'thinking blocks' should be displayed by default. 'Auto' fully expands during streaming, then auto-collapses when done. 'Preview' auto-expands with a height constraint during streaming. 'Always Expanded' shows full content. 'Always Collapsed' keeps them collapsed.", "\u{601d}\u{8003}\u{5757}\u{201d}\u{7684}\u{9ed8}\u{8ba4}\u{663e}\u{793a}\u{65b9}\u{5f0f}\u{3002}\u{81ea}\u{52a8}\u{ff1a}\u{6d41}\u{5f0f}\u{4f20}\u{8f93}\u{65f6}\u{5c55}\u{5f00}\u{ff0c}\u{5b8c}\u{6210}\u{540e}\u{81ea}\u{52a8}\u{6298}\u{53e0}\u{3002}\u{9884}\u{89c8}\u{ff1a}\u{5e26}\u{9ad8}\u{5ea6}\u{9650}\u{5236}\u{81ea}\u{52a8}\u{5c55}\u{5f00}\u{3002}\u{59cb}\u{7ec8}\u{5c55}\u{5f00}\u{ff1a}\u{663e}\u{793a}\u{5168}\u{90e8}\u{5185}\u{5bb9}\u{3002}\u{59cb}\u{7ec8}\u{6298}\u{53e0}\u{ff1a}\u{4fdd}\u{6301}\u{6298}\u{53e0}\u{3002}");
        map.insert("Whether clicking the stop button on a running terminal tool should also cancel the agent's generation. Note that this only applies to the stop button, not to ctrl+c inside the terminal.", "\u{70b9}\u{51fb}\u{8fd0}\u{884c}\u{4e2d}\u{7ec8}\u{7aef}\u{5de5}\u{5177}\u{7684}\u{505c}\u{6b62}\u{6309}\u{9215}\u{662f}\u{5426}\u{4e5f}\u{5e94}\u{53d6}\u{6d88}\u{4ee3}\u{7406}\u{7684}\u{751f}\u{6210}\u{3002}\u{6ce8}\u{610f}\u{ff0c}\u{8fd9}\u{4ec5}\u{9002}\u{7528}\u{4e8e}\u{505c}\u{6b62}\u{6309}\u{9215}\u{ff0c}\u{800c}\u{975e}\u{7ec8}\u{7aef}\u{5185}\u{7684} Ctrl+C\u{3002}");
        map.insert("Whether to always use cmd-enter (or ctrl-enter on Linux or Windows) to send messages.", "\u{662f}\u{5426}\u{59cb}\u{7ec8}\u{4f7f}\u{7528} Cmd+Enter\u{ff08}Linux/Windows \u{4e0a}\u{4e3a} Ctrl+Enter\u{ff09}\u{53d1}\u{9001}\u{6d88}\u{606f}\u{3002}");
        map.insert("Minimum number of lines to display in the agent message editor.", "智能体消息编辑器中显示的最小行数。");
        map.insert("Whether to show turn statistics like elapsed time during generation and final turn duration.", "是否显示轮次统计信息，如生成期间的耗时和最终轮次时长。");
        map.insert("Whether to show the merge conflict indicator in the status bar that offers to resolve conflicts using the agent.", "是否在状态栏中显示合并冲突指示器，提供使用智能体解决冲突的选项。");
        map.insert("Default timeout in seconds for context server tool calls. Can be overridden per-server in context_servers configuration.", "上下文服务器工具调用的默认超时时间（秒），可在 context_servers 配置中按服务器覆盖。");
        map.insert("Set up different edit prediction providers in complement to Zed's built-in Zeta model.", "设置不同的编辑预测提供商，以补充 Zed 内置的 Zeta 模型。");
        map.insert("Controls whether Zed may collect training data when using Zed's Edit Predictions. Data is only collected for files in projects detected as open source. The default value uses the preference previously set via the status-bar toggle, or false if no preference has been stored.", "控制 Zed 是否可在使用编辑预测时收集训练数据。仅对检测为开源项目的文件收集数据。");
        map.insert("Controls whether edit predictions are shown immediately or manually.", "控制编辑预测是立即显示还是手动显示。");
        map.insert("Controls whether edit predictions are shown in the given language scopes.", "控制编辑预测是否在指定语言范围内显示。");
        map.insert("When to show edit predictions previews in buffer. The eager mode displays them inline, while the subtle mode displays them only when holding a modifier key.", "何时在缓冲区中显示编辑预测预览。急切模式内联显示，微妙模式仅在按住修饰键时显示。");
        map.insert("The proxy to use for network requests.", "用于网络请求的代理。");
        map.insert("The URL of the Zed server to connect to.", "要连接的 Zed 服务器 URL。");
        map.insert("Collect timing data for foreground and background executor tasks so they can be inspected via `zed: open performance profiler`. May lead to increased memory usage.", "收集前台和后台任务的计时数据以供性能分析器检查。可能导致内存占用增加。");
        map.insert("The dock position of the debug panel.", "调试面板的停靠位置。");
        map.insert("The width of the active indent guide in pixels, between 1 and 10.", "活动缩进参考线的宽度（像素，1-10）。");
        map.insert("The width of the indent guides in pixels, between 1 and 10.", "缩进参考线的宽度（像素，1-10）。");
        map.insert("Determines how indent guide backgrounds are colored.", "决定缩进参考线背景的着色方式。");
        map.insert("Determines how indent guides are colored.", "决定缩进参考线的着色方式。");
        map.insert("Display indent guides in the editor.", "在编辑器中显示缩进参考线。");
        map.insert("Save after inactivity period (in milliseconds).", "不活动一段时间后自动保存（毫秒）。");
        map.insert("The unit for image file sizes.", "图片文件大小的单位。");
        map.insert("The debounce delay before querying highlights from the language.", "从语言查询高亮前的防抖延迟。");

        // ════════════════════════════════════════════════════════════════════
        // 七、扩展页面
        // ════════════════════════════════════════════════════════════════════
        map.insert("Install Dev Extension", "安装开发扩展");
        map.insert("Search extensions...", "搜索扩展\u{2026}");
        map.insert("All", "全部");
        map.insert("Installed", "已安装");
        map.insert("Not Installed", "未安装");
        map.insert("Themes", "主题");
        map.insert("Icon Themes", "图标主题");
        map.insert("Grammars", "语法");
        map.insert("Language Servers", "语言服务器");
        map.insert("MCP Servers", "MCP 服务器");
        map.insert("Agent Servers", "智能体服务器");
        map.insert("Snippets", "代码片段");
        map.insert("Debug Adapters", "调试适配器");
        map.insert("Slash Commands", "斜杠命令");
        map.insert("Indexed Docs Providers", "索引文档提供商");
        map.insert("Overridden by dev extension.", "已被开发扩展覆盖。");
        map.insert("Incompatible", "不兼容");
        map.insert("Select extension version...", "选择扩展版本...");
        map.insert("Rebuild", "重新构建");
        map.insert("Uninstall", "卸载");
        map.insert("View Registry", "查看注册表");
        map.insert("Learn More", "了解更多");
        map.insert("View Documentation", "查看文档");
        map.insert("Enable Vim mode", "启用 Vim 模式");
        map.insert("Copy Extension ID", "复制扩展 ID");
        map.insert("Copy Author Info", "复制作者信息");
        map.insert("Do you want to install the recommended '{}' extension for '{}' files?", "是否要为 '{}' 文件安装推荐的 '{}' 扩展？");
        map.insert("Yes, install extension", "是，安装扩展");
        map.insert("No, don't install it", "否，不安装");

        // ════════════════════════════════════════════════════════════════════
        // 八、Git 相关 UI
        // ════════════════════════════════════════════════════════════════════
        map.insert("Switch branch\u{2026}", "切换分支\u{2026}");
        map.insert("Current Branch", "当前分支");
        map.insert("Create Branch", "新建分支");
        map.insert("Delete Branch", "删除分支");
        map.insert("Force Delete Branch", "强制删除分支");
        map.insert("No commits found", "未找到提交");
        map.insert("Switch", "切换");
        map.insert("Create", "创建");
        map.insert("Confirm", "确认");
        map.insert("Copy Commit SHA", "复制提交 SHA");
        map.insert("Open Permalink", "打开永久链接");
        map.insert("Copy SHA", "复制 SHA");
        map.insert("Use Both", "同时使用");
        map.insert("Resolve with Agent", "使用智能体解决");
        map.insert("Stash Pop", "弹出暂存");
        map.insert("View Stash", "查看暂存");
        map.insert("Open Diff", "打开差异");
        map.insert("View History", "查看历史");
        map.insert("You may need to configure git for Github.", "您可能需要为 GitHub 配置 Git。");
        map.insert("Learn more", "了解更多");
        map.insert("Filter Remote Branches", "过滤远程分支");
        map.insert("Show All Branches", "显示所有分支");
        map.insert("Force Delete", "强制删除");
        map.insert("No Changes", "无更改");
        map.insert("No changes to commit", "没有可提交的更改");
        map.insert("View Branch Diff", "查看分支差异");
        map.insert("Enter commit message", "输入提交消息");
        map.insert("Fetch", "拉取");
        map.insert("Stage All", "暂存全部");
        map.insert("Commit Tracked", "提交已跟踪");
        map.insert("Untracked", "未跟踪");
        map.insert("Today", "今天");

        // ════════════════════════════════════════════════════════════════════
        // 九、项目面板右键菜单
        // ════════════════════════════════════════════════════════════════════
        map.insert("Search Inside", "在内部搜索");
        map.insert("New File", "新建文件");
        map.insert("New Folder", "新建文件夹");
        map.insert("Open in Default App", "在默认应用中打开");
        map.insert("Open in Terminal", "在终端中打开");
        map.insert("Find in Folder\u{2026}", "在文件夹中查找\u{2026}");
        map.insert("Unfold Directory", "展开目录");
        map.insert("Fold Directory", "折叠目录");
        map.insert("Compare Marked Files", "比较标记文件");
        map.insert("Duplicate", "复制");
        map.insert("Download...", "下载...");
        map.insert("Copy Path", "复制路径");
        map.insert("Copy Relative Path", "复制相对路径");
        map.insert("Restore File", "恢复文件");
        map.insert("Add to .gitignore", "添加到 .gitignore");
        map.insert("Trash", "移入回收站");
        map.insert("Add Folders to Project\u{2026}", "将文件夹添加到项目\u{2026}");
        map.insert("Remove from Project", "从项目中移除");
        map.insert("Collapse All", "全部折叠");

        // ════════════════════════════════════════════════════════════════════
        // 十、智能体面板（Agent UI）
        // ════════════════════════════════════════════════════════════════════
        map.insert("Message the Zed Agent — @ to include context", "向 Zed 智能体发送消息 — 使用 @ 引入上下文");
        map.insert("Message {name} — @ to include context", "向 {name} 发送消息 — 使用 @ 引入上下文");
        map.insert("Message {name} — @ to include context, / for commands", "向 {name} 发送消息 — 使用 @ 引入上下文，/ 查看命令");
        map.insert("Search all threads...", "搜索所有线程\u{2026}");
        map.insert("New Thread\u{2026}", "新建线程\u{2026}");
        map.insert("Loading\u{2026}", "加载中\u{2026}");
        map.insert("Send Now", "立即发送");
        map.insert("Generating Changes\u{2026}", "正在生成更改\u{2026}");
        map.insert("Add LLM Provider", "添加 LLM 提供商");
        map.insert("Provider Name", "提供商名称");
        map.insert("API URL", "API 地址");
        map.insert("API Key", "API 密钥");
        map.insert("Models", "模型");
        map.insert("Add Model", "添加模型");
        map.insert("Remove Model", "删除模型");
        map.insert("Save Provider", "保存提供商");
        map.insert("Select a model\u{2026}", "选择模型\u{2026}");
        map.insert("Search agents...", "搜索智能体...");
        map.insert("Search profiles\u{2026}", "搜索配置文件\u{2026}");
        map.insert("Configure", "配置");
        map.insert("Custom Profiles", "自定义配置文件");
        map.insert("Add New Profile", "添加新配置文件");
        map.insert("Fork Profile", "复刻配置文件");
        map.insert("Configure Default Model", "配置默认模型");
        map.insert("Configure Built-in Tools", "配置内置工具");
        map.insert("Configure MCP Tools", "配置 MCP 工具");
        map.insert("Delete Profile", "删除配置文件");
        map.insert("Go Back", "返回");
        map.insert("Search built-in tools\u{2026}", "搜索内置工具\u{2026}");
        map.insert("Profile name", "配置文件名称");
        map.insert("Open Repository", "打开代码仓库");
        map.insert("Authenticate", "认证");
        map.insert("Start New Thread", "开始新线程");
        map.insert("Remove Provider", "移除提供商");
        map.insert("Add Provider", "添加提供商");
        map.insert("Compatible APIs", "兼容的 API");
        map.insert("LLM Providers", "LLM 提供商");
        map.insert("Add at least one provider to use AI-powered features with Zed's native agent.", "至少添加一个提供商以使用 Zed 原生智能体的 AI 功能。");
        map.insert("Add Server", "添加服务器");
        map.insert("Add Custom Server", "添加自定义服务器");
        map.insert("Install from Extensions", "从扩展安装");
        map.insert("Model Context Protocol (MCP) Servers", "模型上下文协议 (MCP) 服务器");
        map.insert("All MCP servers connected directly or via a Zed extension.", "所有直接连接或通过 Zed 扩展连接的 MCP 服务器。");
        map.insert("No MCP servers added yet.", "尚未添加 MCP 服务器。");
        map.insert("Configure MCP Server", "配置 MCP 服务器");
        map.insert("Configure Server", "配置服务器");
        map.insert("View Tools", "查看工具");
        map.insert("Log Out", "登出");
        map.insert("Authenticate to connect this server", "认证以连接此服务器");
        map.insert("Authenticating…", "认证中…");
        map.insert("External Agents", "外部智能体");
        map.insert("All agents connected through the Agent Client Protocol.", "通过 Agent Client Protocol 连接的所有智能体。");
        map.insert("Add Agent", "添加智能体");
        map.insert("Install from Registry", "从注册表安装");
        map.insert("Add Custom Agent", "添加自定义智能体");
        map.insert("ACP Docs", "ACP 文档");
        map.insert("Restart Agent Connection", "重新连接智能体");
        map.insert("Uninstall Agent Extension", "卸载智能体扩展");
        map.insert("Remove Registry Agent", "移除注册表智能体");
        map.insert("Remove Custom Agent", "移除自定义智能体");

        // ── 工具权限设置 ──────────────────────────────────────────────────
        map.insert("Commands executed in the terminal", "在终端中执行的命令");
        map.insert("Edit File", "编辑文件");
        map.insert("File editing operations", "文件编辑操作");
        map.insert("Write File", "写入文件");
        map.insert("File creation and overwrite operations", "文件创建与覆盖操作");
        map.insert("Delete Path", "删除路径");
        map.insert("File and directory deletion", "文件和目录删除");
        map.insert("File and directory copying", "文件和目录复制");
        map.insert("Move Path", "移动路径");
        map.insert("File and directory moves/renames", "文件和目录移动/重命名");
        map.insert("Create Directory", "创建目录");
        map.insert("Directory creation", "目录创建");
        map.insert("HTTP requests to URLs", "对 URL 的 HTTP 请求");
        map.insert("Web Search", "网页搜索");
        map.insert("Web search queries", "网页搜索查询");
        map.insert("Patterns are matched against each command in the input. Commands chained with &&, ||, ;, or pipes are split and checked individually.", "模式将匹配输入中的每条命令。使用 &&、||、; 或管道链接的命令会被拆分并逐一检查。");
        map.insert("Patterns are matched against the file path being edited.", "模式将匹配正在编辑的文件路径。");
        map.insert("Patterns are matched against the file path being written.", "模式将匹配正在写入的文件路径。");
        map.insert("Patterns are matched against the path being deleted.", "模式将匹配正在删除的路径。");
        map.insert("Patterns are matched independently against the source path and the destination path. Enter either path below to test.", "模式将分别匹配源路径和目标路径。在下面输入任一路径进行测试。");
        map.insert("Patterns are matched against the directory path being created.", "模式将匹配正在创建的目录路径。");
        map.insert("Patterns are matched against the URL being fetched.", "模式将匹配正在请求的 URL。");
        map.insert("Patterns are matched against the search query.", "模式将匹配搜索查询。");
        map.insert("Terminal Tool", "终端工具");
        map.insert("Edit File Tool", "编辑文件工具");
        map.insert("Write File Tool", "写入文件工具");
        map.insert("Delete Path Tool", "删除路径工具");
        map.insert("Copy Path Tool", "复制路径工具");
        map.insert("Move Path Tool", "移动路径工具");
        map.insert("Create Directory Tool", "创建目录工具");
        map.insert("Fetch Tool", "网络请求工具");
        map.insert("Web Search Tool", "网页搜索工具");
        map.insert("Always Deny", "始终拒绝");
        map.insert("Always Allow", "始终允许");
        map.insert("Always Confirm", "始终确认");
        map.insert("If any of these regexes match, the tool action will be denied.", "如果这些正则表达式中有任意一个匹配，工具操作将被拒绝。");
        map.insert("If any of these regexes match, the action will be approved—unless an Always Confirm or Always Deny matches.", "如果这些正则表达式中有任意一个匹配，操作将被批准——除非有\u{201C}始终确认\u{201D}或\u{201C}始终拒绝\u{201D}的规则同时匹配。");
        map.insert("If any of these regexes match, a confirmation will be shown unless an Always Deny regex matches.", "如果这些正则表达式中有任意一个匹配，将显示确认提示，除非有\u{201C}始终拒绝\u{201D}的规则同时匹配。");
        map.insert("Enter a tool input to test your rules…", "输入工具输入以测试您的规则…");
        map.insert("Test Your Rules", "测试您的规则");
        map.insert("No regex matches, using the default action.", "无正则匹配，使用默认操作。");
        map.insert("Pattern preview differs from engine — showing authoritative result.", "模式预览与引擎不同 — 显示权威结果。");
        map.insert("Result:", "结果：");
        map.insert("Allow", "允许");
        map.insert("Deny", "拒绝");
        map.insert("Invalid Patterns", "无效模式");
        map.insert("These patterns failed to compile as regular expressions. The tool will be blocked until they are fixed or removed.", "这些模式无法编译为正则表达式。该工具将被阻止，直到这些问题得到修复或移除。");
        map.insert("Delete Invalid Pattern", "删除无效模式");
        map.insert("No patterns configured", "未配置模式");
        map.insert("Delete Pattern", "删除模式");
        map.insert("Add regex pattern…", "添加正则模式…");
        map.insert("A pattern with that name already exists in this rule list.", "该规则列表中已存在同名的模式。");
        map.insert("Default Permission", "默认权限");
        map.insert("Controls the default behavior for all tool actions. Per-tool rules and patterns can override this.", "控制所有工具操作的默认行为。每个工具的规则和模式可以覆盖此设置。");
        map.insert("Default Action", "默认操作");
        map.insert("Action to take when no patterns match.", "无模式匹配时采取的操作。");
        map.insert("rule", "条规则");
        map.insert("rules", "条规则");
        map.insert("invalid", "无效");
        map.insert("Tool", "工具");
        map.insert("tool", "个工具");
        map.insert("tools", "个工具");
        map.insert("Denied: ", "已拒绝：");
        map.insert("Reason: ", "原因：");
        map.insert("Error: ", "错误：");
        map.insert("Invalid regex: ", "无效的正则表达式：");
        map.insert(". Pattern saved but will block this tool until fixed or removed.", "。模式已保存，但在修复或移除之前将阻止此工具。");

        // ── 其他遗漏翻译 ──────────────────────────────────────────────────
        map.insert("Zed Agent", "Zed 智能体");
        map.insert("Add More Agents", "添加更多智能体");
        map.insert("Toggle GPUI Inspector", "切换 GPUI 检查器");
        map.insert("Open Local Folders", "打开本地文件夹");
        map.insert("Open Remote Folder", "打开远程文件夹");
        map.insert("Search projects…", "搜索项目…");
        map.insert("Recently opened projects will show up here", "最近打开的项目将在此处显示");
        map.insert("Get help to write anything.", "获取编写任何内容的帮助。");
        map.insert("Chat about your codebase.", "讨论您的代码库。");
        map.insert("Chat about anything with no tools.", "无工具限制的自由对话。");

        // ════════════════════════════════════════════════════════════════════
        // 十一、搜索面板
        // ════════════════════════════════════════════════════════════════════
        map.insert("Search\u{2026}", "搜索\u{2026}");
        map.insert("Replace with\u{2026}", "替换为\u{2026}");
        map.insert("Search all files\u{2026}", "搜索所有文件\u{2026}");
        map.insert("Replace in project\u{2026}", "在项目中替换\u{2026}");
        map.insert("No results found in this project for the provided query", "在此项目中未找到与查询匹配的结果");
        map.insert("Hit enter to search. For more options:", "按 Enter 开始搜索。更多选项：");
        map.insert("Include/exclude specific paths", "包含/排除特定路径");
        map.insert("Find and replace", "查找与替换");
        map.insert("Match with regex", "使用正则匹配");
        map.insert("Match case", "区分大小写");
        map.insert("Match whole words", "全词匹配");
        map.insert("Only Search Open Files", "仅搜索已打开的文件");
        map.insert("Find in Results", "在结果中查找");
        map.insert("Toggle Filters", "切换过滤器");
        map.insert("Project Search", "项目搜索");
        map.insert("Search Limits Reached\nTry narrowing your search", "已达到搜索限制\n请尝试缩小搜索范围");

        // ════════════════════════════════════════════════════════════════════
        // 十二、命令面板
        // ════════════════════════════════════════════════════════════════════
        map.insert("Execute a command...", "执行命令...");
        map.insert("Change Keybinding\u{2026}", "更改键位绑定\u{2026}");

        // ════════════════════════════════════════════════════════════════════
        // 十三、调试器面板
        // ════════════════════════════════════════════════════════════════════
        map.insert("Console", "控制台");
        map.insert("Variables", "变量");
        map.insert("Breakpoints", "断点");
        map.insert("Frames", "调用帧");
        map.insert("Modules", "模块");
        map.insert("Sources", "源文件");
        map.insert("Memory View", "内存视图");
        map.insert("No Breakpoints Set", "未设置断点");
        map.insert("Evaluate", "求值");
        map.insert("New Session", "新建会话");
        map.insert("Edit debug.json", "编辑 debug.json");
        map.insert("Debugger Docs", "调试器文档");
        map.insert("Start", "启动");
        map.insert("Launch Custom", "自定义启动");
        map.insert("Displays program output and allows manual input of debugger commands.", "显示程序输出并允许手动输入调试器命令。");
        map.insert("Shows current values of local and global variables in the current stack frame.", "显示当前栈帧中本地和全局变量的当前值。");
        map.insert("Lists all active breakpoints set in the code.", "列出代码中设置的所有活动断点。");
        map.insert("Displays the call stack, letting you navigate between function calls.", "显示调用栈，可在函数调用之间导航。");
        map.insert("Shows all modules or libraries loaded by the program.", "显示程序加载的所有模块或库。");
        map.insert("Lists all source files currently loaded and used by the debugger.", "列出调试器当前加载和使用的所有源文件。");
        map.insert("Provides an interactive terminal session within the debugging environment.", "在调试环境中提供交互式终端会话。");
        map.insert("Allows inspection of memory contents.", "允许检查内存内容。");
        map.insert("Edit debug.json", "编辑 debug.json");
        map.insert("Open Documentation", "打开文档");
        map.insert("Open Debug Adapter Logs", "打开调试适配器日志");
        map.insert("Close Panel", "关闭面板");
        map.insert("Start Debug Session", "启动调试会话");
        map.insert("Remove Breakpoint", "删除断点");

        // ════════════════════════════════════════════════════════════════════
        // 十四、终端面板
        // ════════════════════════════════════════════════════════════════════
        map.insert("Failed to spawn terminal", "无法启动终端");
        map.insert("Edit Settings", "编辑设置");
        map.insert("Inline Assist", "内联协助");
        map.insert("Rerun task", "重新运行任务");

        // ════════════════════════════════════════════════════════════════════
        // 十五、诊断面板
        // ════════════════════════════════════════════════════════════════════
        map.insert("Project Diagnostics", "项目诊断");
        map.insert("No problems in workspace", "工作区中没有问题");
        map.insert("No errors in workspace", "工作区中没有错误");
        map.insert("No problems", "无问题");
        map.insert("Buffer Diagnostics", "缓冲区诊断");
        map.insert("Copy Diagnostic", "复制诊断信息");

        // ════════════════════════════════════════════════════════════════════
        // 十六、协作面板
        // ════════════════════════════════════════════════════════════════════
        map.insert("Manage Members", "管理成员");
        map.insert("Invite Members", "邀请成员");
        map.insert("Invited", "已邀请");
        map.insert("Admin", "管理员");
        map.insert("Guest", "访客");
        map.insert("Member", "成员");
        map.insert("Add a Contact", "添加联系人");
        map.insert("Accept", "接受");
        map.insert("Decline", "拒绝");
        map.insert("Calling", "呼叫中");
        map.insert("notes", "笔记");
        map.insert("Copy Link", "复制链接");
        map.insert("Open Channel Notes", "打开频道笔记");
        map.insert("Create Channel", "创建频道");
        map.insert("Join Channel", "加入频道");
        map.insert("Accept invite", "接受邀请");
        map.insert("Decline invite", "拒绝邀请");
        map.insert("Cancel invite", "取消邀请");
        map.insert("Clear Filter", "清除过滤器");
        map.insert("Search for new contact", "搜索新联系人");
        map.insert("Open Shared Screen", "打开共享屏幕");
        map.insert("Not in a call", "未在通话中");
        map.insert("Call Diagnostics", "通话诊断");
        map.insert("Open", "打开");
        map.insert("Dismiss", "关闭");

        // ════════════════════════════════════════════════════════════════════
        // 十七、通用 UI 字符串
        // ════════════════════════════════════════════════════════════════════
        map.insert("Could not open file", "无法打开文件");
        map.insert("OK", "确定");
        map.insert("Cancel", "取消");
        map.insert("Close", "关闭");
        map.insert("Delete", "删除");
        map.insert("Rename", "重命名");
        map.insert("Replace", "替换");
        map.insert("Preferences", "偏好设置");
        map.insert("Edit in settings.json", "在 settings.json 中编辑");
        map.insert("Search settings...", "搜索设置\u{2026}");
        map.insert("Reset to Default", "重置为默认值");
        map.insert("Modified in", "修改于");
        map.insert("No Results", "无结果");
        map.insert("User", "用户");
        map.insert("Copied into clipboard", "已复制到剪贴板");
        map.insert("Open Keymap", "打开键位映射");
        map.insert("Open Application Menu", "打开应用程序菜单");
        map.insert("Open File", "打开文件");
        map.insert("New Window", "新建窗口");
        map.insert("Left", "左侧");
        map.insert("Right", "右侧");
        map.insert("Bottom", "底部");
        map.insert("Dock Left", "停靠到左侧");
        map.insert("Dock Bottom", "停靠到底部");
        map.insert("Dock Right", "停靠到右侧");
        map.insert("Flex Width", "弹性宽度");
        map.insert("Fixed Width", "固定宽度");

        // ── 状态栏 ────────────────────────────────────────────────────────────
        map.insert("Line", "行");
        map.insert("Column", "列");
        map.insert("Errors", "错误");
        map.insert("Warnings", "警告");

        // ════════════════════════════════════════════════════════════════════
        // 十八、第二次补齐——最后一批遗漏翻译
        // ════════════════════════════════════════════════════════════════════
        map.insert("Add Folder to Project\u{2026}", "将文件夹添加到项目\u{2026}");
        map.insert("Add Keybinding\u{2026}", "添加键位绑定\u{2026}");
        map.insert("Attach", "附加");
        map.insert("Attach the debugger to a running process", "将调试器附加到正在运行的进程");
        map.insert("Buffer Search", "缓冲区搜索");
        map.insert("Click to Resolve with Agent", "点击以使用智能体解决");
        map.insert("Commit SHA", "提交 SHA");
        map.insert("Continue Program", "继续程序");
        map.insert("Debug", "调试");
        map.insert("Detach", "分离");
        map.insert("Do you want to install the recommended", "是否要安装推荐的");
        map.insert("Dock", "停靠");
        map.insert("Edit settings.json", "编辑 settings.json");
        map.insert("Follow", "跟随");
        map.insert("Found", "已找到");
        map.insert("Grant Mic Access", "授予麦克风访问权限");

        map.insert("Launch", "启动");
        map.insert("Launch a new process with a debugger", "使用调试器启动新进程");
        map.insert("Max Completion Tokens", "最大补全令牌数");
        map.insert("Max Output Tokens", "最大输出令牌数");
        map.insert("Max Tokens", "最大令牌数");
        map.insert("Microphone will be unmuted", "麦克风将取消静音");
        map.insert("Model Name", "模型名称");
        map.insert("New Terminal", "新终端");
        map.insert("Open\u{2026}", "打开\u{2026}");
        map.insert("Pause Program", "暂停程序");
        map.insert("Rerun Session", "重新运行会话");
        map.insert("Resolve Merge Conflict with Agent", "使用智能体解决合并冲突");
        map.insert("Resolve Merge Conflicts with Agent", "使用智能体解决合并冲突");
        map.insert("Room ID copied to clipboard", "房间 ID 已复制到剪贴板");
        map.insert("Run predefined task", "运行预定义任务");
        map.insert("Select the process you want to attach the debugger to", "选择要附加调试器的进程");
        map.insert("Start a predefined debug scenario", "启动预定义调试方案");
        map.insert("Step In", "单步步入");
        map.insert("Terminate All Threads", "终止所有线程");
        map.insert("Terminate Thread", "终止线程");
        map.insert("Unknown Session", "未知会话");
        map.insert("Use", "使用");
        map.insert("View on", "在\u{2026}上查看");
        map.insert("across the codebase", "在整个代码库中");
        map.insert("conflict", "个冲突");
        map.insert("conflicts", "个冲突");
        map.insert("extension for", "的扩展");
        map.insert("files?", "文件？");
        map.insert("untitled", "未命名");

        // ── strum 枚举变体标签（CliDefaultOpenBehavior）──────────────────
        map.insert("Add to Existing Window", "添加到现有窗口");
        map.insert("Open a New Window", "打开新窗口");
        // ── strum 枚举变体标签（RestoreOnStartupBehavior）───────────────
        map.insert("EmptyTab", "空白标签页");
        map.insert("LastWorkspace", "上次工作区");
        map.insert("LastSession", "上次会话");
        map.insert("Launchpad", "启动面板");

        // ── 智能体线程默认标题 ──────────────────────────────────────────
        map.insert("New Agent Thread", "新建智能体线程");

        // ── strum 枚举变体标签（自动生成）─────────────────────────────
        map.insert("ActiveEditor", "活动编辑器");
        map.insert("AfterDelay", "延迟后");
        map.insert("AllEditors", "所有编辑器");
        map.insert("AllScreens", "所有屏幕");
        map.insert("Alt", "Alt");
        map.insert("Always", "始终");
        map.insert("AlwaysCollapsed", "始终折叠");
        map.insert("AlwaysExpanded", "始终展开");
        map.insert("AlwaysHome", "始终主目录");
        map.insert("Anywhere", "任意位置");
        map.insert("Atom", "Atom");
        map.insert("Auto", "自动");
        map.insert("Bar", "竖线");
        map.insert("Binary", "二进制");
        map.insert("Block", "方块");
        map.insert("Boundary", "边界");
        map.insert("Bounded", "受限");
        map.insert("Center", "居中");
        map.insert("Chinese Simplified", "中文简体");
        map.insert("Client", "客户端");
        map.insert("CloseWindow", "关闭窗口");
        map.insert("CmdOrCtrl", "Cmd 或 Ctrl");
        map.insert("CodeGemma", "CodeGemma");
        map.insert("CodeLlama", "CodeLlama");
        map.insert("Codestral", "Codestral");
        map.insert("Combined", "合并");
        map.insert("Comfortable", "舒适");
        map.insert("Contained", "包含");
        map.insert("CurrentFileDirectory", "当前文件目录");
        map.insert("CurrentProjectDirectory", "当前项目目录");
        map.insert("Dark", "深色");
        map.insert("Decimal", "十进制");
        map.insert("DeepseekCoder", "DeepseekCoder");
        map.insert("Default", "默认");
        map.insert("Detect", "自动检测");
        map.insert("DirectoriesFirst", "目录优先");
        map.insert("Disabled", "已禁用");
        map.insert("Down", "向下");
        map.insert("Dynamic", "动态");
        map.insert("Eager", "急切");
        map.insert("EditorWidth", "编辑器宽度");
        map.insert("Emacs", "Emacs");
        map.insert("Enforce CRLF", "强制 CRLF");
        map.insert("Enforce LF", "强制 LF");
        map.insert("English", "English");
        map.insert("Error", "错误");
        map.insert("Fallback", "回退");
        map.insert("FileNameFirst", "文件名优先");
        map.insert("FilePathFirst", "文件路径优先");
        map.insert("FilesFirst", "文件优先");
        map.insert("FindAllReferences", "查找所有引用");
        map.insert("FirstProjectDirectory", "第一个项目目录");
        map.insert("Fixed", "固定");
        map.insert("Full", "完整");
        map.insert("Glm", "Glm");
        map.insert("Grayscale", "灰度");
        map.insert("Hide", "隐藏");
        map.insert("Hint", "提示");
        map.insert("History", "历史记录");
        map.insert("Hollow", "空心");
        map.insert("Hover", "悬停");
        map.insert("Icon", "图标");
        map.insert("InComments", "注释中");
        map.insert("InSelections", "选区中");
        map.insert("IndentAware", "缩进感知");
        map.insert("Indexed", "已索引");
        map.insert("Infer", "Infer");
        map.insert("Info", "信息");
        map.insert("Information", "信息");
        map.insert("Inherit", "继承");
        map.insert("Inline", "内联");
        map.insert("Insert", "插入");
        map.insert("Instruction", "指令");
        map.insert("JetBrains", "JetBrains");
        map.insert("KeepWindowOpen", "保持窗口打开");
        map.insert("LabelColor", "标签颜色");
        map.insert("Large", "大");
        map.insert("LeftAligned", "左对齐");
        map.insert("LeftNeighbour", "左侧邻居");
        map.insert("LeftOnly", "仅左侧");
        map.insert("LeftOpen", "左侧开口");
        map.insert("Light", "浅色");
        map.insert("Lower", "小写");
        map.insert("Medium", "中");
        map.insert("Menu", "菜单");
        map.insert("Minimum", "最小");
        map.insert("Mixed", "混合");
        map.insert("Neighbour", "邻居");
        map.insert("Never", "从不");
        map.insert("No", "否");
        map.insert("NonUtf8", "非 UTF-8");
        map.insert("None", "无");
        map.insert("Normal", "普通");
        map.insert("Off", "关闭");
        map.insert("On", "开启");
        map.insert("OnFocusChange", "焦点变化时");
        map.insert("OnTyping", "输入时");
        map.insert("OnTypingAndAction", "输入和操作时");
        map.insert("OnWindowChange", "窗口变化时");
        map.insert("OnYank", "复制时");
        map.insert("OnePage", "一页");
        map.insert("PlatformDefault", "平台默认");
        map.insert("Prefer CRLF", "优先 CRLF");
        map.insert("Prefer LF", "优先 LF");
        map.insert("PreferLine", "优先行长");
        map.insert("Preserve", "保持");
        map.insert("PreserveIndent", "保持缩进");
        map.insert("Preview", "预览");
        map.insert("PrimaryScreen", "主屏幕");
        map.insert("QuitApp", "退出应用");
        map.insert("Qwen", "Qwen");
        map.insert("ReplaceSubsequence", "替换子序列");
        map.insert("ReplaceSuffix", "替换后缀");
        map.insert("RightAligned", "右对齐");
        map.insert("RightOpen", "右侧开口");
        map.insert("Select", "选择");
        map.insert("Server", "服务器");
        map.insert("Small", "小");
        map.insert("Smart", "智能");
        map.insert("Split", "拆分");
        map.insert("StagedHollow", "已暂存空心");
        map.insert("Standard", "标准");
        map.insert("StarCoder", "StarCoder");
        map.insert("Statement", "语句");
        map.insert("Static", "静态");
        map.insert("Sublime Text", "Sublime Text");
        map.insert("Subpixel", "亚像素");
        map.insert("Subtle", "微妙");
        map.insert("SyntaxAware", "语法感知");
        map.insert("System", "系统");
        map.insert("TerminalControlled", "终端控制");
        map.insert("TextMate", "TextMate");
        map.insert("Top", "顶部");
        map.insert("TrackedFiles", "已跟踪文件");
        map.insert("Trailing", "尾部");
        map.insert("Underline", "下划线");
        map.insert("Unicode", "Unicode");
        map.insert("Unified", "统一");
        map.insert("UnstagedHollow", "未暂存空心");
        map.insert("Up", "向上");
        map.insert("Upper", "大写");
        map.insert("VSCode", "VSCode");
        map.insert("VerticalScrollMargin", "垂直滚动边距");
        map.insert("Warning", "警告");
        map.insert("WhenHidden", "隐藏时");
        map.insert("WithArguments", "带参数");
        map.insert("Wrapped", "换行");
        map.insert("XLarge", "超大");
        map.insert("Yes", "是");
        map.insert("Zeta", "Zeta");
        map.insert("Zeta2", "Zeta2");
        map.insert("Zeta2_1", "Zeta2_1");

        // ── 第三批补齐 ────────────────────────────────────────────────
        map.insert("(child)", "(子级)");
        map.insert("Are you sure you want to discard changes to", "确定要放弃更改吗？");
        map.insert("Click to Follow", "点击以跟随");
        map.insert("Discard Changes", "放弃更改");
        map.insert("Discard Tracked Changes", "放弃已跟踪更改");
        map.insert("Disconnected from SSH host", "已断开 SSH 主机连接");
        map.insert("Disconnected from remote project", "已断开远程项目连接");
        map.insert("Editor Layout", "编辑器布局");
        map.insert("Flat View", "平铺视图");
        map.insert("Hide Others", "隐藏其他");
        map.insert("Hide Zed", "隐藏 Zed");
        map.insert("Install CLI", "安装 CLI");
        map.insert("Mic only", "仅麦克风");
        map.insert("New Profile", "新建配置文件");
        map.insert("New\u{2026}", "新建\u{2026}");
        map.insert("Save As\u{2026}", "另存为\u{2026}");
        map.insert("Screen", "屏幕");
        map.insert("Search channels\u{2026}", "搜索频道\u{2026}");
        map.insert("Select Debugger", "选择调试器");
        map.insert("Show All", "显示全部");
        map.insert("Show in Git Graph", "在 Git 图中显示");
        map.insert("Sort by Path", "按路径排序");
        map.insert("Sort by Status", "按状态排序");
        map.insert("Split Pane", "拆分窗格");
        map.insert("Stash All", "暂存全部");
        map.insert("This Debug Session is still running. Are you sure you want to terminate it?", "此调试会话仍在运行。确定要终止吗？");
        map.insert("This cannot be undone.", "此操作不可撤销。");
        map.insert("This provider will use an OpenAI compatible API.", "此提供商将使用 OpenAI 兼容的 API。");
        map.insert("Trash Untracked Files", "删除未跟踪文件");
        map.insert("Unstage All", "取消暂存全部");
        map.insert("Zed", "Zed");
        map.insert("Zed \u{2014} Settings", "Zed \u{2014} 设置");
        map
    })
}
