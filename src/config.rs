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

    pub fn create_default_config() -> Self {
        Config {
            global: Some(GlobalConfig {
                log_level: Some("info".to_string()),
                buffer_size: Some(8192),
            }),
            tcp: Some(vec![
                TcpRule {
                    bind_addr: "127.0.0.1".to_string(),
                    bind_port: 8080,
                    target_addr: "127.0.0.1".to_string(),
                    target_port: 80,
                    name: Some("web_proxy_example".to_string()),
                },
                TcpRule {
                    bind_addr: "127.0.0.1".to_string(),
                    bind_port: 2222,
                    target_addr: "127.0.0.1".to_string(),
                    target_port: 22,
                    name: Some("ssh_proxy_example".to_string()),
                },
            ]),
            udp: Some(vec![
                UdpRule {
                    bind_addr: "127.0.0.1".to_string(),
                    bind_port: 5353,
                    target_addr: "8.8.8.8".to_string(),
                    target_port: 53,
                    name: Some("dns_proxy_example".to_string()),
                    timeout: Some(30),
                },
            ]),
        }
    }

    pub fn save_to_file(&self, path: &str) -> anyhow::Result<()> {
        let content = self.to_toml_with_comments();
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn to_toml_with_comments(&self) -> String {
        let mut content = String::new();
        
        content.push_str("# Porture Configuration File\n");
        content.push_str("# TCP/UDP Port Forwarding Tool\n");
        content.push_str("# \n");
        content.push_str("# This configuration was automatically generated.\n");
        content.push_str("# Please edit it to suit your needs.\n");
        content.push_str("# \n");
        content.push_str("# For more examples and documentation, visit:\n");
        content.push_str("# https://github.com/AptS-1547/porture\n\n");

        content.push_str("# Global settings\n");
        content.push_str("[global]\n");
        content.push_str("# Log level: error, warn, info, debug, trace\n");
        if let Some(ref global) = self.global {
            if let Some(ref log_level) = global.log_level {
                content.push_str(&format!("log_level = \"{}\"\n", log_level));
            }
            content.push_str("# Buffer size for data transfer (in bytes)\n");
            if let Some(buffer_size) = global.buffer_size {
                content.push_str(&format!("buffer_size = {}\n", buffer_size));
            }
        }
        content.push_str("\n");

        if let Some(ref tcp_rules) = self.tcp {
            content.push_str("# TCP forwarding rules\n");
            for rule in tcp_rules {
                content.push_str("[[tcp]]\n");
                content.push_str("# Local address to bind to (use \"0.0.0.0\" for all interfaces)\n");
                content.push_str(&format!("bind_addr = \"{}\"\n", rule.bind_addr));
                content.push_str("# Local port to bind to\n");
                content.push_str(&format!("bind_port = {}\n", rule.bind_port));
                content.push_str("# Target address to forward to\n");
                content.push_str(&format!("target_addr = \"{}\"\n", rule.target_addr));
                content.push_str("# Target port to forward to\n");
                content.push_str(&format!("target_port = {}\n", rule.target_port));
                if let Some(ref name) = rule.name {
                    content.push_str("# Optional: rule name for logging\n");
                    content.push_str(&format!("name = \"{}\"\n", name));
                }
                content.push_str("\n");
            }
        }

        if let Some(ref udp_rules) = self.udp {
            content.push_str("# UDP forwarding rules\n");
            for rule in udp_rules {
                content.push_str("[[udp]]\n");
                content.push_str("# Local address to bind to (use \"0.0.0.0\" for all interfaces)\n");
                content.push_str(&format!("bind_addr = \"{}\"\n", rule.bind_addr));
                content.push_str("# Local port to bind to\n");
                content.push_str(&format!("bind_port = {}\n", rule.bind_port));
                content.push_str("# Target address to forward to\n");
                content.push_str(&format!("target_addr = \"{}\"\n", rule.target_addr));
                content.push_str("# Target port to forward to\n");
                content.push_str(&format!("target_port = {}\n", rule.target_port));
                if let Some(ref name) = rule.name {
                    content.push_str("# Optional: rule name for logging\n");
                    content.push_str(&format!("name = \"{}\"\n", name));
                }
                content.push_str("# UDP session timeout in seconds\n");
                if let Some(timeout) = rule.timeout {
                    content.push_str(&format!("timeout = {}\n", timeout));
                }
                content.push_str("\n");
            }
        }

        content
    }

    pub fn from_file_or_create_default(path: &str) -> anyhow::Result<Self> {
        match std::fs::metadata(path) {
            Ok(_) => {
                // 文件存在，直接读取
                Self::from_file(path)
            }
            Err(_) => {
                // 文件不存在，创建默认配置
                let default_config = Self::create_default_config();
                default_config.save_to_file(path)?;
                Ok(default_config)
            }
        }
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
