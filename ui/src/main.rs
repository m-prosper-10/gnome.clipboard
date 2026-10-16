use futures_util::stream::StreamExt;
use gtk::prelude::*;
use gtk4 as gtk;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::env;
use std::rc::Rc;
use zbus::proxy;

#[derive(Debug, Clone, Serialize, Deserialize, zvariant::Type)]
pub struct Emoji {
    pub char: String,
    pub name: String,
    pub keywords: Vec<String>,
    #[serde(default)]
    pub variants: Vec<String>,
    #[serde(default)]
    pub category: String,
}

const SETTINGS_SCHEMA_ID: &str = "org.example.EmojiInput";
const VARIANT_PREFS_KEY: &str = "variant-preferences";

const CATEGORIES: &[(&str, &str, &str)] = &[
    ("all", "✨", "All Emojis"),
    ("recent", "🕒", "Recently Used"),
    ("smileys", "😀", "Smileys & Emotion"),
    ("people", "👋", "People & Gestures"),
    ("hearts", "❤️", "Hearts"),
    ("objects", "🎉", "Objects & Celebration"),
    ("food", "🍕", "Food & Drink"),
    ("animals", "🐶", "Animals & Nature"),
    ("symbols", "✅", "Symbols"),
];

fn settings_backend() -> Option<gio::Settings> {
    let schema = gio::SettingsSchemaSource::default()
        .and_then(|source| source.lookup(SETTINGS_SCHEMA_ID, true));
    if schema.is_none() {
        warn!(
            "GSettings schema '{}' is unavailable; variant preferences are disabled",
            SETTINGS_SCHEMA_ID
        );
    }
    schema.map(|_| gio::Settings::new(SETTINGS_SCHEMA_ID))
}

fn load_variant_preferences(settings: Option<&gio::Settings>) -> HashMap<String, String> {
    let Some(settings) = settings else {
        return HashMap::new();
    };

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

fn save_variant_preferences(settings: Option<&gio::Settings>, prefs: &HashMap<String, String>) {
    let Some(settings) = settings else {
        return;
    };

    let values: Vec<String> = prefs
        .iter()
        .map(|(base, variant)| format!("{}={}", base, variant))
        .collect();
    let refs: Vec<&str> = values.iter().map(|value| value.as_str()).collect();
    let _ = settings.set_strv(VARIANT_PREFS_KEY, refs);
}

fn matches_local_query(emoji: &Emoji, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    emoji.name.to_lowercase().contains(&q)
        || emoji
            .keywords
            .iter()
            .any(|k| k.to_lowercase().contains(&q))
}

fn clear_flow_box(flow_box: &gtk::FlowBox) {
    while let Some(child) = flow_box.first_child() {
        flow_box.remove(&child);
    }
}

fn build_emoji_tile(
    emoji: &Emoji,
    commit_tx: Rc<tokio::sync::mpsc::Sender<String>>,
    window: &gtk::Window,
    variant_settings: Option<gio::Settings>,
    variant_prefs: Rc<RefCell<HashMap<String, String>>>,
) -> gtk::FlowBoxChild {
    let child = gtk::FlowBoxChild::new();
    child.add_css_class("emoji-tile-child");
    child.set_can_focus(false);

    let preferred = variant_prefs.borrow().get(&emoji.char).cloned();
    let display_char = preferred.clone().unwrap_or_else(|| emoji.char.clone());

    let button = gtk::Button::with_label(&display_char);
    button.add_css_class("flat");
    button.add_css_class("emoji-tile");
    button.set_can_focus(false);
    button.set_focus_on_click(false);

    let tooltip = if emoji.variants.is_empty() {
        format!(":{} ({})", emoji.name, emoji.keywords.join(", "))
    } else {
        button.add_css_class("has-variants");
        format!(
            ":{} ({}) · Right-click for variants",
            emoji.name,
            emoji.keywords.join(", ")
        )
    };
    button.set_tooltip_text(Some(&tooltip));

    let commit_tx_click = commit_tx.clone();
    let window_click = window.clone();
    let commit_char = display_char.clone();
    button.connect_clicked(move |_| {
        window_click.hide();
        let _ = commit_tx_click.try_send(commit_char.clone());
    });

    if !emoji.variants.is_empty() {
        let popover = gtk::Popover::new();
        popover.set_has_arrow(true);
        popover.set_autohide(true);
        popover.set_parent(&button);

        let variants_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        variants_box.set_margin_top(4);
        variants_box.set_margin_bottom(4);
        variants_box.set_margin_start(4);
        variants_box.set_margin_end(4);

        let mut all_options = Vec::with_capacity(emoji.variants.len() + 1);
        all_options.push(emoji.char.clone());
        all_options.extend(emoji.variants.iter().cloned());

        for variant in all_options {
            let v_btn = gtk::Button::with_label(&variant);
            v_btn.add_css_class("flat");
            v_btn.add_css_class("emoji-tile");
            if preferred.as_deref() == Some(variant.as_str()) {
                v_btn.add_css_class("suggested-action");
            }

            let commit_tx_v = commit_tx.clone();
            let variant_settings_v = variant_settings.clone();
            let variant_prefs_v = variant_prefs.clone();
            let base_char = emoji.char.clone();
            let popover_v = popover.clone();
            let window_v = window.clone();
            v_btn.connect_clicked(move |_| {
                {
                    let mut prefs = variant_prefs_v.borrow_mut();
                    prefs.insert(base_char.clone(), variant.clone());
                    save_variant_preferences(variant_settings_v.as_ref(), &prefs);
                }
                popover_v.popdown();
                window_v.hide();
                let _ = commit_tx_v.try_send(variant.clone());
            });
            variants_box.append(&v_btn);
        }

        popover.set_child(Some(&variants_box));

        let right_click = gtk::GestureClick::new();
        right_click.set_button(3);
        let popover_open = popover.clone();
        right_click.connect_pressed(move |gesture, _, _, _| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            popover_open.popup();
        });
        button.add_controller(right_click);
    }

    child.set_child(Some(&button));
    child
}

fn populate_grid(
    flow_box: &gtk::FlowBox,
    emojis: &[Emoji],
    selected_index: Option<i32>,
    commit_tx: Rc<tokio::sync::mpsc::Sender<String>>,
    window: &gtk::Window,
    variant_settings: Option<gio::Settings>,
    variant_prefs: Rc<RefCell<HashMap<String, String>>>,
) {
    clear_flow_box(flow_box);
    for emoji in emojis {
        flow_box.insert(
            &build_emoji_tile(
                emoji,
                commit_tx.clone(),
                window,
                variant_settings.clone(),
                variant_prefs.clone(),
            ),
            -1,
        );
    }

    if let Some(idx) = selected_index {
        if let Some(child) = flow_box.child_at_index(idx) {
            flow_box.select_child(&child);
        }
    }
}

#[derive(Clone)]
struct PickerViewState {
    results: Vec<Emoji>,
    recents: Vec<Emoji>,
    selected_index: i32,
    active_category: String,
    local_query: String,
    engine_query: String,
}

#[allow(clippy::too_many_arguments)]
fn refresh_picker_view(
    state: &PickerViewState,
    recents_section: &gtk::Box,
    recents_flow: &gtk::FlowBox,
    main_label: &gtk::Label,
    main_flow: &gtk::FlowBox,
    empty_label: &gtk::Label,
    commit_tx: Rc<tokio::sync::mpsc::Sender<String>>,
    window: &gtk::Window,
    variant_settings: Option<gio::Settings>,
    variant_prefs: Rc<RefCell<HashMap<String, String>>>,
) {
    let effective_query = if !state.local_query.is_empty() {
        state.local_query.as_str()
    } else {
        state.engine_query.as_str()
    };

    let show_recents_strip =
        effective_query.is_empty() && state.active_category == "all" && !state.recents.is_empty();

    if show_recents_strip {
        recents_section.set_visible(true);
        let top_recents: Vec<Emoji> = state.recents.iter().take(7).cloned().collect();
        populate_grid(
            recents_flow,
            &top_recents,
            None,
            commit_tx.clone(),
            window,
            variant_settings.clone(),
            variant_prefs.clone(),
        );
    } else {
        recents_section.set_visible(false);
    }

    let displayed: Vec<Emoji> = if !state.local_query.is_empty() {
        state
            .results
            .iter()
            .filter(|e| matches_local_query(e, &state.local_query))
            .cloned()
            .collect()
    } else if !effective_query.is_empty() {
        state.results.clone()
    } else if state.active_category == "recent" {
        state.recents.clone()
    } else if state.active_category == "all" {
        state.results.clone()
    } else {
        state
            .results
            .iter()
            .filter(|e| e.category == state.active_category)
            .cloned()
            .collect()
    };

    let section_title = if !effective_query.is_empty() {
        format!("Results for \"{}\"", effective_query)
    } else {
        CATEGORIES
            .iter()
            .find(|(id, _, _)| *id == state.active_category)
            .map(|(_, _, label)| (*label).to_string())
            .unwrap_or_else(|| "All Emojis".to_string())
    };
    main_label.set_text(&section_title);

    if displayed.is_empty() {
        main_flow.set_visible(false);
        empty_label.set_visible(true);
    } else {
        empty_label.set_visible(false);
        main_flow.set_visible(true);
        let sel = if !effective_query.is_empty() || state.active_category == "all" {
            Some(state.selected_index)
        } else {
            None
        };
        populate_grid(
            main_flow,
            &displayed,
            sel,
            commit_tx,
            window,
            variant_settings,
            variant_prefs,
        );
    }
}

#[proxy(
    interface = "org.example.EmojiInput.Picker",
    default_service = "org.example.EmojiInput.Picker",
    default_path = "/org/example/EmojiInput/Picker"
)]
trait EmojiPicker {
    #[zbus(signal)]
    fn update_results(
        &self,
        results: Vec<Emoji>,
        recents: Vec<Emoji>,
        selected_index: u32,
        query: String,
    ) -> zbus::Result<()>;

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
                border-radius: 12px;
                border: 1px solid alpha(@borders, 0.85);
                background: @window_bg_color;
                box-shadow: 0 8px 24px alpha(black, 0.25);
            }
            .picker-root {
                padding: 8px;
            }
            .cat-tab {
                min-width: 28px;
                min-height: 28px;
                padding: 2px 6px;
                border-radius: 8px;
                font-size: 14px;
            }
            .cat-tab.active-cat {
                background: alpha(@accent_bg_color, 0.22);
                border: 1px solid alpha(@accent_bg_color, 0.5);
            }
            .emoji-tile-child {
                padding: 0;
                border-radius: 8px;
            }
            .emoji-tile-child:selected {
                background: alpha(@accent_bg_color, 0.28);
                outline: 1px solid alpha(@accent_bg_color, 0.7);
            }
            .emoji-tile {
                min-width: 38px;
                min-height: 38px;
                padding: 0;
                border-radius: 8px;
                font-size: 21px;
            }
            .emoji-tile:hover {
                background: alpha(@view_fg_color, 0.08);
            }
            .has-variants {
                border-bottom: 2px solid alpha(@accent_bg_color, 0.45);
            }
            .section-title {
                font-size: 11px;
                font-weight: 600;
                opacity: 0.7;
                margin-left: 4px;
            }
            .picker-footer {
                opacity: 0.6;
                font-size: 10px;
                padding: 2px 4px 0 4px;
            }
            ",
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
            .default_width(348)
            .decorated(false)
            .focusable(false)
            .hide_on_close(true)
            .resizable(false)
            .build();

        let root = gtk::Box::new(gtk::Orientation::Vertical, 6);
        root.add_css_class("picker-root");

        // Top bar: category tabs + close button
        let top_bar = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        let cat_buttons: Rc<RefCell<Vec<(String, gtk::Button)>>> = Rc::new(RefCell::new(Vec::new()));

        let state = Rc::new(RefCell::new(PickerViewState {
            results: Vec::new(),
            recents: Vec::new(),
            selected_index: 0,
            active_category: "all".to_string(),
            local_query: String::new(),
            engine_query: String::new(),
        }));

        let (commit_tx_raw, mut commit_rx) = tokio::sync::mpsc::channel::<String>(16);
        let commit_tx = Rc::new(commit_tx_raw);

        let variant_settings = settings_backend();
        let variant_prefs = Rc::new(RefCell::new(load_variant_preferences(
            variant_settings.as_ref(),
        )));
        let variant_prefs_for_signal = variant_prefs.clone();
        if let Some(settings) = variant_settings.as_ref() {
            settings.connect_changed(Some(VARIANT_PREFS_KEY), move |settings, _| {
                *variant_prefs_for_signal.borrow_mut() = load_variant_preferences(Some(settings));
            });
        }

        // Search bar
        let search_entry = gtk::SearchEntry::builder()
            .placeholder_text("Search emojis (or keep typing after :)")
            .build();
        search_entry.set_focus_on_click(true);

        // Recents section
        let recents_section = gtk::Box::new(gtk::Orientation::Vertical, 4);
        let recents_label = gtk::Label::new(Some("Recently Used"));
        recents_label.set_xalign(0.0);
        recents_label.add_css_class("section-title");

        let recents_flow = gtk::FlowBox::builder()
            .max_children_per_line(7)
            .min_children_per_line(7)
            .selection_mode(gtk::SelectionMode::None)
            .homogeneous(true)
            .row_spacing(2)
            .column_spacing(2)
            .build();
        recents_section.append(&recents_label);
        recents_section.append(&recents_flow);

        // Main emoji grid
        let main_label = gtk::Label::new(Some("All Emojis"));
        main_label.set_xalign(0.0);
        main_label.add_css_class("section-title");

        let main_flow = gtk::FlowBox::builder()
            .max_children_per_line(7)
            .min_children_per_line(7)
            .selection_mode(gtk::SelectionMode::Single)
            .homogeneous(true)
            .row_spacing(2)
            .column_spacing(2)
            .build();
        main_flow.set_can_focus(false);

        let empty_label = gtk::Label::new(Some("No matching emojis"));
        empty_label.add_css_class("section-title");
        empty_label.set_margin_top(16);
        empty_label.set_margin_bottom(16);
        empty_label.set_visible(false);

        let grid_container = gtk::Box::new(gtk::Orientation::Vertical, 6);
        grid_container.append(&recents_section);
        grid_container.append(&main_label);
        grid_container.append(&main_flow);
        grid_container.append(&empty_label);

        let scroller = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .propagate_natural_height(true)
            .max_content_height(260)
            .build();
        scroller.set_child(Some(&grid_container));

        let footer = gtk::Label::new(Some("↑↓←→ Navigate  ·  ↵ Insert  ·  Right-click variants  ·  Esc Close"));
        footer.add_css_class("picker-footer");
        footer.set_xalign(0.0);

        // Build category tab buttons
        for (id, icon, label) in CATEGORIES {
            let btn = gtk::Button::with_label(icon);
            btn.add_css_class("flat");
            btn.add_css_class("cat-tab");
            btn.set_can_focus(false);
            btn.set_focus_on_click(false);
            btn.set_tooltip_text(Some(label));
            if *id == "all" {
                btn.add_css_class("active-cat");
            }
            top_bar.append(&btn);

            let cat_id = (*id).to_string();
            let state_c = state.clone();
            let cat_buttons_c = cat_buttons.clone();
            let recents_section_c = recents_section.clone();
            let recents_flow_c = recents_flow.clone();
            let main_label_c = main_label.clone();
            let main_flow_c = main_flow.clone();
            let empty_label_c = empty_label.clone();
            let commit_tx_c = commit_tx.clone();
            let window_c = window.clone();
            let variant_settings_c = variant_settings.clone();
            let variant_prefs_c = variant_prefs.clone();

            btn.connect_clicked(move |_| {
                {
                    let mut st = state_c.borrow_mut();
                    st.active_category = cat_id.clone();
                }
                for (cid, b) in cat_buttons_c.borrow().iter() {
                    if *cid == cat_id {
                        b.add_css_class("active-cat");
                    } else {
                        b.remove_css_class("active-cat");
                    }
                }
                let st = state_c.borrow().clone();
                refresh_picker_view(
                    &st,
                    &recents_section_c,
                    &recents_flow_c,
                    &main_label_c,
                    &main_flow_c,
                    &empty_label_c,
                    commit_tx_c.clone(),
                    &window_c,
                    variant_settings_c.clone(),
                    variant_prefs_c.clone(),
                );
            });

            cat_buttons.borrow_mut().push(((*id).to_string(), btn));
        }

        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        top_bar.append(&spacer);

        let close_btn = gtk::Button::from_icon_name("window-close-symbolic");
        close_btn.add_css_class("flat");
        close_btn.add_css_class("cat-tab");
        close_btn.set_can_focus(false);
        close_btn.set_focus_on_click(false);
        close_btn.set_tooltip_text(Some("Close (Esc)"));
        let window_close_btn = window.clone();
        let commit_tx_close_btn = commit_tx.clone();
        close_btn.connect_clicked(move |_| {
            window_close_btn.hide();
            let _ = commit_tx_close_btn.try_send(String::new());
        });
        top_bar.append(&close_btn);

        let updating_search = Rc::new(Cell::new(false));
        let updating_search_changed = updating_search.clone();
        let state_search = state.clone();
        let recents_section_s = recents_section.clone();
        let recents_flow_s = recents_flow.clone();
        let main_label_s = main_label.clone();
        let main_flow_s = main_flow.clone();
        let empty_label_s = empty_label.clone();
        let commit_tx_s = commit_tx.clone();
        let window_s = window.clone();
        let variant_settings_s = variant_settings.clone();
        let variant_prefs_s = variant_prefs.clone();

        search_entry.connect_search_changed(move |entry| {
            if updating_search_changed.get() {
                return;
            }
            {
                let mut st = state_search.borrow_mut();
                st.local_query = entry.text().to_string();
            }
            let st = state_search.borrow().clone();
            refresh_picker_view(
                &st,
                &recents_section_s,
                &recents_flow_s,
                &main_label_s,
                &main_flow_s,
                &empty_label_s,
                commit_tx_s.clone(),
                &window_s,
                variant_settings_s.clone(),
                variant_prefs_s.clone(),
            );
        });

        root.append(&top_bar);
        root.append(&search_entry);
        root.append(&scroller);
        root.append(&footer);
        window.set_child(Some(&root));

        let commit_tx_for_close = commit_tx.clone();
        window.connect_close_request(move |window| {
            window.hide();
            let _ = commit_tx_for_close.try_send(String::new());
            glib::Propagation::Stop
        });

        let app_clone = app.clone();
        let window_clone = window.clone();
        let state_signal = state.clone();
        let updating_search_signal = updating_search.clone();
        let search_entry_signal = search_entry.clone();
        let recents_section_sig = recents_section.clone();
        let recents_flow_sig = recents_flow.clone();
        let main_label_sig = main_label.clone();
        let main_flow_sig = main_flow.clone();
        let empty_label_sig = empty_label.clone();
        let commit_tx_sig = commit_tx.clone();
        let variant_settings_sig = variant_settings.clone();
        let variant_prefs_sig = variant_prefs.clone();

        let picker_token = picker_token.clone();
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
                    error!("Failed to create EmojiPicker proxy: {}", e);
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
                                let recents = args.recents;
                                let selected_index = args.selected_index as i32;
                                let query = args.query;

                                debug!(
                                    "Received {} results, {} recents, selected_index: {}, query: '{}'",
                                    results.len(),
                                    recents.len(),
                                    selected_index,
                                    query
                                );

                                let window = window_clone.clone();
                                let state = state_signal.clone();
                                let updating_search = updating_search_signal.clone();
                                let search_entry = search_entry_signal.clone();
                                let recents_section = recents_section_sig.clone();
                                let recents_flow = recents_flow_sig.clone();
                                let main_label = main_label_sig.clone();
                                let main_flow = main_flow_sig.clone();
                                let empty_label = empty_label_sig.clone();
                                let commit_tx = commit_tx_sig.clone();
                                let variant_settings = variant_settings_sig.clone();
                                let variant_prefs = variant_prefs_sig.clone();

                                glib::idle_add_local_once(move || {
                                    let is_closing = results.is_empty() && query.is_empty();
                                    if is_closing {
                                        window.hide();
                                        let mut st = state.borrow_mut();
                                        st.local_query.clear();
                                        st.engine_query.clear();
                                        updating_search.set(true);
                                        search_entry.set_text("");
                                        updating_search.set(false);
                                    } else {
                                        {
                                            let mut st = state.borrow_mut();
                                            st.results = results;
                                            st.recents = recents;
                                            st.selected_index = selected_index;
                                            st.engine_query = query.clone();
                                            st.local_query.clear();
                                        }
                                        updating_search.set(true);
                                        search_entry.set_text(&query);
                                        updating_search.set(false);

                                        let st = state.borrow().clone();
                                        refresh_picker_view(
                                            &st,
                                            &recents_section,
                                            &recents_flow,
                                            &main_label,
                                            &main_flow,
                                            &empty_label,
                                            commit_tx.clone(),
                                            &window,
                                            variant_settings.clone(),
                                            variant_prefs.clone(),
                                        );
                                        window.set_visible(true);
                                    }
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
