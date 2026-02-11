use zbus::{interface, connection, fdo, Connection};
use zvariant::ObjectPath;
use std::env;
use std::process::ExitCode;
use log::{info, error, debug, warn};

mod engine;
use engine::EmojiEngine;

struct EmojiFactory;

#[interface(name = "org.freedesktop.IBus.Factory")]
impl EmojiFactory {
    async fn create_engine(&self, _name: String) -> fdo::Result<ObjectPath<'static>> {
        info!("IBus requested CreateEngine('{}')", _name);
        Ok(ObjectPath::from_static_str("/org/freedesktop/IBus/Engine/1").expect("Static path should be valid"))
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    // Initialize env_logger with a default level if not set
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 && args[1] == "--ibus" {
        info!("Starting emoji-input-engine in IBus mode...");
        if let Err(e) = run_ibus_engine().await {
            error!("Fatal error running IBus engine: {}", e);
            return ExitCode::FAILURE;
        }
    } else {
        println!("emoji-input-engine v{}", env!("CARGO_PKG_VERSION"));
        println!("PHASE 8: Hardening");
        println!();
        println!("To use as IBus engine:");
        println!("  1. Copy ibus-component.xml to ~/.local/share/ibus/component/");
        println!("  2. Run: ibus restart");
        println!("  3. Select 'Emoji Input' in ibus-setup");
        println!("  4. Type ':emoji:' to insert 🙂");
    }
    ExitCode::SUCCESS
}

async fn run_ibus_engine() -> Result<(), Box<dyn std::error::Error>> {
    // Load emoji database
    let prefix = env::var("PREFIX").unwrap_or_else(|_| "/home/polo/.local".to_string());
    let db_path = env::var("EMOJI_DATA_DIR")
        .unwrap_or_else(|_| format!("{}/share/gnome-emoji-input", prefix));
    let db_file = format!("{}/emojis.json", db_path);
    
    // Fallback to local data dir for development if not found
    let db_file = if std::path::Path::new(&db_file).exists() {
        db_file
    } else {
        "data/emojis.json".to_string()
    };
    
    info!("Loading emoji database from: {}", db_file);
    let db_content = std::fs::read_to_string(&db_file)
        .map_err(|e| format!("Failed to read emoji database {}: {}", db_file, e))?;
    let database: engine::EmojiDatabase = serde_json::from_str(&db_content)
        .map_err(|e| format!("Failed to parse emoji database: {}", e))?;
    info!("Loaded {} emojis.", database.emojis.len());

    let addr_str = env::var("IBUS_ADDRESS")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| get_ibus_address().ok())
        .ok_or_else(|| "Could not find IBUS_ADDRESS. Is IBus running?".to_string())?;
    
    debug!("Connecting to IBus at {}", addr_str);
    let address: zbus::Address = addr_str.parse()
        .map_err(|e| format!("Invalid IBus address '{}': {}", addr_str, e))?;
    
    let connection: Connection = connection::Builder::address(address)?
        .name("org.freedesktop.IBus.EmojiInput")?
        .serve_at("/org/freedesktop/IBus/Factory", EmojiFactory)?
        .serve_at("/org/freedesktop/IBus/Engine/1", EmojiEngine::with_database(database))?
        .build()
        .await?;
        
    info!("Engine process started. Unique name: {}", 
        connection.unique_name().map(|n| n.as_str()).unwrap_or("none"));
    
    // Launch UI process
    let current_exe = env::current_exe().ok();
    let bin_dir = current_exe.as_ref().and_then(|p| p.parent());
    
    let ui_path = bin_dir.map(|bit| bit.join("emoji-input-ui"))
        .filter(|p| p.exists())
        .or_else(|| {
            let path = format!("{}/libexec/emoji-input-ui", prefix);
            let p = std::path::PathBuf::from(&path);
            if p.exists() { Some(p) } else { None }
        })
        .or_else(|| {
            let p = std::path::PathBuf::from("./ui/target/debug/emoji-input-ui");
            if p.exists() { Some(p) } else { None }
        });

    let mut ui_child = if let Some(path) = ui_path {
        info!("Launching UI from: {:?}", path);
        match std::process::Command::new(&path).spawn() {
            Ok(child) => Some(child),
            Err(e) => {
                error!("Failed to launch UI process at {:?}: {}", path, e);
                None
            }
        }
    } else {
        warn!("UI binary 'emoji-input-ui' not found. Popup will not appear.");
        None
    };
    
    tokio::signal::ctrl_c().await?;
    info!("Shutting down engine...");
    if let Some(mut child) = ui_child {
        let _ = child.kill();
    }
    
    Ok(())
}

fn get_ibus_address() -> Result<String, Box<dyn std::error::Error>> {
    // Basic implementation to find the address file if env var is missing
    // Mirrors the logic in librush/ibus-rs
    let home = env::var("HOME")?;
    let machine_id = std::fs::read_to_string("/etc/machine-id")?.trim().to_string();
    
    let path = format!("{}/.config/ibus/bus/", home);
    let entries = std::fs::read_dir(path)?;
    
    for entry in entries {
        let entry = entry?;
        let filename = entry.file_name().to_string_lossy().to_string();
        if filename.starts_with(&machine_id) {
            let content = std::fs::read_to_string(entry.path())?;
            for line in content.lines() {
                if let Some(addr) = line.strip_prefix("IBUS_ADDRESS=") {
                    return Ok(addr.trim().to_string());
                }
            }
        }
    }
    
    Err("Could not find IBUS_ADDRESS in any file".into())
}
