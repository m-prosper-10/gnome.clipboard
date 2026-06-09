use libadwaita as adw;
use gtk4 as gtk;
use gio::prelude::*;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use log::info;
use std::env;
use std::path::PathBuf;

const SETTINGS_SCHEMA_ID: &str = "org.example.EmojiInput";
const SETTINGS_TRIGGER_CHAR: &str = "trigger-char";
const VARIANT_PREFS_KEY: &str = "variant-preferences";

#[derive(Debug, Clone)]
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

fn gsettings() -> gio::Settings {
    gio::Settings::new(SETTINGS_SCHEMA_ID)
}

fn load_settings(settings: &gio::Settings) -> Settings {
    let trigger_char = settings.string(SETTINGS_TRIGGER_CHAR).to_string();
    if trigger_char.is_empty() {
        Settings::default()
    } else {
        Settings { trigger_char }
    }
}

fn save_settings(settings: &gio::Settings, value: &Settings) {
    let _ = settings.set_string(SETTINGS_TRIGGER_CHAR, &value.trigger_char);
}

fn get_recents_path() -> Option<PathBuf> {
    let home = env::var("HOME").ok()?;
    let path = PathBuf::from(home)
        .join(".cache")
        .join("gnome-emoji-input")
        .join("recents.json");
    Some(path)
}

#[tokio::main]
async fn main() -> glib::ExitCode {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
    info!("Starting Preferences app v{}", env!("CARGO_PKG_VERSION"));

    let application = adw::Application::builder()
        .application_id("org.example.EmojiInputPrefs")
        .build();

    application.connect_activate(|app| {
        let settings_backend = gsettings();
        let settings = load_settings(&settings_backend);
        
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
        
        let settings_backend = settings_backend.clone();
        trigger_row.connect_apply(move |row: &adw::EntryRow| {
            let mut s = load_settings(&settings_backend);
            s.trigger_char = row.text().to_string();
            save_settings(&settings_backend, &s);
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

        // Variants Group
        let variants_group = adw::PreferencesGroup::builder()
            .title("Variants")
            .build();
        page.add(&variants_group);

        let variants_label = gtk::Label::new(Some("Variant choices are remembered from the popup."));
        variants_label.set_wrap(true);
        variants_label.set_xalign(0.0);
        variants_group.add(&variants_label);

        let clear_variants_button = gtk::Button::builder()
            .label("Clear Preferred Variants")
            .css_classes(["destructive-action"])
            .build();
        let settings_backend = settings_backend.clone();
        clear_variants_button.connect_clicked(move |_| {
            let empty: [&str; 0] = [];
            let _ = settings_backend.set_strv(VARIANT_PREFS_KEY, &empty);
        });
        variants_group.add(&clear_variants_button);

        window.present();
    });

    application.run()
}
