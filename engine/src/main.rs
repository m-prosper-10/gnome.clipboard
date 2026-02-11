use zbus::{interface, connection, fdo, Connection};
use zvariant::ObjectPath;
use std::env;
use std::process::ExitCode;

mod engine;
use engine::EmojiEngine;

struct EmojiFactory;

#[interface(name = "org.freedesktop.IBus.Factory")]
impl EmojiFactory {
    async fn create_engine(&self, _name: String) -> fdo::Result<ObjectPath<'static>> {
        println!("IBus requested CreateEngine('{}')", _name);
        // In a minimal implementation, we assume the engine is already registered 
        // or we register it on the fly. 
        // Standard IBus engines register a new engine object for each request.
        // For simplicity, we'll use a fixed path for now.
        Ok(ObjectPath::from_static_str("/org/freedesktop/IBus/Engine/1").unwrap())
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 && args[1] == "--ibus" {
        // IBus mode - run as an input method engine
        println!("Starting emoji-input-engine in IBus mode...");
        if let Err(e) = run_ibus_engine().await {
            eprintln!("Error running IBus engine: {:?}", e);
            return ExitCode::FAILURE;
        }
    } else {
        // Standalone mode - just print version info
        println!("emoji-input-engine v{}", env!("CARGO_PKG_VERSION"));
        println!("PHASE 3: Composition Buffer + Search Core");
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
    let db_path = env::var("EMOJI_DATA_DIR")
        .unwrap_or_else(|_| "/usr/local/share/gnome-emoji-input".to_string());
    let db_file = format!("{}/emojis.json", db_path);
    
    // Fallback to local data dir for development if not found
    let db_file = if std::path::Path::new(&db_file).exists() {
        db_file
    } else {
        "data/emojis.json".to_string()
    };
    
    println!("Loading emoji database from: {}", db_file);
    let db_content = std::fs::read_to_string(db_file)?;
    let database: engine::EmojiDatabase = serde_json::from_str(&db_content)?;
    println!("Loaded {} emojis.", database.emojis.len());

    let addr_str = env::var("IBUS_ADDRESS")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| get_ibus_address().ok())
        .ok_or_else(|| "Could not find IBUS_ADDRESS".to_string())?;
    let address: zbus::Address = addr_str.parse()?;
    
    let connection: Connection = connection::Builder::address(address)?
        .name("org.freedesktop.IBus.EmojiInput")?
        .serve_at("/org/freedesktop/IBus/Factory", EmojiFactory)?
        .serve_at("/org/freedesktop/IBus/Engine/1", EmojiEngine::with_database(database))?
        .build()
        .await?;
        
    println!("Engine process started. Serving Factory and Engine objects.");
    println!("Bus: {}", connection.unique_name().map(|n| n.as_str()).unwrap_or("unknown"));
    
    // Launch UI process
    let current_exe = env::current_exe().ok();
    let ui_path = current_exe.as_ref()
        .and_then(|p| p.parent())
        .map(|p| p.join("emoji-input-ui"))
        .filter(|p| p.exists())
        .map(|p| p.to_string_lossy().to_string())
        .or_else(|| {
            let path = "/usr/local/libexec/emoji-input-ui";
            if std::path::Path::new(path).exists() {
                Some(path.to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            // Local dev fallback
            "./ui/target/debug/emoji-input-ui".to_string()
        });

    println!("Launching UI from: {}", ui_path);
    let mut ui_child = std::process::Command::new(ui_path)
        .spawn()
        .expect("Failed to launch UI process");
    
    tokio::signal::ctrl_c().await?;
    println!("\nShutting down engine...");
    let _ = ui_child.kill();
    
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
