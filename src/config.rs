use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub global: Option<GlobalConfig>,
    pub tcp: Option<Vec<TcpRule>>,
    pub udp: Option<Vec<UdpRule>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GlobalConfig {
    pub log_level: Option<String>,
    pub buffer_size: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TcpRule {
    pub bind_addr: String,
    pub bind_port: u16,
    pub target_addr: String,
    pub target_port: u16,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UdpRule {
    pub bind_addr: String,
    pub bind_port: u16,
    pub target_addr: String,
    pub target_port: u16,
    pub name: Option<String>,
    pub timeout: Option<u64>,
}

impl Config {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if let Some(tcp_rules) = &self.tcp {
            for rule in tcp_rules {
                rule.validate()?;
            }
        }

        if let Some(udp_rules) = &self.udp {
            for rule in udp_rules {
                rule.validate()?;
            }
        }

        Ok(())
    }
}

impl TcpRule {
    pub fn bind_socket_addr(&self) -> anyhow::Result<SocketAddr> {
        let ip = IpAddr::from_str(&self.bind_addr)?;
        Ok(SocketAddr::new(ip, self.bind_port))
    }

    pub fn target_socket_addr(&self) -> anyhow::Result<SocketAddr> {
        let ip = IpAddr::from_str(&self.target_addr)?;
        Ok(SocketAddr::new(ip, self.target_port))
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        self.bind_socket_addr()?;
        self.target_socket_addr()?;
        Ok(())
    }

    pub fn rule_name(&self) -> String {
        self.name.clone().unwrap_or_else(|| {
            format!("tcp_{}:{}_to_{}:{}", 
                self.bind_addr, self.bind_port,
                self.target_addr, self.target_port)
        })
    }
}

impl UdpRule {
    pub fn bind_socket_addr(&self) -> anyhow::Result<SocketAddr> {
        let ip = IpAddr::from_str(&self.bind_addr)?;
        Ok(SocketAddr::new(ip, self.bind_port))
    }

    pub fn target_socket_addr(&self) -> anyhow::Result<SocketAddr> {
        let ip = IpAddr::from_str(&self.target_addr)?;
        Ok(SocketAddr::new(ip, self.target_port))
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        self.bind_socket_addr()?;
        self.target_socket_addr()?;
        Ok(())
    }

    pub fn rule_name(&self) -> String {
        self.name.clone().unwrap_or_else(|| {
            format!("udp_{}:{}_to_{}:{}", 
                self.bind_addr, self.bind_port,
                self.target_addr, self.target_port)
        })
    }

    pub fn timeout_seconds(&self) -> u64 {
        self.timeout.unwrap_or(30)
    }
}
