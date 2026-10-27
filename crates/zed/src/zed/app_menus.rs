use gpui::{App, Menu, MenuItem, OsAction};
use zed_actions::{Quit, project_panel};

pub fn app_menus(_cx: &mut App) -> Vec<Menu> {
    vec![
        Menu {
            name: "zed-md-viewer".into(),
            disabled: false,
            items: vec![
                MenuItem::action("About zed-md-viewer", zed_actions::About),
                MenuItem::separator(),
                MenuItem::submenu(Menu::new("Settings").items([
                    MenuItem::action("Open Settings File", super::OpenSettingsFile),
                    MenuItem::action("Open Project Settings File", super::OpenProjectSettingsFile),
                    MenuItem::action("Open Default Settings", super::OpenDefaultSettings),
                    MenuItem::separator(),
                    MenuItem::action("Open Keymap File", zed_actions::OpenKeymapFile),
                    MenuItem::action("Open Default Key Bindings", zed_actions::OpenDefaultKeymap),
                    MenuItem::separator(),
                    MenuItem::action(
                        "Select Theme...",
                        zed_actions::theme_selector::Toggle::default(),
                    ),
                ])),
                MenuItem::separator(),
                #[cfg(target_os = "macos")]
                MenuItem::action("Hide zed-md-viewer", super::Hide),
                #[cfg(target_os = "macos")]
                MenuItem::action("Hide Others", super::HideOthers),
                #[cfg(target_os = "macos")]
                MenuItem::action("Show All", super::ShowAll),
                MenuItem::separator(),
                MenuItem::action("Quit zed-md-viewer", Quit),
            ],
        },
        Menu {
            name: "File".into(),
            disabled: false,
            items: vec![
                MenuItem::action("New", workspace::NewFile),
                MenuItem::action("New Window", workspace::NewWindow),
                MenuItem::separator(),
                #[cfg(not(target_os = "macos"))]
                MenuItem::action("Open File...", workspace::OpenFiles),
                MenuItem::action(
                    if cfg!(not(target_os = "macos")) {
                        "Open Folder..."
                    } else {
                        "Open…"
                    },
                    workspace::Open::default(),
                ),
                MenuItem::action("Open Recent…", zed_actions::OpenRecent::default()),
                MenuItem::separator(),
                MenuItem::action("Add Folder to Project…", workspace::AddFolderToProject),
                MenuItem::separator(),
                MenuItem::action("Save", workspace::Save { save_intent: None }),
                MenuItem::action("Save As…", workspace::SaveAs),
                MenuItem::action("Save All", workspace::SaveAll { save_intent: None }),
                MenuItem::separator(),
                MenuItem::action(
                    "Close Editor",
                    workspace::CloseActiveItem {
                        save_intent: None,
                        close_pinned: true,
                    },
                ),
                MenuItem::action("Close Project", workspace::CloseProject),
                MenuItem::action("Close Window", workspace::CloseWindow),
            ],
        },
        Menu {
            name: "Edit".into(),
            disabled: false,
            items: vec![
                MenuItem::os_action("Undo", editor::actions::Undo, OsAction::Undo),
                MenuItem::os_action("Redo", editor::actions::Redo, OsAction::Redo),
                MenuItem::separator(),
                MenuItem::os_action("Cut", editor::actions::Cut, OsAction::Cut),
                MenuItem::os_action("Copy", editor::actions::Copy, OsAction::Copy),
                MenuItem::action("Copy and Trim", editor::actions::CopyAndTrim),
                MenuItem::os_action("Paste", editor::actions::Paste, OsAction::Paste),
                MenuItem::separator(),
                MenuItem::action("Find", search::buffer_search::Deploy::find()),
                MenuItem::action("Find in Project", workspace::DeploySearch::default()),
                MenuItem::separator(),
                MenuItem::action(
                    "Toggle Line Comment",
                    editor::actions::ToggleComments::default(),
                ),
            ],
        },
        Menu {
            name: "Selection".into(),
            disabled: false,
            items: vec![
                MenuItem::os_action(
                    "Select All",
                    editor::actions::SelectAll,
                    OsAction::SelectAll,
                ),
                MenuItem::action("Expand Selection", editor::actions::SelectLargerSyntaxNode),
                MenuItem::action("Shrink Selection", editor::actions::SelectSmallerSyntaxNode),
                MenuItem::separator(),
                MenuItem::action(
                    "Add Cursor Above",
                    editor::actions::AddSelectionAbove {
                        skip_soft_wrap: true,
                    },
                ),
                MenuItem::action(
                    "Add Cursor Below",
                    editor::actions::AddSelectionBelow {
                        skip_soft_wrap: true,
                    },
                ),
                MenuItem::action(
                    "Select Next Occurrence",
                    editor::actions::SelectNext {
                        replace_newest: false,
                    },
                ),
                MenuItem::action(
                    "Select Previous Occurrence",
                    editor::actions::SelectPrevious {
                        replace_newest: false,
                    },
                ),
                MenuItem::action("Select All Occurrences", editor::actions::SelectAllMatches),
            ],
        },
        Menu {
            name: "View".into(),
            disabled: false,
            items: vec![
                MenuItem::action(
                    "Zoom In",
                    zed_actions::IncreaseBufferFontSize { persist: false },
                ),
                MenuItem::action(
                    "Zoom Out",
                    zed_actions::DecreaseBufferFontSize { persist: false },
                ),
                MenuItem::action(
                    "Reset Zoom",
                    zed_actions::ResetBufferFontSize { persist: false },
                ),
                MenuItem::separator(),
                MenuItem::action("Toggle Left Dock", workspace::ToggleLeftDock),
                MenuItem::action("Toggle Right Dock", workspace::ToggleRightDock),
                MenuItem::action("Toggle Bottom Dock", workspace::ToggleBottomDock),
                MenuItem::separator(),
                MenuItem::action("Project Panel", project_panel::ToggleFocus),
                MenuItem::separator(),
                MenuItem::action(
                    "Markdown Preview",
                    zed_actions::preview::markdown::OpenPreview,
                ),
                MenuItem::action(
                    "Markdown Preview to the Side",
                    zed_actions::preview::markdown::OpenPreviewToTheSide,
                ),
            ],
        },
        Menu {
            name: "Go".into(),
            disabled: false,
            items: vec![
                MenuItem::action("Back", workspace::GoBack),
                MenuItem::action("Forward", workspace::GoForward),
                MenuItem::separator(),
                MenuItem::action("Command Palette...", zed_actions::command_palette::Toggle),
                MenuItem::separator(),
                MenuItem::action("Go to File...", workspace::ToggleFileFinder::default()),
                MenuItem::action("Go to Line/Column...", editor::actions::ToggleGoToLine),
            ],
        },
        Menu {
            name: "Window".into(),
            disabled: false,
            items: vec![
                MenuItem::action("Minimize", super::Minimize),
                MenuItem::action("Zoom", super::Zoom),
            ],
        },
        Menu {
            name: "Help".into(),
            disabled: false,
            items: vec![
                MenuItem::action(
                    "Documentation",
                    super::OpenBrowser {
                        url: "https://zed.dev/docs".into(),
                    },
                ),
                MenuItem::action("View Dependency Licenses", zed_actions::OpenLicenses),
            ],
        },
    ]
}
