use crate::config::TcpRule;
use anyhow::Result;
use log::{error, info, debug};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub struct TcpForwarder {
    rule: TcpRule,
    buffer_size: usize,
}

impl TcpForwarder {
    pub fn new(rule: TcpRule, buffer_size: usize) -> Self {
        Self { rule, buffer_size }
    }

    pub async fn start(&self) -> Result<()> {
        let bind_addr = self.rule.bind_socket_addr()?;
        let listener = TcpListener::bind(bind_addr).await?;
        
        info!("TCP forwarder '{}' listening on {}", 
              self.rule.rule_name(), bind_addr);
        info!("TCP forwarding {} -> {}", 
              bind_addr, self.rule.target_socket_addr()?);

        loop {
            match listener.accept().await {
                Ok((client_stream, client_addr)) => {
                    debug!("New TCP connection from {}", client_addr);
                    
                    let rule = self.rule.clone();
                    let buffer_size = self.buffer_size;
                    
                    tokio::spawn(async move {
                        if let Err(e) = handle_tcp_client(client_stream, rule, buffer_size).await {
                            error!("TCP connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept TCP connection: {}", e);
                }
            }
        }
    }
}

async fn handle_tcp_client(
    mut client_stream: TcpStream,
    rule: TcpRule,
    buffer_size: usize,
) -> Result<()> {
    let target_addr = rule.target_socket_addr()?;
    
    // Connect to target server
    let mut target_stream = match TcpStream::connect(target_addr).await {
        Ok(stream) => stream,
        Err(e) => {
            error!("Failed to connect to target {}: {}", target_addr, e);
            return Err(e.into());
        }
    };

    debug!("Connected to target {}", target_addr);

    // Split streams for bidirectional forwarding
    let (mut client_read, mut client_write) = client_stream.split();
    let (mut target_read, mut target_write) = target_stream.split();

    // Forward data bidirectionally
    let client_to_target = async {
        let mut buffer = vec![0u8; buffer_size];
        loop {
            match client_read.read(&mut buffer).await {
                Ok(0) => break, // Connection closed
                Ok(n) => {
                    if let Err(e) = target_write.write_all(&buffer[..n]).await {
                        error!("Failed to write to target: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to read from client: {}", e);
                    break;
                }
            }
        }
    };

    let target_to_client = async {
        let mut buffer = vec![0u8; buffer_size];
        loop {
            match target_read.read(&mut buffer).await {
                Ok(0) => break, // Connection closed
                Ok(n) => {
                    if let Err(e) = client_write.write_all(&buffer[..n]).await {
                        error!("Failed to write to client: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to read from target: {}", e);
                    break;
                }
            }
        }
    };

    // Run both directions concurrently
    tokio::select! {
        _ = client_to_target => {},
        _ = target_to_client => {},
    }

    debug!("TCP connection closed");
    Ok(())
}
