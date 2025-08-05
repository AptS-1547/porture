use crate::config::UdpRule;
use anyhow::Result;
use log::{error, info, debug};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use tokio::time::{interval, timeout};

#[derive(Debug, Clone)]
struct UdpSession {
    target_socket: Arc<UdpSocket>,
    last_activity: Instant,
}

pub struct UdpForwarder {
    rule: UdpRule,
    buffer_size: usize,
}

impl UdpForwarder {
    pub fn new(rule: UdpRule, buffer_size: usize) -> Self {
        Self { rule, buffer_size }
    }

    pub async fn start(&self) -> Result<()> {
        let bind_addr = self.rule.bind_socket_addr()?;
        let target_addr = self.rule.target_socket_addr()?;
        
        let socket = UdpSocket::bind(bind_addr).await?;
        
        info!("UDP forwarder '{}' listening on {}", 
              self.rule.rule_name(), bind_addr);
        info!("UDP forwarding {} -> {}", 
              bind_addr, target_addr);

        // Session management
        let sessions: Arc<RwLock<HashMap<SocketAddr, UdpSession>>> = 
            Arc::new(RwLock::new(HashMap::new()));
        
        let socket = Arc::new(socket);
        let timeout_duration = Duration::from_secs(self.rule.timeout_seconds());
        
        // Start cleanup task
        let cleanup_sessions = sessions.clone();
        let cleanup_timeout = timeout_duration;
        tokio::spawn(async move {
            let mut cleanup_interval = interval(Duration::from_secs(30));
            loop {
                cleanup_interval.tick().await;
                cleanup_expired_sessions(cleanup_sessions.clone(), cleanup_timeout).await;
            }
        });

        // Main forwarding loop
        let mut buffer = vec![0u8; self.buffer_size];
        loop {
            match socket.recv_from(&mut buffer).await {
                Ok((len, client_addr)) => {
                    debug!("Received {} bytes from {}", len, client_addr);
                    
                    let data = buffer[..len].to_vec();
                    let socket_clone = socket.clone();
                    let sessions_clone = sessions.clone();
                    let rule_clone = self.rule.clone();
                    let buffer_size = self.buffer_size;
                    
                    tokio::spawn(async move {
                        if let Err(e) = handle_udp_packet(
                            socket_clone,
                            sessions_clone,
                            client_addr,
                            data,
                            rule_clone,
                            buffer_size,
                        ).await {
                            error!("UDP packet handling error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to receive UDP packet: {}", e);
                }
            }
        }
    }
}

async fn handle_udp_packet(
    client_socket: Arc<UdpSocket>,
    sessions: Arc<RwLock<HashMap<SocketAddr, UdpSession>>>,
    client_addr: SocketAddr,
    data: Vec<u8>,
    rule: UdpRule,
    buffer_size: usize,
) -> Result<()> {
    let target_addr = rule.target_socket_addr()?;
    
    // Get or create session
    let session = {
        let mut sessions_write = sessions.write().await;
        if let Some(session) = sessions_write.get_mut(&client_addr) {
            // Update last activity
            session.last_activity = Instant::now();
            session.clone()
        } else {
            // Create new session
            debug!("Creating new UDP session for {}", client_addr);
            
            let target_socket = UdpSocket::bind("0.0.0.0:0").await?;
            let target_socket = Arc::new(target_socket);
            
            let session = UdpSession {
                target_socket: target_socket.clone(),
                last_activity: Instant::now(),
            };
            
            sessions_write.insert(client_addr, session.clone());
            
            // Start response forwarding task
            let client_socket_clone = client_socket.clone();
            let target_socket_clone = target_socket.clone();
            let sessions_clone = sessions.clone();
            
            tokio::spawn(async move {
                if let Err(e) = forward_responses(
                    target_socket_clone,
                    client_socket_clone,
                    client_addr,
                    sessions_clone,
                    buffer_size,
                ).await {
                    error!("Response forwarding error: {}", e);
                }
            });
            
            session
        }
    };

    // Forward packet to target
    if let Err(e) = session.target_socket.send_to(&data, target_addr).await {
        error!("Failed to send to target {}: {}", target_addr, e);
        // Remove failed session
        sessions.write().await.remove(&client_addr);
    } else {
        debug!("Forwarded {} bytes to {}", data.len(), target_addr);
    }

    Ok(())
}

async fn forward_responses(
    target_socket: Arc<UdpSocket>,
    client_socket: Arc<UdpSocket>,
    client_addr: SocketAddr,
    sessions: Arc<RwLock<HashMap<SocketAddr, UdpSession>>>,
    buffer_size: usize,
) -> Result<()> {
    let mut buffer = vec![0u8; buffer_size];
    
    loop {
        match timeout(Duration::from_secs(60), target_socket.recv(&mut buffer)).await {
            Ok(Ok(len)) => {
                debug!("Received {} bytes from target, forwarding to {}", len, client_addr);
                
                // Update session activity
                if let Some(session) = sessions.write().await.get_mut(&client_addr) {
                    session.last_activity = Instant::now();
                } else {
                    // Session was removed, stop forwarding
                    break;
                }
                
                // Forward response to client
                if let Err(e) = client_socket.send_to(&buffer[..len], client_addr).await {
                    error!("Failed to send response to client {}: {}", client_addr, e);
                    break;
                }
            }
            Ok(Err(e)) => {
                error!("Target socket error: {}", e);
                break;
            }
            Err(_) => {
                // Timeout - check if session still exists
                if !sessions.read().await.contains_key(&client_addr) {
                    break;
                }
            }
        }
    }
    
    // Clean up session
    sessions.write().await.remove(&client_addr);
    debug!("UDP session for {} ended", client_addr);
    
    Ok(())
}

async fn cleanup_expired_sessions(
    sessions: Arc<RwLock<HashMap<SocketAddr, UdpSession>>>,
    timeout_duration: Duration,
) {
    let now = Instant::now();
    let mut expired_clients = Vec::new();
    
    {
        let sessions_read = sessions.read().await;
        for (client_addr, session) in sessions_read.iter() {
            if now.duration_since(session.last_activity) > timeout_duration {
                expired_clients.push(*client_addr);
            }
        }
    }
    
    if !expired_clients.is_empty() {
        let mut sessions_write = sessions.write().await;
        for client_addr in expired_clients {
            sessions_write.remove(&client_addr);
            debug!("Cleaned up expired UDP session for {}", client_addr);
        }
    }
}
