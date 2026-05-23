//! Cross-workspace awareness bridge between agent tools and the UI workspace state.

use std::sync::Arc;

use agent::{
    GlobalWorkspaceRegistry, WorkspaceInfo, WorkspaceMessageSender, WorkspaceRefreshFn,
    init_workspace_registry, update_workspace_snapshots,
};
use collections::HashMap;
use gpui::{App, Task, WeakEntity};
use workspace::Workspace;

use crate::AgentPanel;

#[derive(Default, Clone)]
struct WorkspaceHandleMap(Arc<std::sync::Mutex<HashMap<String, WeakEntity<Workspace>>>>);

impl gpui::Global for WorkspaceHandleMap {}

pub(crate) fn ensure_workspace_registry(cx: &mut App) {
    if cx.has_global::<GlobalWorkspaceRegistry>() {
        return;
    }

    if !cx.has_global::<WorkspaceHandleMap>() {
        cx.set_global(WorkspaceHandleMap::default());
    }

    let handle_map = cx.global::<WorkspaceHandleMap>().0.clone();

    let refresh_fn: WorkspaceRefreshFn = Arc::new({
        let handle_map = handle_map.clone();
        move |cx: &mut App| {
            let workspace_handles: Vec<(String, WeakEntity<Workspace>)> = {
                let map = handle_map.lock().unwrap();
                map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            };

            let mut snapshots = Vec::new();
            for (id, ws_handle) in workspace_handles {
                let Some(workspace_entity) = ws_handle.upgrade() else {
                    continue;
                };
                let snapshot = workspace_entity.read_with(cx, |workspace, cx| {
                    let paths: Vec<String> = workspace
                        .worktrees(cx)
                        .map(|wt| wt.read(cx).abs_path().to_string_lossy().to_string())
                        .collect();

                    let active_item_path = workspace
                        .active_item(cx)
                        .and_then(|item| item.project_path(cx))
                        .map(|p| p.path.as_ref().as_std_path().to_string_lossy().to_string());

                    let (agent_status, active_thread_title) = workspace
                        .panel::<AgentPanel>(cx)
                        .map(|panel| {
                            let panel = panel.read(cx);
                            let title = panel
                                .active_conversation_view()
                                .and_then(|cv| cv.read(cx).root_thread_view())
                                .and_then(|tv| {
                                    tv.read(cx).thread.read(cx).title().map(|t| t.to_string())
                                });
                            let status = if panel
                                .active_agent_thread(cx)
                                .map(|thread| {
                                    matches!(
                                        thread.read(cx).status(),
                                        acp_thread::ThreadStatus::Generating
                                    )
                                })
                                .unwrap_or(false)
                            {
                                "working".to_string()
                            } else if panel.active_agent_thread(cx).is_some() {
                                "idle".to_string()
                            } else {
                                "no active thread".to_string()
                            };
                            (status, title)
                        })
                        .unwrap_or_else(|| ("no agent panel".to_string(), None));

                    let is_active = workspace
                        .multi_workspace()
                        .and_then(|mw| mw.upgrade())
                        .map(|mw| {
                            let mw = mw.read(cx);
                            mw.workspace().entity_id() == workspace_entity.entity_id()
                        })
                        .unwrap_or(false);

                    WorkspaceInfo {
                        id,
                        paths,
                        active_item_path,
                        agent_status,
                        is_active,
                        active_thread_title,
                    }
                });
                snapshots.push(snapshot);
            }
            update_workspace_snapshots(snapshots, cx);
        }
    });

    let send_message_fn: WorkspaceMessageSender =
        Arc::new(move |target_id: String, message: String, cx: &mut App| {
            let handle_map = handle_map.clone();
            let workspace_handle = {
                let map = handle_map.lock().unwrap();
                map.get(&target_id).cloned()
            };

            let Some(workspace_handle) = workspace_handle else {
                return Task::ready(Err(anyhow::anyhow!("Workspace not found: {target_id}")));
            };

            cx.spawn(async move |cx| {
                let Some(workspace) = workspace_handle.upgrade() else {
                    return Err(anyhow::anyhow!("Workspace dropped"));
                };

                let update_result: anyhow::Result<()> = cx.update(|cx| {
                    match workspace.update(cx, |workspace, cx| {
                        let Some(panel) = workspace.panel::<AgentPanel>(cx) else {
                            return Err(anyhow::anyhow!("No agent panel in workspace"));
                        };

                        panel.update(cx, |panel, cx| {
                            panel.receive_workspace_message(message.clone(), cx);
                        });

                        Ok(())
                    }) {
                        Ok(()) => Ok(()),
                        Err(e) => Err(e),
                    }
                });

                update_result
            })
        });

    init_workspace_registry(cx, send_message_fn, refresh_fn);
}

pub(crate) fn register_workspace_handle(id: String, handle: WeakEntity<Workspace>, cx: &App) {
    if let Some(map) = cx.try_global::<WorkspaceHandleMap>() {
        map.0.lock().unwrap().insert(id, handle);
    }
}

#[allow(dead_code)]
pub(crate) fn unregister_workspace_handle(id: &str, cx: &App) {
    if let Some(map) = cx.try_global::<WorkspaceHandleMap>() {
        map.0.lock().unwrap().remove(id);
    }
}
