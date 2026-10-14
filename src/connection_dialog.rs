// Module: magic_hotspot.connection_dialog
// Purpose: Present the approved connection and open-Wi-Fi dialog layouts.
// Created: 2026-10-11
// Architecture: Shared GTK layout; network workers remain outside presentation.

use crate::i18n::{tr, trf};
use gtk::prelude::*;

const ICON_SIZE: i32 = 64;
const CONTENT_MARGIN: i32 = 24;
const ROW_TOP: i32 = 40;
const ICON_TEXT_GAP: i32 = 16;
const BUTTON_GAP: i32 = 32;
const BUTTON_WIDTH: i32 = 112;
const BUTTON_HEIGHT: i32 = 40;
const INFORMATION: &[u8] = include_bytes!("../assets/dialog-information.svg");
const WARNING: &[u8] = include_bytes!("../assets/dialog-warning.svg");

pub struct ConnectionDialog {
    dialog: gtk::Dialog,
    icon: gtk::Image,
    message: gtk::Label,
}

impl ConnectionDialog {
    pub fn new(parent: &gtk::ApplicationWindow, target: &str) -> Self {
        let popup = Self::build(
            parent,
            "Connection",
            &trf("Connecting to {target}…", &[("target", target)]),
            464,
            INFORMATION,
            &[("Close", gtk::ResponseType::Close)],
        );
        // Destruction runs on GTK's thread; owned Rust references remain valid.
        popup
            .dialog
            .connect_response(|dialog, _| unsafe { dialog.destroy() });
        popup.dialog.show_all();
        popup
    }

    pub fn disconnecting(parent: &gtk::ApplicationWindow) -> Self {
        let popup = Self::new(parent, "device");
        popup.message.set_text(&tr("Disconnecting…"));
        popup
    }

    fn build(
        parent: &gtk::ApplicationWindow,
        title: &str,
        text: &str,
        width: i32,
        artwork: &'static [u8],
        choices: &[(&'static str, gtk::ResponseType)],
    ) -> Self {
        let dialog = gtk::Dialog::with_buttons(
            Some(&tr(title)),
            Some(parent),
            gtk::DialogFlags::DESTROY_WITH_PARENT | gtk::DialogFlags::USE_HEADER_BAR,
            &[],
        );
        dialog.set_default_size(width, -1);
        dialog.set_position(gtk::WindowPosition::CenterOnParent);
        dialog.set_resizable(false);
        if let Some(header) = dialog
            .header_bar()
            .and_then(|w| w.downcast::<gtk::HeaderBar>().ok())
        {
            header.set_show_close_button(false);
        }
        // Theme action bars may stretch buttons or put them at the window edge.
        // Our centered row is shared by every dialog, using logical GTK units.
        let content = dialog.content_area();
        content.set_spacing(0);
        content.set_border_width(0);
        content.set_margin_start(CONTENT_MARGIN);
        content.set_margin_end(CONTENT_MARGIN);
        content.set_margin_top(ROW_TOP);
        content.set_margin_bottom(32);

        let row = gtk::Box::new(gtk::Orientation::Horizontal, ICON_TEXT_GAP);
        let icon = gtk::Image::from_gicon(&vector_icon(artwork), gtk::IconSize::Dialog);
        icon.set_pixel_size(ICON_SIZE);
        icon.set_size_request(ICON_SIZE, ICON_SIZE);
        icon.set_valign(gtk::Align::Center);
        row.pack_start(&icon, false, false, 0);
        let message = gtk::Label::new(Some(&tr(text)));
        message.set_line_wrap(true);
        message.set_line_wrap_mode(gtk::pango::WrapMode::WordChar);
        message.set_max_width_chars(44);
        message.set_justify(gtk::Justification::Left);
        message.set_xalign(0.0);
        message.set_valign(gtk::Align::Center);
        message.set_hexpand(true);
        row.pack_start(&message, true, true, 0);
        content.pack_start(&row, false, false, 0);

        let styles = gtk::CssProvider::new();
        styles.load_from_data(b"button { min-width: 0; min-height: 0; padding: 0 12px; border-radius: 8px; } button.confirm { background: #0f6cbd; color: white; border-color: #0f6cbd; } label { font-size: 15px; }")
            .expect("dialog styles are valid");
        message
            .style_context()
            .add_provider(&styles, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 16);
        actions.set_halign(gtk::Align::Center);
        // Reserve the icon column so actions center under the message column.
        actions.set_margin_start(ICON_SIZE + ICON_TEXT_GAP);
        actions.set_margin_top(BUTTON_GAP);
        for &(label, response) in choices {
            let button = crate::i18n::button(label);
            button.set_size_request(BUTTON_WIDTH, BUTTON_HEIGHT);
            button
                .style_context()
                .add_provider(&styles, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
            if response == gtk::ResponseType::Yes {
                button.style_context().add_class("confirm");
            }
            let weak = dialog.downgrade();
            button.connect_clicked(move |_| {
                if let Some(dialog) = weak.upgrade() {
                    dialog.response(response);
                }
            });
            actions.pack_start(&button, false, false, 0);
        }
        content.pack_end(&actions, false, false, 0);
        Self {
            dialog,
            icon,
            message,
        }
    }

    pub fn finish(self, result: Result<String, String>, status: &gtk::Label) {
        match result {
            Ok(message) => {
                status.set_text(&tr(&message));
                unsafe { self.dialog.destroy() };
            }
            Err(error) => {
                status.set_text(&tr("Connection failed"));
                if !self.dialog.is_visible() {
                    return;
                }
                self.dialog.set_title(&tr("Connection failed"));
                if let Some(header) = self
                    .dialog
                    .header_bar()
                    .and_then(|w| w.downcast::<gtk::HeaderBar>().ok())
                {
                    header.set_title(Some(&tr("Connection failed")));
                }
                self.icon
                    .set_from_gicon(&vector_icon(WARNING), gtk::IconSize::Dialog);
                // Keep the same icon allocation; only the artwork and message change.
                let error = error.split("\nHint:").next().unwrap_or(&error).trim();
                self.message.set_text(&tr(error
                    .trim_start_matches("Error: ")
                    .trim_start_matches("Connection activation failed: ")));
                self.dialog.set_default_size(528, -1);
            }
        }
    }
}

pub fn confirm_open_wifi(parent: &gtk::ApplicationWindow) -> bool {
    let popup = ConnectionDialog::build(
        parent,
        "Connect to Wi-Fi",
        "This network does not encrypt your connection. Connect anyway?",
        480,
        WARNING,
        &[
            ("No", gtk::ResponseType::No),
            ("Yes", gtk::ResponseType::Yes),
        ],
    );
    popup.dialog.set_modal(true);
    popup.dialog.show_all();
    let accepted = popup.dialog.run() == gtk::ResponseType::Yes;
    unsafe { popup.dialog.destroy() };
    accepted
}

fn vector_icon(artwork: &'static [u8]) -> gtk::gio::BytesIcon {
    // GTK renders the embedded SVG at the monitor scale, rather than enlarging a bitmap.
    gtk::gio::BytesIcon::new(&glib::Bytes::from_static(artwork))
}
