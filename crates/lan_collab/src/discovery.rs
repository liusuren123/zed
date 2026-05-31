//! LAN peer discovery via UDP broadcast.
//!
//! Periodically broadcasts this instance's presence on a well-known
//! UDP port and listens for broadcasts from other instances on the
//! same local network segment.

use anyhow::{Context as _, Result};
use futures::SinkExt;
use futures::channel::mpsc;
use rpc::proto::PeerId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;

const DISCOVERY_PORT: u16 = 45678;
const BROADCAST_ADDR: Ipv4Addr = Ipv4Addr::new(255, 255, 255, 255);
const ADVERTISE_INTERVAL: Duration = Duration::from_secs(5);
/// How long before a peer is considered lost if no new broadcast is received.
const PEER_EXPIRY: Duration = Duration::from_secs(15);

/// Information about a peer discovered on the LAN.
#[derive(Debug, Clone)]
pub struct LanDiscoveredPeer {
    /// Human-readable name of the peer (hostname).
    pub name: String,
    /// The peer's self-assigned identifier.
    pub peer_id: PeerId,
    /// Addresses where this peer can be reached.
    pub addresses: Vec<SocketAddr>,
}

/// Events from the discovery service.
#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    /// A new peer was discovered or its info was updated.
    PeerDiscovered(LanDiscoveredPeer),
    /// A previously-discovered peer has expired.
    PeerLost { name: String },
}

/// Payload broadcast over UDP for peer discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiscoveryPayload {
    name: String,
    peer_id_owner: u32,
    peer_id_id: u32,
    tcp_port: u16,
}

/// A simple UDP broadcast-based peer discovery service.
pub struct LanDiscoveryService {
    #[allow(dead_code)]
    socket: Arc<UdpSocket>,
}

impl LanDiscoveryService {
    /// Creates a new discovery service.
    pub async fn new(
        display_name: &str,
        peer_id: PeerId,
        tcp_port: u16,
    ) -> Result<(Self, mpsc::Receiver<DiscoveryEvent>)> {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), DISCOVERY_PORT);
        let socket = match UdpSocket::bind(addr).await {
            Ok(s) => s,
            Err(e) => {
                log::warn!(
                    "UDP discovery port {DISCOVERY_PORT} unavailable ({e}). LAN discovery disabled."
                );
                return Ok((
                    Self {
                        socket: Arc::new(UdpSocket::bind("0.0.0.0:0").await?),
                    },
                    mpsc::channel::<DiscoveryEvent>(1).1,
                ));
            }
        };
        socket
            .set_broadcast(true)
            .context("failed to set broadcast on discovery socket")?;

        let socket = Arc::new(socket);
        let (event_tx, event_rx) = mpsc::channel::<DiscoveryEvent>(64);

        // Track last-seen timestamps for peer expiry.
        let last_seen: Arc<std::sync::Mutex<HashMap<String, Instant>>> =
            Arc::new(std::sync::Mutex::new(HashMap::new()));

        let payload = DiscoveryPayload {
            name: display_name.to_string(),
            peer_id_owner: peer_id.owner_id,
            peer_id_id: peer_id.id,
            tcp_port,
        };
        let payload_bytes = Arc::new(serde_json::to_vec(&payload)?);

        // Spawn the listener
        let listener_socket = socket.clone();
        let mut listener_events = event_tx.clone();
        let listener_last_seen = last_seen.clone();
        let listener_payload = payload_bytes.clone();
        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            loop {
                match listener_socket.recv_from(&mut buf).await {
                    Ok((len, src)) => {
                        // Handle direct query — respond with our own discovery payload.
                        if &buf[..len] == b"query" {
                            let _ = listener_socket.send_to(&listener_payload, src).await;
                            continue;
                        }
                        if let Ok(payload) = serde_json::from_slice::<DiscoveryPayload>(&buf[..len])
                        {
                            let peer_id = PeerId {
                                owner_id: payload.peer_id_owner,
                                id: payload.peer_id_id,
                            };
                            let name = payload.name.clone();
                            let peer = LanDiscoveredPeer {
                                name: name.clone(),
                                peer_id,
                                addresses: vec![SocketAddr::new(src.ip(), payload.tcp_port)],
                            };
                            if let Ok(mut seen) = listener_last_seen.lock() {
                                seen.insert(name, Instant::now());
                            }
                            let _ = listener_events
                                .send(DiscoveryEvent::PeerDiscovered(peer))
                                .await;
                        }
                    }
                    Err(e) => {
                        log::error!("discovery listener error: {}", e);
                        break;
                    }
                }
            }
        });

        // Spawn the advertiser
        let advertiser_socket = socket.clone();
        let broadcast_addr = SocketAddr::new(IpAddr::V4(BROADCAST_ADDR), DISCOVERY_PORT);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(ADVERTISE_INTERVAL);
            loop {
                interval.tick().await;
                let _ = advertiser_socket
                    .send_to(&payload_bytes, &broadcast_addr)
                    .await;
            }
        });

        // Spawn expiry checker: periodically emits PeerLost for stale peers.
        let mut expiry_events = event_tx.clone();
        let expiry_last_seen = last_seen.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                let now = Instant::now();
                let expired: Vec<String> = {
                    let seen = match expiry_last_seen.lock() {
                        Ok(s) => s,
                        Err(_) => break,
                    };
                    seen.iter()
                        .filter(|(_, t)| now.duration_since(**t) > PEER_EXPIRY)
                        .map(|(name, _)| name.clone())
                        .collect()
                };
                for name in expired {
                    if let Ok(mut seen) = expiry_last_seen.lock() {
                        seen.remove(&name);
                    }
                    if expiry_events
                        .send(DiscoveryEvent::PeerLost { name })
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
            }
        });

        Ok((Self { socket }, event_rx))
    }

    /// Sends an immediate broadcast query to solicit responses from peers.
    #[allow(dead_code)]
    pub async fn query(&self) -> Result<()> {
        let broadcast_addr = SocketAddr::new(IpAddr::V4(BROADCAST_ADDR), DISCOVERY_PORT);
        self.socket.send_to(b"query", &broadcast_addr).await?;
        Ok(())
    }

    /// Probe a specific IP address for a LAN peer.
    pub async fn probe_peer(&self, ip: &str) -> Result<()> {
        let addr: SocketAddr = format!("{}:{}", ip, DISCOVERY_PORT)
            .parse()
            .context("invalid IP address")?;
        self.socket.send_to(b"query", &addr).await?;
        log::info!("probed LAN peer at {}", addr);
        Ok(())
    }

    /// Shuts down the discovery service gracefully.
    #[allow(dead_code)]
    pub async fn shutdown(self) {
        drop(self.socket);
    }
}

/// Gets the IPv4 addresses of local network interfaces (excluding loopback).
pub fn local_addresses() -> Vec<Ipv4Addr> {
    let mut addrs = Vec::new();
    if let Ok(hostname) = hostname::get() {
        if let Ok(host_str) = hostname.into_string() {
            if let Ok(iter) = std::net::ToSocketAddrs::to_socket_addrs(&format!("{}:0", host_str)) {
                for addr in iter {
                    if let IpAddr::V4(ip) = addr.ip() {
                        if !ip.is_loopback() && !ip.is_link_local() {
                            addrs.push(ip);
                        }
                    }
                }
            }
        }
    }
    addrs
}
