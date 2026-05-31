//! LAN-based peer-to-peer collaboration for Zed.
//!
//! This module provides service discovery via UDP broadcast and direct TCP
//! connections between Zed instances on the same local network, integrating
//! with the existing RPC `Peer` infrastructure.

mod discovery;
mod transport;

use anyhow::Result;
use client::Client;
use discovery::{DiscoveryEvent, LanDiscoveryService};
use futures::channel::{mpsc, oneshot};
use futures::{SinkExt, StreamExt};
use gpui::{App, AppContext, AsyncApp, Context, Entity, EventEmitter, Global, Task};
use project::Project;
use rand::Rng;
use rpc::ConnectionId;
use rpc::proto::{self, PeerId, TypedEnvelope};
use std::collections::HashMap;
use std::sync::Arc;
use transport::LanNetworkRequest;
use util::ResultExt;

pub use discovery::{LanDiscoveredPeer, local_addresses};
pub use transport::{accept_connection, bind_listener, connect_to_peer, dial_peer};

/// Events emitted by the LAN collaboration manager.
#[derive(Debug, Clone)]
pub enum LanCollabEvent {
    /// A new peer was discovered on the local network.
    PeerDiscovered(LanDiscoveredPeer),
    /// A previously-discovered peer has gone away.
    PeerLost { peer_name: String },
    /// A LAN peer connection was fully established (Hello exchanged).
    PeerConnected { peer_name: String, peer_id: PeerId },
    /// A LAN peer connection was lost.
    PeerDisconnected { peer_name: String },
    /// A LAN peer shared a project with us (JoinProjectResponse payload).
    ProjectShared {
        peer_name: String,
        project_id: u64,
        response_payload: proto::JoinProjectResponse,
    },
}

impl EventEmitter<LanCollabEvent> for LanCollabManager {}

/// Global accessor for the LAN collaboration manager.
struct GlobalLanCollabManager(Entity<LanCollabManager>);

impl Global for GlobalLanCollabManager {}

/// Manages LAN peer discovery and connections.
pub struct LanCollabManager {
    client: Arc<Client>,
    peer_id: PeerId,
    /// Active LAN connections: peer_name → ConnectionId for direct messaging.
    connected_peers: HashMap<String, ConnectionId>,
    /// Channel to send dial requests to the background Tokio runtime.
    network_tx: mpsc::Sender<LanNetworkRequest>,
}

impl LanCollabManager {
    /// Initializes LAN collaboration: starts UDP discovery/advertising and
    /// binds a TCP listener for incoming connections.
    pub fn init(client: Arc<Client>, cx: &mut App) {
        let peer_id = generate_peer_id();
        let display_name = hostname();
        let (network_tx, network_rx) = mpsc::channel::<LanNetworkRequest>(32);
        let (discovery_tx, mut discovery_rx) = mpsc::unbounded::<DiscoveryEvent>();
        let (incoming_tx, mut incoming_rx) = mpsc::unbounded::<(rpc::Connection, String)>();

        let manager = cx.new(|cx| {
            let peer = client.peer().clone();
            let client_for_session = client.clone();
            let weak_self = cx.weak_entity();

            // Background: Tokio networking runtime
            cx.background_spawn(run_network(
                network_rx,
                display_name,
                peer_id,
                discovery_tx,
                incoming_tx,
            ))
            .detach();

            // Foreground: process incoming connections
            cx.spawn({
                let peer = peer.clone();
                let client = client_for_session.clone();
                let weak_self = weak_self.clone();
                async move |_this, cx| {
                    while let Some((connection, label)) = incoming_rx.next().await {
                        log::info!("accepted LAN connection from {}", label);
                        lan_peer_session(
                            &peer, &client, connection, peer_id, label, &weak_self, cx,
                        )
                        .await;
                    }
                }
            })
            .detach();

            // Foreground: process discovery events
            cx.spawn(async move |this, cx| {
                while let Some(event) = discovery_rx.next().await {
                    match event {
                        DiscoveryEvent::PeerDiscovered(peer) => {
                            this.update(cx, |_this, cx| {
                                cx.emit(LanCollabEvent::PeerDiscovered(peer));
                            })?;
                        }
                        DiscoveryEvent::PeerLost { name } => {
                            this.update(cx, |_this, cx| {
                                cx.emit(LanCollabEvent::PeerLost { peer_name: name });
                            })?;
                        }
                    }
                }
                anyhow::Ok(())
            })
            .detach();

            LanCollabManager {
                client,
                peer_id,
                connected_peers: HashMap::default(),
                network_tx,
            }
        });

        cx.set_global(GlobalLanCollabManager(manager));
    }

    /// Returns the global LAN collaboration manager.
    pub fn global(cx: &App) -> Option<Entity<LanCollabManager>> {
        cx.try_global::<GlobalLanCollabManager>()
            .map(|g| g.0.clone())
    }

    /// This instance's LAN peer ID.
    pub fn peer_id(&self) -> PeerId {
        self.peer_id
    }

    /// Trigger an active re-scan for LAN peers.
    pub fn refresh_discovery(&self, cx: &mut App) {
        let mut tx = self.network_tx.clone();
        cx.background_spawn(async move {
            let _ = tx.send(LanNetworkRequest::RefreshDiscovery).await;
        })
        .detach();
    }

    /// Probe a specific IP address for a LAN peer.
    pub fn probe_peer(&self, ip: &str, cx: &mut App) {
        let mut tx = self.network_tx.clone();
        let ip = ip.to_string();
        cx.background_spawn(async move {
            let _ = tx.send(LanNetworkRequest::ProbePeer { ip }).await;
        })
        .detach();
    }

    /// Initiates an outgoing connection to a discovered peer.
    pub fn connect_to_peer(
        &self,
        address: &str,
        peer_name: &str,
        cx: &mut Context<Self>,
    ) -> Task<Result<()>> {
        let address = address.to_string();
        let peer_name = peer_name.to_string();
        let peer = self.client.peer().clone();
        let client = self.client.clone();
        let my_peer_id = self.peer_id;
        let weak_self = cx.weak_entity();
        let mut network_tx = self.network_tx.clone();

        cx.spawn(async move |_this, cx| {
            let (tx, rx) = oneshot::channel();
            network_tx
                .send(LanNetworkRequest::DialPeer {
                    address,
                    response_tx: tx,
                })
                .await?;

            let connection = rx.await??;
            lan_peer_session(
                &peer, &client, connection, my_peer_id, peer_name, &weak_self, cx,
            )
            .await;
            anyhow::Ok(())
        })
    }

    /// Share the given project with a connected LAN peer.
    pub fn share_project_with_peer(
        &mut self,
        peer_name: &str,
        project: Entity<Project>,
        cx: &mut Context<Self>,
    ) -> Task<Result<()>> {
        let Some(conn_id) = self.connected_peers.get(peer_name).copied() else {
            return Task::ready(Err(anyhow::anyhow!("peer not connected: {}", peer_name)));
        };

        let peer = self.client.peer().clone();
        let peer_name = peer_name.to_string();

        cx.spawn(async move |_this, cx| {
            let response = project.update(cx, |project, cx| {
                let project_id = if let Some(id) = project.remote_id() {
                    id
                } else {
                    let mut rng = rand::rng();
                    rng.random::<u64>()
                };

                project.shared(project_id, cx)?;

                anyhow::Ok(proto::JoinProjectResponse {
                    project_id,
                    worktrees: project.worktree_metadata_protos(cx),
                    replica_id: 0,
                    collaborators: Vec::new(),
                    language_servers: Vec::new(),
                    language_server_capabilities: Vec::new(),
                    role: proto::ChannelRole::Admin.into(),
                    windows_paths: false,
                    features: Vec::new(),
                })
            })?;

            let project_id = response.project_id;
            peer.send(conn_id, response)?;
            log::info!("shared project {} with LAN peer {}", project_id, peer_name,);
            anyhow::Ok(())
        })
    }
}

/// Runs the Tokio networking runtime on the current (background) thread.
async fn run_network(
    mut network_rx: mpsc::Receiver<LanNetworkRequest>,
    display_name: String,
    peer_id: PeerId,
    discovery_tx: mpsc::UnboundedSender<DiscoveryEvent>,
    incoming_tx: mpsc::UnboundedSender<(rpc::Connection, String)>,
) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => {
            log::error!("failed to create Tokio runtime for LAN: {e:#}");
            return;
        }
    };

    runtime.block_on(async {
        let (listener, tcp_port) = match transport::bind_listener(0).await {
            Ok(v) => v,
            Err(e) => {
                log::error!("failed to bind LAN TCP listener: {e:#}");
                return;
            }
        };
        log::info!("LAN collab TCP listener on port {}", tcp_port);

        let (service, mut events) =
            match LanDiscoveryService::new(&display_name, peer_id, tcp_port).await {
                Ok(v) => v,
                Err(e) => {
                    log::error!("failed to start LAN discovery: {e:#}");
                    return;
                }
            };
        let service = Arc::new(service);

        let discovery_tx_clone = discovery_tx.clone();
        let discovery_handle = tokio::spawn(async move {
            while let Some(event) = events.next().await {
                if discovery_tx_clone.unbounded_send(event).is_err() {
                    break;
                }
            }
        });

        let incoming_tx_clone = incoming_tx.clone();
        let accept_handle = tokio::spawn(async move {
            loop {
                match transport::accept_connection(&listener).await {
                    Ok((connection, peer_addr)) => {
                        if incoming_tx_clone
                            .unbounded_send((connection, peer_addr.to_string()))
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(e) => {
                        log::error!("accept error: {}", e);
                        break;
                    }
                }
            }
        });

        let service_for_dial = service.clone();
        let dial_handle = tokio::spawn(async move {
            while let Some(request) = network_rx.next().await {
                match request {
                    LanNetworkRequest::DialPeer {
                        address,
                        response_tx,
                    } => {
                        let result = transport::dial_peer(&address).await;
                        let _ = response_tx.send(result);
                    }
                    LanNetworkRequest::RefreshDiscovery => {
                        let _ = service_for_dial.query().await;
                    }
                    LanNetworkRequest::ProbePeer { ip } => {
                        let _ = service_for_dial.probe_peer(&ip).await;
                    }
                }
            }
        });

        tokio::select! {
            _ = discovery_handle => {},
            _ = accept_handle => {},
            _ = dial_handle => {},
        }

        drop(service);
    });
}

/// Runs a full LAN peer session on the foreground:
/// 1. Adds the connection to the RPC Peer
/// 2. Sends our Hello
/// 3. Waits for the peer's Hello
/// 4. Dispatches incoming messages through Client
/// 5. Emits connect/disconnect events
async fn lan_peer_session(
    peer: &Arc<rpc::Peer>,
    client: &Arc<Client>,
    connection: rpc::Connection,
    my_peer_id: PeerId,
    peer_label: String,
    weak_manager: &gpui::WeakEntity<LanCollabManager>,
    cx: &mut AsyncApp,
) {
    let (conn_id, handle_io, mut incoming) = peer.add_connection(connection, {
        let executor = cx.background_executor().clone();
        move |duration| executor.timer(duration)
    });

    // Spawn I/O handler on background
    cx.background_executor().spawn(handle_io).detach();

    // Send our Hello
    let hello = proto::Hello {
        peer_id: Some(my_peer_id),
    };
    if let Err(e) = peer.send(conn_id, hello) {
        log::error!("failed to send LAN Hello: {}", e);
        return;
    }

    // Wait for the peer's Hello
    let remote_peer_id = match incoming.next().await {
        Some(message) => {
            let type_name = message.payload_type_name().to_string();
            match message.into_any().downcast::<TypedEnvelope<proto::Hello>>() {
                Ok(hello) => match hello.payload.peer_id {
                    Some(id) => {
                        log::info!(
                            "received Hello from LAN peer {} (peer_id: {:?})",
                            peer_label,
                            id,
                        );
                        id
                    }
                    None => {
                        log::error!("LAN peer {} sent Hello without peer_id", peer_label);
                        return;
                    }
                },
                Err(_) => {
                    log::error!(
                        "expected Hello from LAN peer {}, got {}",
                        peer_label,
                        type_name,
                    );
                    return;
                }
            }
        }
        None => {
            log::error!("LAN peer {} disconnected before Hello", peer_label);
            return;
        }
    };

    // Emit connected event and store conn_id for direct messaging
    weak_manager
        .update(cx, |manager, cx| {
            manager.connected_peers.insert(peer_label.clone(), conn_id);
            cx.emit(LanCollabEvent::PeerConnected {
                peer_name: peer_label.clone(),
                peer_id: remote_peer_id,
            });
        })
        .log_err();

    log::info!(
        "LAN session established with {} (remote peer_id: {:?}, conn_id: {})",
        peer_label,
        remote_peer_id,
        conn_id,
    );

    // Dispatch incoming messages
    while let Some(message) = incoming.next().await {
        // Intercept JoinProjectResponse for LAN project sharing.
        if message.payload_type_name() == "JoinProjectResponse" {
            if let Ok(envelope) = message
                .into_any()
                .downcast::<TypedEnvelope<proto::JoinProjectResponse>>()
            {
                let project_id = envelope.payload.project_id;
                let payload = envelope.payload;
                weak_manager
                    .update(cx, |_manager, cx| {
                        cx.emit(LanCollabEvent::ProjectShared {
                            peer_name: peer_label.clone(),
                            project_id,
                            response_payload: payload,
                        });
                    })
                    .log_err();
            } else {
                log::error!("Failed to downcast JoinProjectResponse despite type name match");
            }
            continue;
        }
        client.dispatch_lan_message(message, cx);
        smol::future::yield_now().await;
    }

    // Connection ended
    log::info!("LAN peer {} disconnected", peer_label);
    weak_manager
        .update(cx, |manager, cx| {
            manager.connected_peers.remove(&peer_label);
            cx.emit(LanCollabEvent::PeerDisconnected {
                peer_name: peer_label,
            });
        })
        .log_err();
}

fn hostname() -> String {
    smol::block_on(async {
        smol::unblock(move || {
            hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".to_string())
        })
        .await
    })
}

/// Generate a random PeerId for LAN identity.
///
/// Uses epoch 1 to distinguish LAN-assigned IDs from server-assigned IDs
/// (which use epoch 0).
fn generate_peer_id() -> PeerId {
    let mut rng = rand::rng();
    PeerId {
        owner_id: 1,
        id: rng.random(),
    }
}

/// Check whether a LAN peer (identified by name) has been trusted.
pub fn is_peer_trusted(peer_name: &str, cx: &App) -> bool {
    db::kvp::KeyValueStore::global(cx)
        .scoped(TRUSTED_PEERS_NAMESPACE)
        .read(peer_name)
        .ok()
        .flatten()
        .is_some()
}

/// Persist trust for a LAN peer so it won't prompt again on future
/// connections.
pub fn trust_peer(peer_name: &str, cx: &mut App) -> Task<anyhow::Result<()>> {
    let kvp = db::kvp::KeyValueStore::global(cx);
    let peer_name = peer_name.to_string();
    cx.background_spawn(async move {
        kvp.scoped(TRUSTED_PEERS_NAMESPACE)
            .write(peer_name, "1".to_string())
            .await
    })
}

/// Remove trust for a LAN peer.
pub fn untrust_peer(peer_name: &str, cx: &mut App) -> Task<anyhow::Result<()>> {
    let kvp = db::kvp::KeyValueStore::global(cx);
    let peer_name = peer_name.to_string();
    cx.background_spawn(async move { kvp.scoped(TRUSTED_PEERS_NAMESPACE).delete(peer_name).await })
}

const TRUSTED_PEERS_NAMESPACE: &str = "lan_trusted_peers";
