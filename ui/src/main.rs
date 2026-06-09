use gtk4 as gtk;
use gtk::prelude::*;
use zbus::proxy;
use gio::prelude::*;
use serde::{Deserialize, Serialize};
use futures_util::stream::StreamExt;
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::rc::Rc;
use log::{info, error, debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize, zvariant::Type)]
pub struct Emoji {
    pub char: String,
    pub name: String,
    pub keywords: Vec<String>,
    #[serde(default)]
    pub variants: Vec<String>,
}

const SETTINGS_SCHEMA_ID: &str = "org.example.EmojiInput";
const VARIANT_PREFS_KEY: &str = "variant-preferences";

fn load_variant_preferences(settings: &gio::Settings) -> HashMap<String, String> {
    settings
        .strv(VARIANT_PREFS_KEY)
        .into_iter()
        .filter_map(|entry| {
            entry
                .split_once('=')
                .map(|(base, variant)| (base.to_string(), variant.to_string()))
        })
        .collect()
}

fn save_variant_preferences(settings: &gio::Settings, prefs: &HashMap<String, String>) {
    let values: Vec<String> = prefs
        .iter()
        .map(|(base, variant)| format!("{}={}", base, variant))
        .collect();
    let refs: Vec<&str> = values.iter().map(|value| value.as_str()).collect();
    let _ = settings.set_strv(VARIANT_PREFS_KEY, &refs);
}

fn build_row(
    emoji: &Emoji,
    commit_tx: Rc<tokio::sync::mpsc::Sender<String>>,
    window: &gtk::Window,
    variant_settings: gio::Settings,
    variant_prefs: Rc<RefCell<HashMap<String, String>>>,
) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::builder()
        .selectable(true)
        .activatable(true)
        .build();
    row.add_css_class("emoji-row");

    let outer = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    outer.set_margin_top(8);
    outer.set_margin_bottom(8);
    outer.set_margin_start(10);
    outer.set_margin_end(10);

    let glyph = gtk::Label::new(Some(&emoji.char));
    glyph.add_css_class("title-1");
    glyph.set_width_chars(2);
    glyph.set_xalign(0.0);
    glyph.set_valign(gtk::Align::Center);

    let text_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    text_box.set_hexpand(true);

    let name = gtk::Label::new(Some(&format!(":{}", emoji.name)));
    name.set_xalign(0.0);
    name.set_halign(gtk::Align::Start);
    name.add_css_class("title-4");

    let keywords_text = if emoji.keywords.is_empty() {
        String::from(" ")
    } else {
        emoji.keywords.join(", ")
    };
    let keywords = gtk::Label::new(Some(&keywords_text));
    keywords.set_xalign(0.0);
    keywords.set_halign(gtk::Align::Start);
    keywords.add_css_class("caption");

    text_box.append(&name);
    text_box.append(&keywords);

    let content = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    content.set_hexpand(true);
    content.append(&glyph);
    content.append(&text_box);

    if !emoji.variants.is_empty() {
        let variants_popover = gtk::Popover::new();
        variants_popover.set_has_arrow(true);
        variants_popover.set_autohide(true);

        let variants_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
        variants_box.set_margin_top(8);
        variants_box.set_margin_bottom(8);
        variants_box.set_margin_start(8);
        variants_box.set_margin_end(8);

        let variants_label = gtk::Label::new(Some("Choose variant"));
        variants_label.set_xalign(0.0);
        variants_label.add_css_class("caption");
        variants_box.append(&variants_label);

        let preferred_variant = variant_prefs.borrow().get(&emoji.char).cloned();
        let mut variants = emoji.variants.clone();
        variants.sort_by_key(|variant| {
            if preferred_variant.as_deref() == Some(variant.as_str()) {
                0
            } else {
                1
            }
        });

        for variant in variants {
            let variant_button = gtk::Button::with_label(&variant);
            variant_button.add_css_class("flat");
            variant_button.set_halign(gtk::Align::Fill);
            variant_button.set_hexpand(true);
            variant_button.set_tooltip_text(Some("Commit this variant"));
            if preferred_variant.as_deref() == Some(variant.as_str()) {
                variant_button.add_css_class("suggested-action");
            }

            let commit_tx = commit_tx.clone();
            let variant_settings = variant_settings.clone();
            let variant_prefs = variant_prefs.clone();
            let base_char = emoji.char.clone();
            let popover = variants_popover.clone();
            let window = window.clone();
            variant_button.connect_clicked(move |_| {
                {
                    let mut prefs = variant_prefs.borrow_mut();
                    prefs.insert(base_char.clone(), variant.clone());
                    save_variant_preferences(&variant_settings, &prefs);
                }
                let _ = commit_tx.try_send(variant.clone());
                popover.popdown();
                window.hide();
            });

            variants_box.append(&variant_button);
        }

        variants_popover.set_child(Some(&variants_box));

        let variants_button = gtk::MenuButton::builder()
            .icon_name("view-more-symbolic")
            .popover(&variants_popover)
            .build();
        variants_button.add_css_class("flat");
        variants_button.set_halign(gtk::Align::End);
        variants_button.set_valign(gtk::Align::Center);
        variants_button.set_tooltip_text(Some("Choose variant"));

        content.append(&variants_button);
    }

    outer.append(&content);
    row.set_child(Some(&outer));

    row
}

fn render_results(
    list_box: &gtk::ListBox,
    results: &[Emoji],
    selected_index: i32,
    commit_tx: Rc<tokio::sync::mpsc::Sender<String>>,
    window: &gtk::Window,
    variant_settings: gio::Settings,
    variant_prefs: Rc<RefCell<HashMap<String, String>>>,
) {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }

    for emoji in results {
        list_box.append(&build_row(
            emoji,
            commit_tx.clone(),
            window,
            variant_settings.clone(),
            variant_prefs.clone(),
        ));
    }

    if let Some(row) = list_box.row_at_index(selected_index) {
        list_box.select_row(Some(&row));
    }
}

fn row_count(list_box: &gtk::ListBox) -> i32 {
    let mut count = 0;
    while list_box.row_at_index(count).is_some() {
        count += 1;
    }
    count
}

fn move_selection(list_box: &gtk::ListBox, delta: i32) {
    let count = row_count(list_box);
    if count == 0 {
        return;
    }

    let current = list_box.selected_row().map(|row| row.index()).unwrap_or(0);
    let next = (current + delta).rem_euclid(count);
    if let Some(row) = list_box.row_at_index(next) {
        list_box.select_row(Some(&row));
        row.grab_focus();
    }
}

fn active_popup_parent() -> Option<gtk::Window> {
    gtk::Window::list_toplevels()
        .into_iter()
        .filter_map(|widget| widget.downcast::<gtk::Window>().ok())
        .find(|window| window.is_active())
}

fn current_popup_parent() -> Option<gtk::Window> {
    // Best-effort anchor: use the toplevel under the pointer when available.
    let display = gtk::gdk::Display::default()?;
    let seat = display.default_seat()?;
    let pointer = seat.pointer()?;
    let (surface, _, _) = pointer.surface_at_position();
    let pointer_surface = surface?;
    gtk::Window::list_toplevels()
        .into_iter()
        .filter_map(|widget| widget.downcast::<gtk::Window>().ok())
        .find(|window| {
            window
                .native()
                .and_then(|native| native.surface())
                .is_some_and(|window_surface| {
                    pointer_surface.as_ptr() == window_surface.as_ptr()
                })
        })
}

fn anchor_popup_window(window: &gtk::Window) {
    let parent = current_popup_parent().or_else(active_popup_parent);
    if let Some(parent) = parent {
        if !std::ptr::eq(window.as_ptr(), parent.as_ptr()) {
            window.set_transient_for(Some(&parent));
        }
    }
}

#[proxy(
    interface = "org.example.EmojiInput.Picker",
    default_service = "org.example.EmojiInput.Picker",
    default_path = "/org/example/EmojiInput/Picker"
)]
trait EmojiPicker {
    #[zbus(signal)]
    fn update_results(&self, results: Vec<Emoji>, selected_index: u32) -> zbus::Result<()>;

    fn commit_emoji(&self, text: &str, token: &str) -> zbus::Result<()>;
}

#[tokio::main]
async fn main() -> glib::ExitCode {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let picker_token = env::var("EMOJI_INPUT_PICKER_TOKEN").unwrap_or_default();
    if picker_token.is_empty() {
        warn!("EMOJI_INPUT_PICKER_TOKEN is missing; commit requests will be rejected");
    }

    let application = gtk::Application::builder()
        .application_id("org.example.EmojiInputUI")
        .build();

    application.connect_activate(move |app| {
        let css = gtk::CssProvider::new();
        css.load_from_data(
            "
            window {
                border-radius: 16px;
            }
            .emoji-row {
                border-radius: 12px;
            }
            .emoji-row:selected {
                background: alpha(@accent_bg_color, 0.18);
            }
            .caption {
                opacity: 0.72;
            }
            "
        );
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &css,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        let window = gtk::Window::builder()
            .application(app)
            .title("Emoji Picker")
            .default_width(360)
            .default_height(320)
            .decorated(false)
            .hide_on_close(true)
            .resizable(false)
            .build();
        window.connect_close_request(|window| {
            window.hide();
            glib::Propagation::Stop
        });
        window.connect_is_active_notify(|window| {
            if !window.is_active() {
                window.hide();
            }
        });
        anchor_popup_window(&window);

        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.add_css_class("boxed-list");

        let header = gtk::Box::new(gtk::Orientation::Vertical, 2);
        header.set_margin_top(12);
        header.set_margin_bottom(4);
        header.set_margin_start(14);
        header.set_margin_end(14);

        let title = gtk::Label::new(Some("Emoji Picker"));
        title.set_xalign(0.0);
        title.add_css_class("title-4");

        let subtitle = gtk::Label::new(Some("Enter to commit, Esc to close"));
        subtitle.set_xalign(0.0);
        subtitle.add_css_class("caption");

        header.append(&title);
        header.append(&subtitle);

        let scroller = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .min_content_height(220)
            .build();

        let list_box = gtk::ListBox::builder().build();
        list_box.add_css_class("navigation-sidebar");
        list_box.set_selection_mode(gtk::SelectionMode::Single);
        list_box.set_can_focus(true);
        list_box.set_focus_on_click(true);
        list_box.set_activate_on_single_click(true);
        scroller.set_child(Some(&list_box));

        let footer = gtk::Label::new(Some("Select with mouse or arrow keys"));
        footer.set_margin_top(4);
        footer.set_margin_bottom(10);
        footer.set_margin_start(14);
        footer.set_margin_end(14);
        footer.set_xalign(0.0);
        footer.add_css_class("caption");

        root.append(&header);
        root.append(&scroller);
        root.append(&footer);
        window.set_child(Some(&root));

        let window_clone = window.clone();
        let list_box_clone = list_box.clone();
        let app_clone = app.clone();
        let picker_token = picker_token.clone();
        let variant_settings = gio::Settings::new(SETTINGS_SCHEMA_ID);
        let variant_prefs = Rc::new(RefCell::new(load_variant_preferences(&variant_settings)));
        let variant_prefs_for_signal = variant_prefs.clone();
        variant_settings.connect_changed(Some(VARIANT_PREFS_KEY), move |settings, _| {
            *variant_prefs_for_signal.borrow_mut() = load_variant_preferences(settings);
        });
        let (commit_tx_raw, mut commit_rx) = tokio::sync::mpsc::channel::<String>(16);
        let commit_tx = Rc::new(commit_tx_raw);

        // Store current results for row-activated (click) lookup
        let results_store: Rc<RefCell<Vec<Emoji>>> = Rc::new(RefCell::new(Vec::new()));
        let results_store_for_activate = results_store.clone();
        let window_for_activate = window.clone();
        let commit_tx_for_render = commit_tx.clone();

        list_box.connect_row_activated(move |_, row| {
            let index = row.index();
            let results = results_store_for_activate.borrow();
            if let Some(emoji) = results.get(index as usize) {
                let _ = commit_tx.try_send(emoji.char.clone());
                window_for_activate.hide();
            }
        });

        let window_for_keys = window.clone();
        let list_box_for_keys = list_box.clone();
        let results_store_for_keys = results_store.clone();
        let commit_tx_for_keys = commit_tx.clone();
        let key_controller = gtk::EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, keyval, _, _| {
            match keyval {
                gtk::gdk::Key::Escape => {
                    window_for_keys.hide();
                    glib::Propagation::Stop
                }
                gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter => {
                    if let Some(row) = list_box_for_keys.selected_row() {
                        let results = results_store_for_keys.borrow();
                        if let Some(emoji) = results.get(row.index() as usize) {
                            let _ = commit_tx_for_keys.try_send(emoji.char.clone());
                            window_for_keys.hide();
                        }
                    }
                    glib::Propagation::Stop
                }
                gtk::gdk::Key::Up => {
                    move_selection(&list_box_for_keys, -1);
                    glib::Propagation::Stop
                }
                gtk::gdk::Key::Down => {
                    move_selection(&list_box_for_keys, 1);
                    glib::Propagation::Stop
                }
                gtk::gdk::Key::Left => {
                    move_selection(&list_box_for_keys, -1);
                    glib::Propagation::Stop
                }
                gtk::gdk::Key::Right => {
                    move_selection(&list_box_for_keys, 1);
                    glib::Propagation::Stop
                }
                gtk::gdk::Key::Tab => {
                    move_selection(&list_box_for_keys, 1);
                    glib::Propagation::Stop
                }
                gtk::gdk::Key::ISO_Left_Tab => {
                    move_selection(&list_box_for_keys, -1);
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });
        list_box.add_controller(key_controller);

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

            loop {
                tokio::select! {
                    signal = stream.next() => {
                        match signal {
                            Some(signal) => {
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
                                let results_store = results_store.clone();
                                let commit_tx = commit_tx_for_render.clone();
                                let variant_settings = variant_settings.clone();
                                let variant_prefs = variant_prefs.clone();
                                
                                glib::idle_add_local(move || {
                                    *results_store.borrow_mut() = results.clone();
                                    if results.is_empty() {
                                        window.hide();
                                    } else {
                                        render_results(
                                            &list_box,
                                            &results,
                                            selected_index,
                                            commit_tx.clone(),
                                            &window,
                                            variant_settings.clone(),
                                            variant_prefs.clone(),
                                        );
                                        anchor_popup_window(&window);
                                        window.present();
                                        list_box.grab_focus();
                                    }
                                    glib::ControlFlow::Break
                                });
                            }
                            None => break,
                        }
                    }
                    text = commit_rx.recv() => {
                        match text {
                            Some(t) => {
                                if let Err(e) = proxy.commit_emoji(&t, &picker_token).await {
                                    warn!("commit_emoji failed: {}", e);
                                }
                            }
                            None => break,
                        }
                    }
                }
            }
            info!("Engine stream ended, shutting down UI...");
            app.quit();
        });
    });

    application.run()
}
