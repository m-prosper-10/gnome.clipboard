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
        println!("PHASE 2: Minimal IBus engine with hardcoded emoji");
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
    // Get IBus address from environment or file
    let addr_str = env::var("IBUS_ADDRESS").or_else(|_| get_ibus_address())?;
    let address: zbus::Address = addr_str.parse()?;
    
    let connection: Connection = connection::Builder::address(address)?
        .name("org.freedesktop.IBus.EmojiInput")?
        .serve_at("/org/freedesktop/IBus/Factory", EmojiFactory)?
        .serve_at("/org/freedesktop/IBus/Engine/1", EmojiEngine::new())?
        .build()
        .await?;
        
    println!("Engine process started. Serving Factory and Engine objects.");
    println!("Bus: {}", connection.unique_name().map(|n| n.as_str()).unwrap_or("unknown"));
    
    // Notify IBus daemon about our factory (usually done via RegisterComponent)
    // For PHASE 2, we rely on the component XML pointing to this binary.
    
    tokio::signal::ctrl_c().await?;
    println!("\nShutting down engine...");
    
    Ok(())
}

fn get_ibus_address() -> Result<String, Box<dyn std::error::Error>> {
    // Basic implementation to find the address file if env var is missing
    // Mirrors the logic in librush/ibus-rs
    let home = env::var("HOME")?;
    let machine_id = std::fs::read_to_string("/etc/machine-id")?.trim().to_string();
    let display = env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string());
    let display_num = display.split(':').nth(1).and_then(|s| s.split('.').next()).unwrap_or("0");
    
    let path = format!("{}/.config/ibus/bus/{}-unix-{}", home, machine_id, display_num);
    let content = std::fs::read_to_string(path)?;
    
    for line in content.lines() {
        if let Some(addr) = line.strip_prefix("IBUS_ADDRESS=") {
            return Ok(addr.to_string());
        }
    }
    
    Err("Could not find IBUS_ADDRESS".into())
}
