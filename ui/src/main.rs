use gtk4 as gtk;
use gtk::prelude::*;
use zbus::{Connection, proxy};
use zvariant::Value;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize, zvariant::Type)]
pub struct Emoji {
    pub char: String,
    pub name: String,
    pub keywords: Vec<String>,
}

#[proxy(
    interface = "org.freedesktop.IBus.Engine",
    default_service = "org.freedesktop.IBus.EmojiInput",
    default_path = "/org/freedesktop/IBus/Engine/1"
)]
trait EmojiEngine {
    #[zbus(signal)]
    fn update_results(&self, results: Vec<Emoji>) -> zbus::Result<()>;
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

fn main() -> glib::ExitCode {
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

        // Run DBus listener in a separate task
        glib::MainContext::default().spawn_local(async move {
            let addr = get_ibus_address().expect("Could not find IBus address");
            let conn = Connection::builder()
                .address(addr.parse().expect("Invalid IBus address"))
                .expect("Failed to create connection builder")
                .build()
                .await
                .expect("Failed to connect to IBus");

            let proxy = EmojiEngineProxy::new(&conn).await.expect("Failed to create proxy");
            let mut stream = proxy.receive_update_results().await.expect("Failed to receive stream");

            while let Some(signal) = stream.next().await {
                let results = signal.args().expect("Failed to get signal args").results;
                
                // Update UI on main thread
                let list_box = list_box_clone.clone();
                let window = window_clone.clone();
                
                if results.is_empty() {
                    window.hide();
                } else {
                    // Clear list
                    while let Some(child) = list_box.first_child() {
                        list_box.remove(&child);
                    }
                    
                    // Add results
                    for emoji in results {
                        let label = gtk::Label::new(Some(&format!("{} :{}", emoji.char, emoji.name)));
                        label.set_halign(gtk::Align::Start);
                        list_box.append(&label);
                    }
                    window.show();
                }
            }
        });
    });

    application.run()
}
