// PHASE 2: Minimal IBus Engine Implementation
// This is a hardcoded test implementation that commits a single emoji

use ibus::{self, Bus, Factory, Engine};
use glib;
use std::env;

mod engine;
use engine::EmojiEngine;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 && args[1] == "--ibus" {
        // IBus mode - run as an input method engine
        println!("Starting emoji-input-engine in IBus mode...");
        run_ibus_engine();
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
}

fn run_ibus_engine() {
    // Initialize IBus
    ibus::init();
    
    // Create a new IBus Bus
    let bus = Bus::new();
    if !bus.is_connected() {
        eprintln!("Failed to connect to IBus.");
        return;
    }

    // Create a factory for our engine
    let mut factory = Factory::new(bus.connection());
    
    // Create our engine state
    let mut emoji_engine = EmojiEngine::new();
    
    // Register the engine
    // Note: The specific API for ibus-rs might vary, 
    // but we'll try to follow the C-style pattern it claims to mirror.
    factory.add_engine("emoji-input", "EmojiEngine");
    
    println!("Engine process started. Registered 'emoji-input'.");
    println!("Waiting for IBus connections...");
    
    // Initialize GLib main loop (IBus uses it internally)
    let main_loop = glib::MainLoop::new(None, false);
    
    // Set up signal handlers for clean exit
    let loop_clone = main_loop.clone();
    let _source_id = glib::unix_signal_add(libc::SIGINT, move || {
        println!("\nShutting down engine...");
        loop_clone.quit();
        glib::ControlFlow::Break
    });
    
    // Run the main loop
    main_loop.run();
    
    println!("Engine stopped.");
}
