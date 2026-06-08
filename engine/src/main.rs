use zbus::{interface, connection, fdo, proxy, Connection};
use zbus::object_server::SignalEmitter;
use zvariant::ObjectPath;
use std::env;
use std::process::ExitCode;
use std::sync::Arc;
use log::{info, error, debug, warn};

mod engine;
use engine::{EmojiEngine, Emoji};

struct EmojiFactory;

/// Session bus service for UI - forwards CommitEmoji to engine via channel
struct PickerService {
    commit_tx: tokio::sync::mpsc::Sender<String>,
    picker_token: String,
}

#[interface(name = "org.example.EmojiInput.Picker")]
impl PickerService {
    async fn commit_emoji(&self, text: String, token: String) -> fdo::Result<()> {
        if token != self.picker_token {
            warn!("Rejecting commit_emoji with invalid instance token");
            return Ok(());
        }

        let _ = self.commit_tx.send(text).await;
        Ok(())
    }
}

#[proxy(
    interface = "org.freedesktop.IBus.Engine",
    default_service = "org.example.EmojiInput",
    default_path = "/org/freedesktop/IBus/Engine/1"
)]
trait EngineCommit {
    fn commit_emoji(&self, text: &str) -> zbus::Result<()>;
}

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
        println!("PHASE 2: Minimal IBus Engine");
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
    let prefix = env::var("PREFIX").unwrap_or_else(|_| {
        let home = env::var("HOME").unwrap_or_else(|_| "/usr/local".to_string());
        format!("{}/.local", home)
    });
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
    let database = engine::EmojiDatabase::load_from_source_with_cache(&db_file)
        .map_err(|e| format!("Failed to load emoji database {}: {}", db_file, e))?;
    info!("Loaded {} emojis.", database.emojis.len());

    let addr_str = env::var("IBUS_ADDRESS")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| get_ibus_address().ok())
        .ok_or_else(|| "Could not find IBUS_ADDRESS. Is IBus running?".to_string())?;
    
    debug!("Connecting to IBus at {}", addr_str);
    let address: zbus::Address = addr_str.parse()
        .map_err(|e| format!("Invalid IBus address '{}': {}", addr_str, e))?;

    // Session bus channel for UI popup (IBus daemon bus is not visible to GTK app)
    let (picker_tx, mut picker_rx) = tokio::sync::mpsc::channel::<(Vec<Emoji>, u32)>(32);
    let picker_tx = Arc::new(picker_tx);

    // Channel for UI click-to-commit (session bus -> engine)
    let (commit_tx, mut commit_rx) = tokio::sync::mpsc::channel::<String>(16);
    let picker_token = format!(
        "{:x}-{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default(),
        std::process::id()
    );
    let picker_token_for_bridge = picker_token.clone();
    let picker_token_for_ui = picker_token.clone();
    let picker_service = PickerService {
        commit_tx,
        picker_token: picker_token.clone(),
    };

    let session_conn = zbus::connection::Builder::session()?
        .name("org.example.EmojiInput.Picker")?
        .serve_at("/org/example/EmojiInput/Picker", picker_service)?
        .build()
        .await?;
    let session_emitter = SignalEmitter::new(&session_conn, "/org/example/EmojiInput/Picker")
        .expect("Failed to create signal emitter");

    let picker_task = tokio::spawn(async move {
        while let Some((results, selected_index)) = picker_rx.recv().await {
            let body = (results, selected_index);
            if let Err(e) = session_emitter
                .emit("org.example.EmojiInput.Picker", "UpdateResults", &body)
                .await
            {
                warn!("Failed to emit UpdateResults on session bus: {}", e);
            }
        }
    });

    let connection: Connection = connection::Builder::address(address)?
        .name("org.example.EmojiInput")?
        .serve_at("/org/freedesktop/IBus/Factory", EmojiFactory)?
        .serve_at(
            "/org/freedesktop/IBus/Engine/1",
            EmojiEngine::with_database_and_picker(database, Some(picker_tx)),
        )?
        .build()
        .await?;
        
    info!("Engine process started. Unique name: {}", 
        connection.unique_name().map(|n| n.as_str()).unwrap_or("none"));

    // Bridge: forward UI commit requests to engine (engine must be built first)
    let addr_str_clone = addr_str.clone();
    let bridge_task = tokio::spawn(async move {
        let addr: zbus::Address = match addr_str_clone.parse() {
            Ok(a) => a,
            Err(e) => {
                error!("Bridge: invalid IBus address: {}", e);
                return;
            }
        };
        let builder = match connection::Builder::address(addr) {
            Ok(b) => b,
            Err(e) => {
                error!("Bridge: invalid IBus address: {}", e);
                return;
            }
        };
        let ibus_conn = match builder.build().await {
            Ok(c) => c,
            Err(e) => {
                error!("Bridge: failed to connect to IBus: {}", e);
                return;
            }
        };
        let proxy = match EngineCommitProxy::new(&ibus_conn).await {
            Ok(p) => p,
            Err(e) => {
                error!("Bridge: failed to create engine proxy: {}", e);
                return;
            }
        };
        while let Some(text) = commit_rx.recv().await {
            if let Err(e) = proxy.commit_emoji(&text, &picker_token_for_bridge).await {
                warn!("Bridge: commit_emoji failed: {}", e);
            }
        }
    });
    
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

    let ui_child = if let Some(path) = ui_path {
        info!("Launching UI from: {:?}", path);
        match std::process::Command::new(&path)
            .env("EMOJI_INPUT_PICKER_TOKEN", &picker_token_for_ui)
            .spawn()
        {
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
    picker_task.abort();
    bridge_task.abort();
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
