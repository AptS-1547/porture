mod config;
mod tcp_forwarder;
mod udp_forwarder;

use anyhow::Result;
use clap::{Arg, Command};
use config::Config;
use log::{error, info, warn};
use std::env;
use tcp_forwarder::TcpForwarder;
use udp_forwarder::UdpForwarder;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup panic handler
    human_panic::setup_panic!();

    // Parse command line arguments
    let matches = Command::new("porture")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("A minimal, programmable port forwarder written in Rust")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .default_value("config.toml")
        )
        .arg(
            Arg::new("log-level")
                .short('l')
                .long("log-level")
                .value_name("LEVEL")
                .help("Log level (error, warn, info, debug, trace)")
        )
        .arg(
            Arg::new("init")
                .long("init")
                .help("Generate default configuration file and exit")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();

    // Handle init command
    if matches.get_flag("init") {
        let config_path = matches.get_one::<String>("config").unwrap();
        match Config::create_default_config().save_to_file(config_path) {
            Ok(_) => {
                println!("Default configuration file created: {}", config_path);
                println!("Please edit the configuration file to suit your needs.");
                println!("The default configuration contains example rules that bind to localhost.");
                return Ok(());
            }
            Err(e) => {
                eprintln!("Failed to create configuration file '{}': {}", config_path, e);
                std::process::exit(1);
            }
        }
    }

    // Load configuration
    let config_path = matches.get_one::<String>("config").unwrap();
    let config_existed = std::path::Path::new(config_path).exists();
    
    let config = match Config::from_file_or_create_default(config_path) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load or create configuration file '{}': {}", config_path, e);
            std::process::exit(1);
        }
    };

    // Validate configuration
    if let Err(e) = config.validate() {
        eprintln!("Configuration validation failed: {}", e);
        std::process::exit(1);
    }

    // Setup logging
    let log_level = matches.get_one::<String>("log-level")
        .or_else(|| config.global.as_ref().and_then(|g| g.log_level.as_ref()))
        .map(|s| s.as_str())
        .unwrap_or("info");

    unsafe {
        env::set_var("RUST_LOG", log_level);
    }
    env_logger::init();

    info!("Starting Porture v{}", env!("CARGO_PKG_VERSION"));
    
    if !config_existed {
        info!("Created default configuration file: {}", config_path);
        info!("Please edit the configuration file to suit your needs");
        info!("Current configuration contains example rules that bind to localhost");
    } else {
        info!("Loaded configuration from: {}", config_path);
    }

    // Get buffer size
    let buffer_size = config.global
        .as_ref()
        .and_then(|g| g.buffer_size)
        .unwrap_or(8192);

    info!("Using buffer size: {} bytes", buffer_size);

    // Start TCP forwarders
    let mut tcp_tasks = Vec::new();
    if let Some(tcp_rules) = config.tcp {
        for rule in tcp_rules {
            let forwarder = TcpForwarder::new(rule, buffer_size);
            let task = tokio::spawn(async move {
                if let Err(e) = forwarder.start().await {
                    error!("TCP forwarder failed: {}", e);
                }
            });
            tcp_tasks.push(task);
        }
    }

    // Start UDP forwarders
    let mut udp_tasks = Vec::new();
    if let Some(udp_rules) = config.udp {
        for rule in udp_rules {
            let forwarder = UdpForwarder::new(rule, buffer_size);
            let task = tokio::spawn(async move {
                if let Err(e) = forwarder.start().await {
                    error!("UDP forwarder failed: {}", e);
                }
            });
            udp_tasks.push(task);
        }
    }

    // Check if we have any forwarders
    if tcp_tasks.is_empty() && udp_tasks.is_empty() {
        warn!("No forwarding rules configured. Nothing to do.");
        return Ok(());
    }

    info!("Started {} TCP forwarders and {} UDP forwarders", 
          tcp_tasks.len(), udp_tasks.len());

    // Setup signal handling
    let mut sigterm = tokio::signal::unix::signal(
        tokio::signal::unix::SignalKind::terminate()
    )?;
    let mut sigint = tokio::signal::unix::signal(
        tokio::signal::unix::SignalKind::interrupt()
    )?;

    // Wait for termination signal or all tasks to complete
    tokio::select! {
        _ = sigterm.recv() => {
            info!("Received SIGTERM, shutting down...");
        }
        _ = sigint.recv() => {
            info!("Received SIGINT, shutting down...");
        }
        _ = futures::future::try_join_all(tcp_tasks) => {
            warn!("All TCP forwarders stopped");
        }
        _ = futures::future::try_join_all(udp_tasks) => {
            warn!("All UDP forwarders stopped");
        }
    }

    info!("Porture shutdown complete");
    Ok(())
}
