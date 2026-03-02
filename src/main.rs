mod server;

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "second-opinion", about = "Second opinion CLI")]
struct Cli {
    /// Port to use (overrides env and config)
    #[arg(long, global = true)]
    port: Option<u16>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the WebSocket server daemon
    Start,
    /// Stop the WebSocket server daemon
    Stop,
    /// Show server status as JSON
    Status,
    /// Send a message and get a response
    Ask {
        /// The message to send
        message: String,
    },
}

#[derive(Deserialize, Default)]
struct Config {
    port: Option<u16>,
    timeout_secs: Option<u64>,
}

fn load_config() -> Config {
    let config_path = PathBuf::from(".agents/second-opinion/second-opinion.toml");
    if config_path.exists() {
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(config) = toml::from_str::<Config>(&content) {
                return config;
            }
        }
    }
    Config::default()
}

fn resolve_port(cli_port: Option<u16>, config: &Config) -> u16 {
    if let Some(p) = cli_port {
        return p;
    }
    if let Ok(env_val) = std::env::var("SECOND_OPINION_PORT") {
        if let Ok(p) = env_val.parse::<u16>() {
            return p;
        }
    }
    if let Some(p) = config.port {
        return p;
    }
    7878
}

fn state_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Could not find home directory")
        .join(".second-opinion")
}

fn pid_file() -> PathBuf {
    state_dir().join("server.pid")
}

fn port_file() -> PathBuf {
    state_dir().join("server.port")
}

fn read_pid() -> Option<i32> {
    let content = fs::read_to_string(pid_file()).ok()?;
    content.trim().parse::<i32>().ok()
}

fn read_port_file() -> Option<u16> {
    let content = fs::read_to_string(port_file()).ok()?;
    content.trim().parse::<u16>().ok()
}

fn is_process_alive(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}

fn is_server_running() -> bool {
    if let Some(pid) = read_pid() {
        is_process_alive(pid)
    } else {
        false
    }
}

fn port_available(port: u16) -> bool {
    std::net::TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok()
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = load_config();
    let port = resolve_port(cli.port, &config);
    let timeout_secs = config.timeout_secs.unwrap_or(60);

    match cli.command {
        Commands::Start => cmd_start(port).await?,
        Commands::Stop => cmd_stop().await?,
        Commands::Status => cmd_status(port).await?,
        Commands::Ask { message } => cmd_ask(port, &message, timeout_secs).await?,
    }

    Ok(())
}

async fn cmd_start(port: u16) -> Result<()> {
    // Check if already running
    if is_server_running() {
        let running_port = read_port_file().unwrap_or(port);
        println!("Server already running on port {}", running_port);
        return Ok(());
    }

    // Verify port is available before daemonizing
    if !port_available(port) {
        anyhow::bail!("Port {} is not available", port);
    }

    let state_dir = state_dir();
    fs::create_dir_all(&state_dir).context("Failed to create state directory")?;

    // Double-fork daemonization
    unsafe {
        let pid = libc::fork();
        if pid < 0 {
            panic!("fork failed");
        }
        if pid > 0 {
            // Parent: wait for port file to appear (up to 5s) then exit
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                if port_file().exists() {
                    if let Some(p) = read_port_file() {
                        println!("Server started on port {}", p);
                        std::process::exit(0);
                    }
                }
                if Instant::now() >= deadline {
                    eprintln!("Timeout waiting for server to start");
                    std::process::exit(1);
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }

        // Intermediate child
        libc::setsid();

        let pid2 = libc::fork();
        if pid2 < 0 {
            panic!("fork2 failed");
        }
        if pid2 > 0 {
            // Intermediate exits
            std::process::exit(0);
        }

        // Daemon process: redirect stdin/stdout/stderr to /dev/null
        let devnull = libc::open(b"/dev/null\0".as_ptr() as *const libc::c_char, libc::O_RDWR);
        if devnull >= 0 {
            libc::dup2(devnull, 0);
            libc::dup2(devnull, 1);
            libc::dup2(devnull, 2);
            if devnull > 2 {
                libc::close(devnull);
            }
        }
    }

    // Daemon: write PID file
    let pid = unsafe { libc::getpid() };
    fs::write(pid_file(), pid.to_string()).context("Failed to write PID file")?;
    fs::write(port_file(), port.to_string()).context("Failed to write port file")?;

    // Run the server
    server::run_server(port).await?;

    Ok(())
}

async fn cmd_stop() -> Result<()> {
    let pid = match read_pid() {
        Some(p) => p,
        None => {
            println!("Server not running.");
            return Ok(());
        }
    };

    if !is_process_alive(pid) {
        // Stale PID file
        let _ = fs::remove_file(pid_file());
        let _ = fs::remove_file(port_file());
        println!("Server not running.");
        return Ok(());
    }

    let port = read_port_file();

    // Send SIGTERM
    unsafe {
        libc::kill(pid, libc::SIGTERM);
    }

    // Wait up to 5s for process to exit
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if !is_process_alive(pid) {
            break;
        }
        if Instant::now() >= deadline {
            eprintln!("Timeout waiting for server to stop");
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // If port was known, also wait for port to be freed
    if let Some(p) = port {
        let deadline2 = Instant::now() + Duration::from_secs(2);
        loop {
            if port_available(p) {
                break;
            }
            if Instant::now() >= deadline2 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    let _ = fs::remove_file(pid_file());
    let _ = fs::remove_file(port_file());
    println!("Server stopped.");

    Ok(())
}

async fn cmd_status(port: u16) -> Result<()> {
    let running = is_server_running();
    let actual_port = read_port_file().unwrap_or(port);

    if !running {
        let status = json!({
            "running": false,
            "port": 0,
            "extension_connected": false
        });
        println!("{}", status);
        return Ok(());
    }

    // Query extension_connected via WS
    let extension_connected = query_extension_connected(actual_port).await;

    let status = json!({
        "running": running,
        "port": actual_port,
        "extension_connected": extension_connected
    });
    println!("{}", status);

    Ok(())
}

async fn query_extension_connected(port: u16) -> bool {
    let url = format!("ws://127.0.0.1:{}", port);
    let ws_result = tokio::time::timeout(
        Duration::from_secs(2),
        connect_async(&url),
    )
    .await;

    let (ws_stream, _) = match ws_result {
        Ok(Ok(ws)) => ws,
        _ => return false,
    };

    let (mut sender, mut receiver) = ws_stream.split();

    let query = json!({"type": "status_query"}).to_string();
    if sender.send(Message::Text(query.into())).await.is_err() {
        return false;
    }

    let result = tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(msg) = receiver.next().await {
            if let Ok(Message::Text(text)) = msg {
                if let Ok(parsed) = serde_json::from_str::<Value>(&text) {
                    if parsed.get("type").and_then(|v| v.as_str()) == Some("status_response") {
                        return parsed
                            .get("extension_connected")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                    }
                }
            }
        }
        false
    })
    .await;

    result.unwrap_or(false)
}

async fn cmd_ask(port: u16, message: &str, timeout_secs: u64) -> Result<()> {
    let actual_port = read_port_file().unwrap_or(port);
    let url = format!("ws://127.0.0.1:{}", actual_port);

    let ws_result = tokio::time::timeout(
        Duration::from_secs(5),
        connect_async(&url),
    )
    .await;

    let (ws_stream, _) = match ws_result {
        Ok(Ok(ws)) => ws,
        _ => {
            eprintln!("Server not running. Call 'start' first.");
            std::process::exit(1);
        }
    };

    let (mut sender, mut receiver) = ws_stream.split();

    let id = Uuid::new_v4().to_string();
    let ask_msg = json!({
        "type": "ask",
        "id": id,
        "message": message
    })
    .to_string();

    if sender.send(Message::Text(ask_msg.into())).await.is_err() {
        eprintln!("Server not running. Call 'start' first.");
        std::process::exit(1);
    }

    let id_clone = id.clone();
    let result = tokio::time::timeout(Duration::from_secs(timeout_secs), async move {
        while let Some(msg) = receiver.next().await {
            if let Ok(Message::Text(text)) = msg {
                if let Ok(parsed) = serde_json::from_str::<Value>(&text) {
                    let msg_id = parsed.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    if msg_id != id_clone {
                        continue;
                    }
                    let msg_type = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    match msg_type {
                        "response" => {
                            let text_val = parsed
                                .get("text")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            return Ok(text_val);
                        }
                        "error" => {
                            let err = parsed
                                .get("error")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();
                            return Err(err);
                        }
                        _ => {}
                    }
                }
            }
        }
        Err("connection_closed".to_string())
    })
    .await;

    match result {
        Ok(Ok(text)) => {
            println!("{}", text);
            std::process::exit(0);
        }
        Ok(Err(err)) => {
            match err.as_str() {
                "extension_not_connected" => {
                    eprintln!("extension_not_connected");
                    std::process::exit(2);
                }
                "timeout" => {
                    eprintln!("timeout");
                    std::process::exit(3);
                }
                other => {
                    eprintln!("{}", other);
                    std::process::exit(1);
                }
            }
        }
        Err(_timeout) => {
            eprintln!("Timeout waiting for Grok response.");
            std::process::exit(3);
        }
    }
}
