//! Direct TCP transport for LAN peer connections.
//!
//! Converts raw TCP streams into the `WebSocketMessage`-based `Sink`/`Stream`
//! interface expected by `rpc::Connection`, using a simple frame-based protocol.

use anyhow::{Context as _, Result, anyhow};
use async_tungstenite::tungstenite::Message as WebSocketMessage;
use futures::channel::oneshot;
use futures::{Sink, Stream};
use rpc::Connection;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};

/// Request sent from the GPUI foreground to the Tokio networking thread.
pub enum LanNetworkRequest {
    /// Dial a peer at the given address, send the resulting Connection back.
    DialPeer {
        address: String,
        response_tx: oneshot::Sender<Result<Connection>>,
    },
    /// Trigger an immediate re-scan for LAN peers.
    RefreshDiscovery,
    /// Probe a specific IP address for a LAN peer.
    ProbePeer { ip: String },
}

/// Frame type markers.
const FRAME_BINARY: u8 = 0;
const FRAME_PING: u8 = 1;
const FRAME_PONG: u8 = 2;

/// Connects to a LAN peer at the given address.
///
/// Returns an `rpc::Connection` that can be passed to `Peer::add_connection()`.
pub async fn dial_peer(addr: &str) -> Result<Connection> {
    let stream = TcpStream::connect(addr)
        .await
        .context("failed to connect to LAN peer")?;
    stream.set_nodelay(true)?;
    Ok(tcp_to_connection(stream))
}

/// Alias for [`dial_peer`].
pub async fn connect_to_peer(addr: &str) -> Result<Connection> {
    dial_peer(addr).await
}

/// Binds a TCP listener for incoming LAN peer connections.
pub async fn bind_listener(port: u16) -> Result<(TcpListener, u16)> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr)
        .await
        .context("failed to bind TCP listener for LAN peers")?;
    let local_addr = listener.local_addr()?;
    Ok((listener, local_addr.port()))
}

/// Accepts one incoming connection and converts it to an `rpc::Connection`.
pub async fn accept_connection(listener: &TcpListener) -> Result<(Connection, SocketAddr)> {
    let (stream, peer_addr) = listener
        .accept()
        .await
        .context("failed to accept LAN connection")?;
    stream.set_nodelay(true)?;
    Ok((tcp_to_connection(stream), peer_addr))
}

/// Wraps a TCP stream as an `rpc::Connection`.
fn tcp_to_connection(stream: TcpStream) -> Connection {
    let (read_half, write_half) = stream.into_split();
    let framed = TcpFramed {
        read_half,
        write_half,
    };
    Connection::new(framed)
}

/// A framed transport over a split TCP stream.
struct TcpFramed {
    read_half: tokio::net::tcp::OwnedReadHalf,
    write_half: tokio::net::tcp::OwnedWriteHalf,
}

impl Stream for TcpFramed {
    type Item = Result<WebSocketMessage>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        let mut type_buf = [0u8; 1];
        let read_half = &mut self.read_half;

        // Poll for the frame type byte
        let type_result = {
            let mut read_fut = Box::pin(AsyncReadExt::read_exact(read_half, &mut type_buf));
            match read_fut.as_mut().poll(cx) {
                Poll::Ready(Ok(_n)) => Ok(type_buf[0]),
                Poll::Ready(Err(e)) => Err(e),
                Poll::Pending => return Poll::Pending,
            }
        };

        match type_result {
            Ok(FRAME_BINARY) => {
                let mut len_buf = [0u8; 4];
                let mut read_fut = Box::pin(AsyncReadExt::read_exact(read_half, &mut len_buf));
                let len = match read_fut.as_mut().poll(cx) {
                    Poll::Ready(Ok(_)) => u32::from_le_bytes(len_buf) as usize,
                    Poll::Ready(Err(e)) => return Poll::Ready(Some(Err(e.into()))),
                    Poll::Pending => return Poll::Pending,
                };

                if len > 16 * 1024 * 1024 {
                    return Poll::Ready(Some(Err(anyhow!("frame too large: {} bytes", len))));
                }

                let mut payload = vec![0u8; len];
                let mut read_fut = Box::pin(AsyncReadExt::read_exact(read_half, &mut payload));
                match read_fut.as_mut().poll(cx) {
                    Poll::Ready(Ok(_)) => {
                        Poll::Ready(Some(Ok(WebSocketMessage::Binary(payload.into()))))
                    }
                    Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e.into()))),
                    Poll::Pending => Poll::Pending,
                }
            }
            Ok(FRAME_PING) => Poll::Ready(Some(Ok(WebSocketMessage::Ping(Vec::new().into())))),
            Ok(FRAME_PONG) => Poll::Ready(Some(Ok(WebSocketMessage::Pong(Vec::new().into())))),
            Ok(_) => Poll::Ready(Some(Err(anyhow!("unknown frame type")))),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    Poll::Ready(None)
                } else {
                    Poll::Ready(Some(Err(e.into())))
                }
            }
        }
    }
}

impl Sink<WebSocketMessage> for TcpFramed {
    type Error = anyhow::Error;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(mut self: Pin<&mut Self>, item: WebSocketMessage) -> Result<()> {
        let write_half = &mut self.write_half;
        match item {
            WebSocketMessage::Binary(data) => {
                let len = data.len();
                let mut frame = Vec::with_capacity(1 + 4 + len);
                frame.push(FRAME_BINARY);
                frame.extend_from_slice(&(len as u32).to_le_bytes());
                frame.extend_from_slice(&data);

                match write_half.try_write(&frame) {
                    Ok(n) if n == frame.len() => Ok(()),
                    Ok(_) => Err(anyhow!("partial write")),
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        Err(anyhow!("would block"))
                    }
                    Err(e) => Err(e.into()),
                }
            }
            WebSocketMessage::Ping(_) => {
                write_half
                    .try_write(&[FRAME_PING])
                    .map_err(|e| anyhow!("ping write failed: {}", e))?;
                Ok(())
            }
            WebSocketMessage::Pong(_) => {
                write_half
                    .try_write(&[FRAME_PONG])
                    .map_err(|e| anyhow!("pong write failed: {}", e))?;
                Ok(())
            }
            WebSocketMessage::Close(_) => Ok(()),
            other => Err(anyhow!(
                "unsupported WebSocket message type: {:?}",
                std::mem::discriminant(&other)
            )),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }
}
