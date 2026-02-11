use libadwaita as adw;
use gtk4 as gtk;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub trigger_char: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            trigger_char: ":".to_string(),
        }
    }
}

fn get_config_path() -> Option<PathBuf> {
    let home = env::var("HOME").ok()?;
    let path = PathBuf::from(home)
        .join(".config")
        .join("gnome-emoji-input");
    let _ = std::fs::create_dir_all(&path);
    Some(path.join("settings.json"))
}

fn load_settings() -> Settings {
    if let Some(path) = get_config_path() {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(settings) = serde_json::from_str::<Settings>(&content) {
                return settings;
            }
        }
    }
    Settings::default()
}

fn save_settings(settings: &Settings) {
    if let Some(path) = get_config_path() {
        if let Ok(content) = serde_json::to_string_pretty(settings) {
            let _ = std::fs::write(path, content);
        }
    }
}

fn get_recents_path() -> Option<PathBuf> {
    let home = env::var("HOME").ok()?;
    let path = PathBuf::from(home)
        .join(".cache")
        .join("gnome-emoji-input")
        .join("recents.json");
    Some(path)
}

fn main() -> glib::ExitCode {
    let application = adw::Application::builder()
        .application_id("org.example.EmojiInputPrefs")
        .build();

    application.connect_activate(|app| {
        let settings = load_settings();
        
        // Window
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Emoji Input Settings")
            .default_width(400)
            .default_height(500)
            .build();

        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        window.set_content(Some(&content));

        let header = adw::HeaderBar::new();
        content.append(&header);

        let page = adw::PreferencesPage::new();
        content.append(&page);

        // General Group
        let general_group = adw::PreferencesGroup::builder()
            .title("General")
            .build();
        page.add(&general_group);

        let trigger_row = adw::EntryRow::builder()
            .title("Trigger Character")
            .text(&settings.trigger_char)
            .build();
        
        trigger_row.connect_apply(move |row: &adw::EntryRow| {
            let mut s = load_settings();
            s.trigger_char = row.text().to_string();
            save_settings(&s);
        });
        general_group.add(&trigger_row);

        // History Group
        let history_group = adw::PreferencesGroup::builder()
            .title("History")
            .build();
        page.add(&history_group);

        let clear_button = gtk::Button::builder()
            .label("Clear Recent Emojis")
            .css_classes(["destructive-action"])
            .build();
        
        clear_button.connect_clicked(|_| {
            if let Some(path) = get_recents_path() {
                let _ = std::fs::remove_file(path);
            }
        });
        history_group.add(&clear_button);

        window.present();
    });

    application.run()
}
