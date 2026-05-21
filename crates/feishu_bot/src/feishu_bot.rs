pub mod api;
pub mod commands;
pub mod config;
pub mod message_handler;
pub mod session;
pub mod workspace_client;

use anyhow::Result;
use gpui::Entity;
use remote_server::HeadlessProject;
use rpc::AnyProtoClient;

/// Initialize the feishu bot subsystem in RPC mode.
///
/// The `client` must be an `AnyProtoClient` connected to a remote_server
/// instance that has registered the remote control message handlers.
pub fn init_rpc(config: config::FeishuBotConfig, client: AnyProtoClient) -> Result<MessageHandler> {
    log::info!(
        "Initializing Feishu Bot (RPC mode) with app_id: {}",
        config.app_id
    );
    Ok(MessageHandler::new(
        workspace_client::WorkspaceClient::new_rpc(client),
    ))
}

/// Initialize the feishu bot subsystem in local mode.
///
/// The `project` is the `HeadlessProject` entity to interact with directly.
/// This mode works within the same process as the remote_server.
pub fn init_local(
    config: config::FeishuBotConfig,
    project: Entity<HeadlessProject>,
) -> Result<MessageHandler> {
    log::info!(
        "Initializing Feishu Bot (local mode) with app_id: {}",
        config.app_id
    );
    Ok(MessageHandler::new(
        workspace_client::WorkspaceClient::new_local(project),
    ))
}

pub use message_handler::MessageHandler;
