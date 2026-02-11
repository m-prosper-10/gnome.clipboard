use gtk4 as gtk;
use gtk::prelude::*;
use zbus::proxy;
use serde::{Deserialize, Serialize};
use futures_util::stream::StreamExt;
use std::env;
use log::{info, error, debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize, zvariant::Type)]
pub struct Emoji {
    pub char: String,
    pub name: String,
    pub keywords: Vec<String>,
    pub variants: Vec<String>,
}

#[proxy(
    interface = "org.freedesktop.IBus.Engine",
    default_service = "org.freedesktop.IBus.EmojiInput",
    default_path = "/org/freedesktop/IBus/Engine/1"
)]
trait EmojiEngine {
    #[zbus(signal)]
    fn update_results(&self, results: Vec<Emoji>, selected_index: u32) -> zbus::Result<()>;
}


fn get_ibus_address() -> Option<String> {
    if let Ok(addr) = env::var("IBUS_ADDRESS") {
        if !addr.is_empty() {
            return Some(addr);
        }
    }
    
    let home = env::var("HOME").ok()?;
    let machine_id = std::fs::read_to_string("/etc/machine-id").ok()?.trim().to_string();
    let path = format!("{}/.config/ibus/bus/", home);
    let entries = std::fs::read_dir(path).ok()?;
    
    for entry in entries {
        if let Ok(entry) = entry {
            let filename = entry.file_name().to_string_lossy().to_string();
            if filename.starts_with(&machine_id) {
                let content = std::fs::read_to_string(entry.path()).ok()?;
                for line in content.lines() {
                    if let Some(addr) = line.strip_prefix("IBUS_ADDRESS=") {
                        return Some(addr.trim().to_string());
                    }
                }
            }
        }
    }
    None
}

#[tokio::main]
async fn main() -> glib::ExitCode {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let application = gtk::Application::builder()
        .application_id("org.example.EmojiInputUI")
        .build();

    application.connect_activate(move |app| {
        let window = gtk::Window::builder()
            .application(app)
            .title("Emoji Picker")
            .default_width(300)
            .default_height(400)
            .decorated(false)
            .can_focus(false)
            .build();

        let list_box = gtk::ListBox::builder()
            .margin_top(10)
            .margin_bottom(10)
            .margin_start(10)
            .margin_end(10)
            .build();

        window.set_child(Some(&list_box));

        let window_clone = window.clone();
        let list_box_clone = list_box.clone();
        let app_clone = app.clone();

        // Run DBus listener on the main thread (local task)
        glib::MainContext::default().spawn_local(async move {
            let app = app_clone;
            let addr = match get_ibus_address() {
                Some(a) => a,
                None => {
                    error!("Could not find IBus address. Is IBus running?");
                    app.quit();
                    return;
                }
            };
            
            let address: zbus::Address = match addr.parse() {
                Ok(a) => a,
                Err(e) => {
                    error!("Invalid IBus address '{}': {}", addr, e);
                    app.quit();
                    return;
                }
            };

            let conn = match zbus::connection::Builder::address(address).build().await {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to connect to IBus at {}: {}", addr, e);
                    app.quit();
                    return;
                }
            };

            let proxy = match EmojiEngineProxy::new(&conn).await {
                Ok(p) => p,
                Err(e) => {
                    error!("Failed to create EmojiEngine proxy: {}", e);
                    app.quit();
                    return;
                }
            };

            let mut stream = match proxy.receive_update_results().await {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to receive update_results stream: {}", e);
                    app.quit();
                    return;
                }
            };

            info!("Connected to IBus engine. Listening for signals...");

            while let Some(signal) = stream.next().await {
                let args = match signal.args() {
                    Ok(a) => a,
                    Err(e) => {
                        warn!("Failed to get signal args: {}", e);
                        continue;
                    }
                };
                let results = args.results;
                let selected_index = args.selected_index as i32;
                
                debug!("Received {} results, selected_index: {}", results.len(), selected_index);
                let list_box = list_box_clone.clone();
                let window = window_clone.clone();
                
                glib::idle_add_local(move || {
                    if results.is_empty() {
                        window.hide();
                    } else {
                        // Clear list
                        while let Some(child) = list_box.first_child() {
                            list_box.remove(&child);
                        }
                        
                        // Add results
                        for emoji in &results {
                            let label = gtk::Label::new(Some(&format!("{} :{}", emoji.char, emoji.name)));
                            label.set_halign(gtk::Align::Start);
                            list_box.append(&label);
                        }
                        
                        // Select the row
                        if let Some(row) = list_box.row_at_index(selected_index) {
                            list_box.select_row(Some(&row));
                            // Row might need focus to show selection style in some themes
                            row.grab_focus();
                        }

                        window.show();
                    }
                    glib::ControlFlow::Break
                });
            }
            info!("Engine stream ended, shutting down UI...");
            app.quit();
        });
    });

    application.run()
}
