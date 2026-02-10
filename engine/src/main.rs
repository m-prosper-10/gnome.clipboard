use librush::ibus::{self, IBus, IBusFactory};
use std::env;
use std::process::ExitCode;

mod engine;
use engine::EmojiEngine;

struct EmojiFactory;

impl IBusFactory<EmojiEngine> for EmojiFactory {
    fn create_engine(&mut self, _name: String) -> Result<EmojiEngine, String> {
        Ok(EmojiEngine::new())
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
    let addr = ibus::get_ibus_addr()?;
    let factory = EmojiFactory;
    
    // The "name" here should match the engine name in ibus-component.xml
    // but librush::IBus::new uses it for the DBus service name usually.
    // IBus engines use a unique bus name like "org.freedesktop.IBus.EmojiInput"
    let _ibus = IBus::new(addr, factory, "org.freedesktop.IBus.EmojiInput".to_string()).await?;
    
    println!("Engine process started. Registered 'emoji-input' via librush.");
    println!("Press Ctrl+C to stop.");
    
    // Keep the process alive
    tokio::signal::ctrl_c().await?;
    println!("\nShutting down engine...");
    
    Ok(())
}
