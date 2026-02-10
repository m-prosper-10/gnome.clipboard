// PHASE 2: Minimal IBus Engine Implementation
// This is a hardcoded test implementation that commits a single emoji

use gio::prelude::*;
use glib;
use std::env;
use std::process;

mod engine;

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
    // Initialize GLib main loop
    let main_loop = glib::MainLoop::new(None, false);
    
    // PHASE 2: Minimal stub - just keep the process alive
    // Real IBus integration will be added after verifying component registration
    println!("Engine process started. Waiting for IBus connections...");
    println!("Press Ctrl+C to stop.");
    
    // Set up signal handlers
    let loop_clone = main_loop.clone();
    let _source_id = glib::unix_signal_add(glib::signal::SIGINT, move || {
        println!("\nShutting down engine...");
        loop_clone.quit();
        glib::ControlFlow::Break
    });
    
    let loop_clone2 = main_loop.clone();
    let _source_id2 = glib::unix_signal_add(glib::signal::SIGTERM, move || {
        println!("\nShutting down engine...");
        loop_clone2.quit();
        glib::ControlFlow::Break
    });
    
    // Run the main loop
    main_loop.run();
    
    println!("Engine stopped.");
}
