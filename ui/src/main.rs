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
    #[serde(default)]
    pub variants: Vec<String>,
}

#[proxy(
    interface = "org.example.EmojiInput.Picker",
    default_service = "org.example.EmojiInput.Picker",
    default_path = "/org/example/EmojiInput/Picker"
)]
trait EmojiPicker {
    #[zbus(signal)]
    fn update_results(&self, results: Vec<Emoji>, selected_index: u32) -> zbus::Result<()>;
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
        // Use session bus - engine forwards UpdateResults there (IBus bus not visible to GTK app)
        glib::MainContext::default().spawn_local(async move {
            let app = app_clone;
            let conn = match zbus::connection::Builder::session() {
                Ok(b) => match b.build().await {
                    Ok(c) => c,
                    Err(e) => {
                        error!("Failed to connect to session bus: {}", e);
                        app.quit();
                        return;
                    }
                },
                Err(e) => {
                    error!("Failed to create session connection: {}", e);
                    app.quit();
                    return;
                }
            };

            let proxy = match EmojiPickerProxy::new(&conn).await {
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

            info!("Connected to session bus. Listening for emoji picker signals...");

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

                        window.present();
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
