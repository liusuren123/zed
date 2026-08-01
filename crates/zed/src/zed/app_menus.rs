use collab_ui::collab_panel;
use gpui::{App, Menu, MenuItem, OsAction};
use release_channel::ReleaseChannel;
use terminal_view::terminal_panel;
use theme::translate;
use zed_actions::{Quit, assistant, debug_panel, dev, git_panel, project_panel};

pub fn app_menus(cx: &mut App) -> Vec<Menu> {
    let mut view_items = vec![
        MenuItem::action(
            translate("Zoom In", cx),
            zed_actions::IncreaseBufferFontSize { persist: false },
        ),
        MenuItem::action(
            translate("Zoom Out", cx),
            zed_actions::DecreaseBufferFontSize { persist: false },
        ),
        MenuItem::action(
            translate("Reset Zoom", cx),
            zed_actions::ResetBufferFontSize { persist: false },
        ),
        MenuItem::action(
            translate("Reset All Zoom", cx),
            zed_actions::ResetAllZoom { persist: false },
        ),
        MenuItem::separator(),
        MenuItem::action(translate("Toggle Left Dock", cx), workspace::ToggleLeftDock),
        MenuItem::action(
            translate("Toggle Right Dock", cx),
            workspace::ToggleRightDock,
        ),
        MenuItem::action(
            translate("Toggle Bottom Dock", cx),
            workspace::ToggleBottomDock,
        ),
        MenuItem::action(translate("Toggle All Docks", cx), workspace::ToggleAllDocks),
        MenuItem::submenu(Menu {
            name: translate("Editor Layout", cx),
            disabled: false,
            items: vec![
                MenuItem::action(translate("Split Up", cx), workspace::SplitUp::default()),
                MenuItem::action(translate("Split Down", cx), workspace::SplitDown::default()),
                MenuItem::action(translate("Split Left", cx), workspace::SplitLeft::default()),
                MenuItem::action(
                    translate("Split Right", cx),
                    workspace::SplitRight::default(),
                ),
            ],
        }),
        MenuItem::separator(),
        MenuItem::action(translate("Project Panel", cx), project_panel::ToggleFocus),
        MenuItem::action(translate("Outline Panel", cx), outline_panel::ToggleFocus),
        MenuItem::action(translate("Collab Panel", cx), collab_panel::ToggleFocus),
        MenuItem::action(translate("Terminal Panel", cx), terminal_panel::Toggle),
        MenuItem::action(translate("Debugger Panel", cx), debug_panel::ToggleFocus),
        MenuItem::action(translate("Agent Panel", cx), assistant::ToggleFocus),
        MenuItem::action(translate("Git Panel", cx), git_panel::ToggleFocus),
        MenuItem::separator(),
        MenuItem::action(translate("Diagnostics", cx), diagnostics::Deploy),
        MenuItem::separator(),
    ];

    if ReleaseChannel::try_global(cx) == Some(ReleaseChannel::Dev) {
        view_items.push(MenuItem::action(
            translate("Toggle GPUI Inspector", cx),
            dev::ToggleInspector,
        ));
        view_items.push(MenuItem::separator());
    }

    vec![
        Menu {
            name: translate("Zed", cx),
            disabled: false,
            items: vec![
                MenuItem::action(translate("About Zed", cx), zed_actions::About),
                MenuItem::action(translate("Check for Updates", cx), auto_update::Check),
                MenuItem::separator(),
                MenuItem::submenu(Menu::new(translate("Settings", cx)).items([
                    MenuItem::action(translate("Open Settings", cx), zed_actions::OpenSettings),
                    MenuItem::action(translate("Open Settings File", cx), super::OpenSettingsFile),
                    MenuItem::action(
                        translate("Open Project Settings", cx),
                        zed_actions::OpenProjectSettings,
                    ),
                    MenuItem::action(
                        translate("Open Project Settings File", cx),
                        super::OpenProjectSettingsFile,
                    ),
                    MenuItem::action(
                        translate("Open Default Settings", cx),
                        super::OpenDefaultSettings,
                    ),
                    MenuItem::separator(),
                    MenuItem::action(translate("Open Keymap", cx), zed_actions::OpenKeymap),
                    MenuItem::action(
                        translate("Open Keymap File", cx),
                        zed_actions::OpenKeymapFile,
                    ),
                    MenuItem::action(
                        translate("Open Default Key Bindings", cx),
                        zed_actions::OpenDefaultKeymap,
                    ),
                    MenuItem::separator(),
                    MenuItem::action(
                        translate("Select Theme...", cx),
                        zed_actions::theme_selector::Toggle::default(),
                    ),
                    MenuItem::action(
                        translate("Select Icon Theme...", cx),
                        zed_actions::icon_theme_selector::Toggle::default(),
                    ),
                ])),
                MenuItem::separator(),
                #[cfg(target_os = "macos")]
                MenuItem::os_submenu("Services", gpui::SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action(
                    translate("Extensions", cx),
                    zed_actions::Extensions::default(),
                ),
                #[cfg(not(target_os = "windows"))]
                MenuItem::action(translate("Install CLI", cx), install_cli::InstallCliBinary),
                MenuItem::separator(),
                #[cfg(target_os = "macos")]
                MenuItem::action(translate("Hide Zed", cx), super::Hide),
                #[cfg(target_os = "macos")]
                MenuItem::action(translate("Hide Others", cx), super::HideOthers),
                #[cfg(target_os = "macos")]
                MenuItem::action(translate("Show All", cx), super::ShowAll),
                MenuItem::separator(),
                MenuItem::action(translate("Quit Zed", cx), Quit),
            ],
        },
        Menu {
            name: translate("File", cx),
            disabled: false,
            items: vec![
                MenuItem::action(translate("New", cx), workspace::NewFile),
                MenuItem::action(translate("New Window", cx), workspace::NewWindow),
                MenuItem::separator(),
                #[cfg(not(target_os = "macos"))]
                MenuItem::action(translate("Open File...", cx), workspace::OpenFiles),
                MenuItem::action(
                    if cfg!(not(target_os = "macos")) {
                        translate("Open Folder...", cx)
                    } else {
                        translate("Open\u{2026}", cx)
                    },
                    workspace::Open::default(),
                ),
                MenuItem::action(
                    translate("Open Recent...", cx),
                    zed_actions::OpenRecent {
                        create_new_window: false,
                    },
                ),
                MenuItem::action(
                    translate("Open Remote...", cx),
                    zed_actions::OpenRemote {
                        create_new_window: false,
                        from_existing_connection: false,
                    },
                ),
                MenuItem::separator(),
                MenuItem::action(
                    translate("Add Folder to Project\u{2026}", cx),
                    workspace::AddFolderToProject,
                ),
                MenuItem::separator(),
                MenuItem::action(translate("Save", cx), workspace::Save { save_intent: None }),
                MenuItem::action(translate("Save As\u{2026}", cx), workspace::SaveAs),
                MenuItem::action(
                    translate("Save All", cx),
                    workspace::SaveAll { save_intent: None },
                ),
                MenuItem::separator(),
                MenuItem::action(
                    translate("Close Editor", cx),
                    workspace::CloseActiveItem {
                        save_intent: None,
                        close_pinned: true,
                    },
                ),
                MenuItem::action(translate("Close Project", cx), workspace::CloseProject),
                MenuItem::action(translate("Close Window", cx), workspace::CloseWindow),
            ],
        },
        Menu {
            name: translate("Edit", cx),
            disabled: false,
            items: vec![
                MenuItem::os_action(translate("Undo", cx), editor::actions::Undo, OsAction::Undo),
                MenuItem::os_action(translate("Redo", cx), editor::actions::Redo, OsAction::Redo),
                MenuItem::separator(),
                MenuItem::os_action(translate("Cut", cx), editor::actions::Cut, OsAction::Cut),
                MenuItem::os_action(translate("Copy", cx), editor::actions::Copy, OsAction::Copy),
                MenuItem::action(translate("Copy and Trim", cx), editor::actions::CopyAndTrim),
                MenuItem::os_action(
                    translate("Paste", cx),
                    editor::actions::Paste,
                    OsAction::Paste,
                ),
                MenuItem::separator(),
                MenuItem::action(translate("Find", cx), search::buffer_search::Deploy::find()),
                MenuItem::action(
                    translate("Find in Project", cx),
                    workspace::DeploySearch::default(),
                ),
                MenuItem::separator(),
                MenuItem::action(
                    translate("Toggle Line Comment", cx),
                    editor::actions::ToggleComments::default(),
                ),
            ],
        },
        Menu {
            name: translate("Selection", cx),
            disabled: false,
            items: vec![
                MenuItem::os_action(
                    translate("Select All", cx),
                    editor::actions::SelectAll,
                    OsAction::SelectAll,
                ),
                MenuItem::action(
                    translate("Expand Selection", cx),
                    editor::actions::SelectLargerSyntaxNode,
                ),
                MenuItem::action(
                    translate("Shrink Selection", cx),
                    editor::actions::SelectSmallerSyntaxNode,
                ),
                MenuItem::action(
                    translate("Select Next Sibling", cx),
                    editor::actions::SelectNextSyntaxNode,
                ),
                MenuItem::action(
                    translate("Select Previous Sibling", cx),
                    editor::actions::SelectPreviousSyntaxNode,
                ),
                MenuItem::separator(),
                MenuItem::action(
                    translate("Add Cursor Above", cx),
                    editor::actions::AddSelectionAbove {
                        skip_soft_wrap: true,
                    },
                ),
                MenuItem::action(
                    translate("Add Cursor Below", cx),
                    editor::actions::AddSelectionBelow {
                        skip_soft_wrap: true,
                    },
                ),
                MenuItem::action(
                    translate("Select Next Occurrence", cx),
                    editor::actions::SelectNext {
                        replace_newest: false,
                    },
                ),
                MenuItem::action(
                    translate("Select Previous Occurrence", cx),
                    editor::actions::SelectPrevious {
                        replace_newest: false,
                    },
                ),
                MenuItem::action(
                    translate("Select All Occurrences", cx),
                    editor::actions::SelectAllMatches,
                ),
                MenuItem::separator(),
                MenuItem::action(translate("Move Line Up", cx), editor::actions::MoveLineUp),
                MenuItem::action(
                    translate("Move Line Down", cx),
                    editor::actions::MoveLineDown,
                ),
                MenuItem::action(
                    translate("Duplicate Selection", cx),
                    editor::actions::DuplicateLineDown,
                ),
            ],
        },
        Menu {
            name: translate("View", cx),
            disabled: false,
            items: view_items,
        },
        Menu {
            name: translate("Go", cx),
            disabled: false,
            items: vec![
                MenuItem::action(translate("Back", cx), workspace::GoBack),
                MenuItem::action(translate("Forward", cx), workspace::GoForward),
                MenuItem::separator(),
                MenuItem::action(
                    translate("Command Palette...", cx),
                    zed_actions::command_palette::Toggle,
                ),
                MenuItem::separator(),
                MenuItem::action(
                    translate("Go to File...", cx),
                    workspace::ToggleFileFinder::default(),
                ),
                // MenuItem::action("Go to Symbol in Project", project_symbols::Toggle),
                MenuItem::action(
                    translate("Go to Symbol in Editor...", cx),
                    zed_actions::outline::ToggleOutline,
                ),
                MenuItem::action(
                    translate("Go to Line/Column...", cx),
                    editor::actions::ToggleGoToLine,
                ),
                MenuItem::separator(),
                MenuItem::action(
                    translate("Go to Definition", cx),
                    editor::actions::GoToDefinition,
                ),
                MenuItem::action(
                    translate("Go to Declaration", cx),
                    editor::actions::GoToDeclaration,
                ),
                MenuItem::action(
                    translate("Go to Type Definition", cx),
                    editor::actions::GoToTypeDefinition,
                ),
                MenuItem::action(
                    translate("Find All References", cx),
                    editor::actions::FindAllReferences::default(),
                ),
                MenuItem::separator(),
                MenuItem::action(
                    translate("Next Problem", cx),
                    editor::actions::GoToDiagnostic::default(),
                ),
                MenuItem::action(
                    translate("Previous Problem", cx),
                    editor::actions::GoToPreviousDiagnostic::default(),
                ),
            ],
        },
        Menu {
            name: translate("Run", cx),
            disabled: false,
            items: vec![
                MenuItem::action(
                    translate("Spawn Task", cx),
                    zed_actions::Spawn::ViaModal {
                        reveal_target: None,
                    },
                ),
                MenuItem::action(translate("Start Debugger", cx), debugger_ui::Start),
                MenuItem::separator(),
                MenuItem::action(
                    translate("Edit tasks.json...", cx),
                    crate::zed::OpenProjectTasks,
                ),
                MenuItem::action(
                    translate("Edit debug.json...", cx),
                    zed_actions::OpenProjectDebugTasks,
                ),
                MenuItem::separator(),
                MenuItem::action(translate("Continue", cx), debugger_ui::Continue),
                MenuItem::action(translate("Step Over", cx), debugger_ui::StepOver),
                MenuItem::action(translate("Step Into", cx), debugger_ui::StepInto),
                MenuItem::action(translate("Step Out", cx), debugger_ui::StepOut),
                MenuItem::separator(),
                MenuItem::action(
                    translate("Toggle Breakpoint", cx),
                    editor::actions::ToggleBreakpoint,
                ),
                MenuItem::action(
                    translate("Edit Breakpoint", cx),
                    editor::actions::EditLogBreakpoint,
                ),
                MenuItem::action(
                    translate("Clear All Breakpoints", cx),
                    debugger_ui::ClearAllBreakpoints,
                ),
            ],
        },
        Menu {
            name: translate("Window", cx),
            disabled: false,
            items: vec![
                MenuItem::action(translate("Minimize", cx), super::Minimize),
                MenuItem::action(translate("Zoom", cx), super::Zoom),
                MenuItem::separator(),
            ],
        },
        Menu {
            name: translate("Help", cx),
            disabled: false,
            items: vec![
                MenuItem::action(
                    translate("View Release Notes Locally", cx),
                    auto_update_ui::ViewReleaseNotesLocally,
                ),
                MenuItem::action(
                    translate("View Telemetry", cx),
                    zed_actions::OpenTelemetryLog,
                ),
                MenuItem::action(
                    translate("View Dependency Licenses", cx),
                    zed_actions::OpenLicenses,
                ),
                MenuItem::action(translate("Show Welcome", cx), onboarding::ShowWelcome),
                MenuItem::separator(),
                MenuItem::action(
                    translate("File Bug Report...", cx),
                    zed_actions::feedback::FileBugReport,
                ),
                MenuItem::action(
                    translate("Request Feature...", cx),
                    zed_actions::feedback::RequestFeature,
                ),
                MenuItem::action(
                    translate("Email Us...", cx),
                    zed_actions::feedback::EmailZed,
                ),
                MenuItem::separator(),
                MenuItem::action(
                    translate("Documentation", cx),
                    super::OpenBrowser {
                        url: "https://zed.dev/docs".into(),
                    },
                ),
                MenuItem::action(translate("Zed Repository", cx), feedback::OpenZedRepo),
                MenuItem::action(
                    translate("Zed Twitter", cx),
                    super::OpenBrowser {
                        url: "https://twitter.com/zeddotdev".into(),
                    },
                ),
                MenuItem::action(
                    translate("Join the Team", cx),
                    super::OpenBrowser {
                        url: "https://zed.dev/jobs".into(),
                    },
                ),
            ],
        },
    ]
}
