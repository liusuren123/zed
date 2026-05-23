use gpui::{
    App, Context, IntoElement, ParentElement, Render, Styled, Subscription, WeakEntity, Window, div,
};
use project::TaskSourceKind;
use task::TaskTemplate;
use ui::{
    Button, ButtonCommon, Clickable, ContextMenu, IconButtonShape, IconName, IconSize, Label,
    LabelSize, PopoverMenu, PopoverMenuHandle, Tooltip, prelude::*,
};
use util::ResultExt;
use workspace::{HideStatusItem, StatusItemView, Workspace, item::ItemHandle};

use crate::spawn_tasks_filtered;

pub struct TaskRunnerButton {
    workspace: WeakEntity<Workspace>,
    tasks: Vec<(TaskSourceKind, TaskTemplate)>,
    default_task_label: Option<String>,
    menu_handle: PopoverMenuHandle<ContextMenu>,
    _observe_task_inventory: Option<Subscription>,
}

impl TaskRunnerButton {
    pub fn new(workspace: &Workspace, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            workspace: workspace.weak_handle(),
            tasks: Vec::new(),
            default_task_label: None,
            menu_handle: PopoverMenuHandle::default(),
            _observe_task_inventory: None,
        };
        this.setup_task_observer(window, cx);
        this.refresh_tasks(window, cx);
        this
    }

    fn setup_task_observer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };
        let project = workspace.read(cx).project().clone();
        let task_inventory = project.read_with(cx, |project, cx| {
            project.task_store().read(cx).task_inventory().cloned()
        });
        if let Some(task_inventory) = task_inventory {
            self._observe_task_inventory =
                Some(
                    cx.observe_in(&task_inventory, window, |this, _, window, cx| {
                        this.refresh_tasks(window, cx);
                    }),
                );
        }
    }

    fn refresh_tasks(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };
        let project = workspace.read(cx).project().clone();

        let task_inventory = project.read_with(cx, |project, cx| {
            project.task_store().read(cx).task_inventory().cloned()
        });

        let Some(task_inventory) = task_inventory else {
            return;
        };

        let worktree_id = workspace
            .read(cx)
            .visible_worktrees(cx)
            .next()
            .map(|tree| tree.read(cx).id());

        cx.spawn_in(window, async move |this, cx| {
            let tasks = task_inventory
                .read_with(cx, |inventory, cx| {
                    inventory.list_tasks(None, None, worktree_id, cx)
                })
                .await;

            let local_tasks: Vec<_> = tasks
                .into_iter()
                .filter(|(kind, _)| matches!(kind, TaskSourceKind::Worktree { .. }))
                .collect();

            this.update_in(cx, |this, _window, cx| {
                this.tasks = local_tasks;
                cx.notify();
            })?;
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    fn default_task(&self) -> Option<(TaskSourceKind, TaskTemplate)> {
        if let Some(label) = &self.default_task_label {
            if let Some(task) = self.tasks.iter().find(|(_, t)| &t.label == label).cloned() {
                return Some(task);
            }
        }
        self.tasks.first().cloned()
    }

    fn run_task(&mut self, task_label: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };
        workspace.update(cx, |_workspace, cx| {
            spawn_tasks_filtered(
                {
                    let task_label = task_label.clone();
                    move |(_, task)| task.label == task_label
                },
                None,
                window,
                cx,
            )
            .detach_and_log_err(cx);
        });
    }

    fn run_default_task(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(default) = self.default_task() else {
            return;
        };
        let label = default.1.label.clone();
        self.run_task(label, window, cx);
    }

    fn select_task(&mut self, label: String, window: &mut Window, cx: &mut Context<Self>) {
        self.default_task_label = Some(label.clone());
        self.menu_handle.hide(cx);
        cx.notify();
        self.run_task(label, window, cx);
    }
}

impl Render for TaskRunnerButton {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.tasks.is_empty() {
            return div().hidden();
        }

        let default_label = self
            .default_task()
            .map(|(_, t)| t.label.clone())
            .unwrap_or_else(|| "Run Task".to_string());

        let weak_entity = cx.entity().downgrade();
        let menu_handle = self.menu_handle.clone();

        h_flex()
            .gap_0()
            .child(
                Button::new("run-default-task", default_label.clone())
                    .label_size(LabelSize::Small)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.run_default_task(window, cx);
                    }))
                    .tooltip(Tooltip::text("Run Default Task")),
            )
            .child(
                PopoverMenu::new("task-runner-menu")
                    .with_handle(menu_handle.clone())
                    .trigger(
                        IconButton::new("task-dropdown-arrow", IconName::ChevronDown)
                            .shape(IconButtonShape::Square)
                            .icon_size(IconSize::Small),
                    )
                    .anchor(gpui::Anchor::BottomLeft)
                    .menu({
                        let weak_entity = weak_entity.clone();
                        move |window, cx| {
                            let entity = weak_entity.upgrade()?;
                            Some(entity.update(cx, |this, cx| {
                                let weak_entity = cx.entity().downgrade();
                                ContextMenu::build(window, cx, |mut menu, _window, _cx| {
                                    for (_, template) in &this.tasks {
                                        let label = template.label.clone();
                                        let is_default = this
                                            .default_task_label
                                            .as_ref()
                                            .is_some_and(|d| d == &label)
                                            || (this.default_task_label.is_none()
                                                && this
                                                    .tasks
                                                    .first()
                                                    .is_some_and(|(_, t)| t.label == label));

                                        let weak_entity = weak_entity.clone();
                                        let label_for_display = label.clone();
                                        let label_for_select = label;

                                        menu = menu.custom_entry(
                                            {
                                                let label = label_for_display.clone();
                                                let is_default = is_default;
                                                move |_window, _cx| {
                                                    h_flex()
                                                        .w_full()
                                                        .gap_1()
                                                        .child(
                                                            Label::new(if is_default {
                                                                format!("{} \u{2713}", label)
                                                            } else {
                                                                label.clone()
                                                            })
                                                            .size(LabelSize::Small),
                                                        )
                                                        .into_any_element()
                                                }
                                            },
                                            move |window, cx| {
                                                weak_entity
                                                    .update(cx, |this, cx| {
                                                        this.select_task(
                                                            label_for_select.clone(),
                                                            window,
                                                            cx,
                                                        );
                                                    })
                                                    .log_err();
                                            },
                                        );
                                    }
                                    menu
                                })
                            }))
                        }
                    }),
            )
    }
}

impl StatusItemView for TaskRunnerButton {
    fn set_active_pane_item(
        &mut self,
        _active_pane_item: Option<&dyn ItemHandle>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.notify();
    }

    fn hide_setting(&self, _: &App) -> Option<HideStatusItem> {
        Some(HideStatusItem::new(|settings| {
            settings
                .status_bar
                .get_or_insert_default()
                .task_runner_button = Some(false);
        }))
    }
}
