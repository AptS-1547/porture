# Porture

A minimal, programmable TCP/UDP port forwarder written in Rust, designed as a replacement for iptables NAT rules.

## Features

- **TCP Forwarding**: Forward TCP connections to remote servers
- **UDP Forwarding**: Forward UDP packets with session management
- **TOML Configuration**: Easy-to-understand configuration format
- **High Performance**: Built with Tokio for async I/O
- **Session Management**: Intelligent UDP session handling with timeouts
- **Logging**: Comprehensive logging with configurable levels
- **Signal Handling**: Graceful shutdown on SIGTERM/SIGINT

## Installation

```bash
cargo build --release
```

The binary will be available at `target/release/porture`.

## Configuration

Create a `config.toml` file with your forwarding rules:

```toml
# Global settings
[global]
log_level = "info"        # error, warn, info, debug, trace
buffer_size = 8192        # Buffer size for data transfer

# TCP forwarding rules
[[tcp]]
bind_addr = "0.0.0.0"     # Address to bind to
bind_port = 8080          # Port to bind to
target_addr = "127.0.0.1" # Target address
target_port = 80          # Target port
name = "web_proxy"        # Optional: rule name for logging

[[tcp]]
bind_addr = "0.0.0.0"
bind_port = 2222
target_addr = "192.168.1.100"
target_port = 22
name = "ssh_proxy"

# UDP forwarding rules
[[udp]]
bind_addr = "0.0.0.0"     # Address to bind to
bind_port = 5353          # Port to bind to
target_addr = "8.8.8.8"   # Target address
target_port = 53          # Target port
name = "dns_proxy"        # Optional: rule name for logging
timeout = 30              # UDP session timeout in seconds

[[udp]]
bind_addr = "0.0.0.0"
bind_port = 1194
target_addr = "vpn.example.com"
target_port = 1194
name = "vpn_proxy"
timeout = 60
```

## Usage

### Basic Usage

```bash
# Use default config.toml
./porture

# Specify custom config file
./porture -c /path/to/config.toml

# Set log level
./porture -l debug
```

### Command Line Options

```bash
./porture --help
```

```
A minimal, programmable port forwarder written in Rust

Usage: porture [OPTIONS]

Options:
  -c, --config <FILE>       Configuration file path [default: config.toml]
  -l, --log-level <LEVEL>   Log level (error, warn, info, debug, trace)
  -h, --help                Print help
  -V, --version             Print version
```

### Running as a Service

#### systemd (Linux)

Create `/etc/systemd/system/porture.service`:

```ini
[Unit]
Description=Porture Port Forwarder
After=network.target

[Service]
Type=simple
User=nobody
Group=nobody
ExecStart=/usr/local/bin/porture -c /etc/porture/config.toml
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl enable porture
sudo systemctl start porture
```

#### launchd (macOS)

Create `~/Library/LaunchAgents/com.example.porture.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.example.porture</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/porture</string>
        <string>-c</string>
        <string>/usr/local/etc/porture/config.toml</string>
    </array>
    <key>KeepAlive</key>
    <true/>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
```

Load and start:

```bash
launchctl load ~/Library/LaunchAgents/com.example.porture.plist
```

## Use Cases

### Replace iptables NAT Rules

Instead of using iptables:

```bash
# Old way with iptables
iptables -t nat -A PREROUTING -p tcp --dport 8080 -j DNAT --to-destination 192.168.1.100:80
iptables -t nat -A PREROUTING -p udp --dport 5353 -j DNAT --to-destination 8.8.8.8:53
```

Use Porture configuration:

```toml
[[tcp]]
bind_addr = "0.0.0.0"
bind_port = 8080
target_addr = "192.168.1.100"
target_port = 80

[[udp]]
bind_addr = "0.0.0.0"
bind_port = 5353
target_addr = "8.8.8.8"
target_port = 53
```

### Load Balancing

```toml
# Multiple rules for simple load balancing
[[tcp]]
bind_addr = "0.0.0.0"
bind_port = 8080
target_addr = "backend1.example.com"
target_port = 80
name = "backend1"

[[tcp]]
bind_addr = "0.0.0.0"
bind_port = 8081
target_addr = "backend2.example.com"
target_port = 80
name = "backend2"
```

### Development Proxy

```toml
# Forward local development server
[[tcp]]
bind_addr = "0.0.0.0"
bind_port = 3000
target_addr = "127.0.0.1"
target_port = 3001
name = "dev_server"
```

## Performance

Porture is built for high performance:

- **Async I/O**: Uses Tokio for non-blocking operations
- **Zero-copy**: Efficient buffer management
- **Session Pooling**: Reuses UDP sessions when possible
- **Minimal Overhead**: Direct forwarding without deep packet inspection

## Security Considerations

- Run with minimal privileges (non-root user when possible)
- Use firewall rules to restrict access to bind addresses
- Monitor logs for unusual connection patterns
- Consider using TLS/encryption for sensitive traffic

## Troubleshooting

### Permission Denied

If you get permission denied errors when binding to ports < 1024:

```bash
# Linux: Use capabilities instead of running as root
sudo setcap 'cap_net_bind_service=+ep' ./porture

# Or run as root (not recommended)
sudo ./porture
```

### Address Already in Use

Check if another process is using the port:

```bash
# Linux/macOS
lsof -i :8080
netstat -tlnp | grep 8080
```

### High CPU Usage

- Increase `buffer_size` in configuration
- Check for connection loops
- Monitor target server performance

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

See LICENSE file for details.

## Changelog

### v0.1.0
- Initial release
- TCP and UDP forwarding
- TOML configuration
- Session management for UDP
- Signal handling
- Comprehensive logging
A minimal, programmable port forwarder written in Rust
