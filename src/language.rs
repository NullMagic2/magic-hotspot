// Module: magic_hotspot.language
// Purpose: Present Chop Chop's four-language flag picker in the header.
// Created: 2026-10-13
// Architecture: One popover; selection updates existing UI without reconnecting.
use crate::i18n::{self, Lang};
use gtk::prelude::*;
use std::rc::Rc;
fn flag(language: Lang) -> gtk::Image {
    let loader = gdk_pixbuf::PixbufLoader::with_type("svg").expect("SVG loader");
    loader.set_size(30, 20);
    loader.write(language.flag()).expect("embedded flag");
    loader.close().expect("embedded flag");
    gtk::Image::from_pixbuf(loader.pixbuf().as_ref())
}
pub fn picker(changed: impl Fn(Result<(), String>) + 'static) -> gtk::MenuButton {
    let button = gtk::MenuButton::new();
    button.style_context().add_class("language-button");
    button.set_widget_name("language-picker");
    i18n::tooltip(&button, "Language");
    let icon = flag(i18n::lang());
    button.add(&icon);
    let popover = gtk::Popover::new(Some(&button));
    popover.style_context().add_class("language-popover");
    let content = gtk::Box::new(gtk::Orientation::Vertical, 2);
    content.set_border_width(8);
    let title = i18n::label("Language");
    title.set_xalign(0.0);
    title.set_margin_start(8);
    title.set_margin_bottom(6);
    content.pack_start(&title, false, false, 0);
    let mut rows = Vec::new();
    for language in Lang::ALL {
        let row = gtk::Button::new();
        row.set_relief(gtk::ReliefStyle::None);
        row.style_context().add_class("language-row");
        row.set_widget_name(&format!("language-{}", language.code()));
        let box_ = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        box_.pack_start(&flag(language), false, false, 0);
        let label = gtk::Label::new(Some(language.native_name()));
        label.set_xalign(0.0);
        label.set_hexpand(true);
        box_.pack_start(&label, true, true, 0);
        let check = gtk::Image::from_icon_name(Some("emblem-ok-symbolic"), gtk::IconSize::Menu);
        check.set_no_show_all(true);
        check.set_visible(language == i18n::lang());
        box_.pack_end(&check, false, false, 0);
        row.add(&box_);
        content.pack_start(&row, false, false, 0);
        rows.push((language, row, check));
    }
    content.show_all();
    popover.add(&content);
    button.set_popover(Some(&popover));
    let rows = Rc::new(rows);
    let changed = Rc::new(changed);
    for (language, row, _) in rows.iter() {
        let (language, rows, icon, popover, changed) = (
            *language,
            rows.clone(),
            icon.clone(),
            popover.clone(),
            changed.clone(),
        );
        row.connect_clicked(move |_| {
            popover.popdown();
            if language == i18n::lang() {
                return;
            }
            let result = i18n::set_lang(language);
            icon.set_from_pixbuf(flag(language).pixbuf().as_ref());
            for (candidate, _, check) in rows.iter() {
                check.set_visible(*candidate == language);
            }
            changed(result);
        });
    }
    button
}
