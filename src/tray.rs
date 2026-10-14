// Module: magic_hotspot.tray
// Purpose: Native Ubuntu GNOME and XFCE tray with Open / Quit actions.
// Created: 2026-10-09
// Architecture: Ayatana exports the SVG and menu over D-Bus; the desktop owns
// scaling, positioning and input grabs. All GTK calls stay on the main thread.

use glib::translate::{ToGlibPtr, from_glib_full};
use gtk::{ApplicationWindow, prelude::*};
use std::{cell::Cell, ffi::CString, rc::Rc};

pub struct Tray {
    object: glib::Object,
    _icons: tempfile::TempDir,
    _menu: gtk::Menu,
}

impl Tray {
    pub fn new(window: &ApplicationWindow) -> Self {
        let icons = tempfile::tempdir().expect("tray icon directory");
        let path = icons.path().join("magic-hotspot.svg");
        std::fs::write(&path, include_bytes!("../assets/magic-hotspot.svg")).expect("tray SVG");
        let name = CString::new(path.to_string_lossy().as_bytes()).expect("tray icon path");
        let menu = gtk::Menu::new();
        let open = crate::i18n::menu_item("Open");
        let anchor = Rc::new(Cell::new(None));
        let open_anchor = anchor.clone();
        let weak = window.downgrade();
        open.connect_activate(move |_| {
            if let Some(window) = weak.upgrade() {
                crate::window::present_from_tray(&window, open_anchor.get());
            }
        });
        let quit = crate::i18n::menu_item("Quit");
        let weak = window.downgrade();
        quit.connect_activate(move |_| {
            if let Some(app) = weak.upgrade().and_then(|w| w.application()) {
                app.quit();
            }
        });
        menu.append(&open);
        menu.append(&quit);
        menu.show_all();
        // The constructor returns an owned GObject. Ayatana retains the menu
        // and copies the icon path; keep its SVG alive until the indicator drops.
        let object: glib::Object = unsafe {
            let pointer = app_indicator_new(c"magic-hotspot".as_ptr(), name.as_ptr(), 0);
            assert!(!pointer.is_null(), "native tray indicator");
            app_indicator_set_menu(pointer, menu.to_glib_none().0);
            app_indicator_set_secondary_activate_target(
                pointer,
                open.upcast_ref::<gtk::Widget>().to_glib_none().0,
            );
            app_indicator_set_status(pointer, 1);
            from_glib_full(pointer)
        };
        // The native menu opens while the pointer is still on its panel icon.
        // Retain that anchor instead of using the later Open-row pointer position.
        // The parser returns owned references to its existing cached menu objects.
        let root: glib::Object = unsafe {
            from_glib_full(dbusmenu_gtk_parse_menu_structure(
                menu.upcast_ref::<gtk::Widget>().to_glib_none().0,
            ))
        };
        let weak = window.downgrade();
        root.connect_local("about-to-show", false, move |_| {
            if let Some(window) = weak.upgrade() {
                anchor.set(crate::window::pointer_position(&window));
            }
            Some(false.to_value())
        });
        let item: glib::Object = unsafe {
            from_glib_full(dbusmenu_gtk_parse_menu_structure(
                open.upcast_ref::<gtk::Widget>().to_glib_none().0,
            ))
        };
        let weak = window.downgrade();
        // GTK's activate signal loses the D-Bus click timestamp. Preserve it for
        // the window manager's normal user-activation/focus policy.
        item.connect_local("item-activated", false, move |values| {
            if let Some(window) = weak.upgrade() {
                window.present_with_time(values[1].get::<u32>().expect("menu activation time"));
            }
            None
        });
        Self {
            object,
            _icons: icons,
            _menu: menu,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.object.property::<bool>("connected")
    }

    pub fn stop(&self) {
        unsafe { app_indicator_set_status(self.object.as_ptr(), 0) }
    }
}

// Use the installed GTK3 runtime ABI. The package records this SONAME dependency.
#[link(
    name = "libayatana-appindicator3.so.1",
    kind = "dylib",
    modifiers = "+verbatim"
)]
unsafe extern "C" {
    fn app_indicator_new(
        id: *const std::ffi::c_char,
        icon: *const std::ffi::c_char,
        category: i32,
    ) -> *mut glib::gobject_ffi::GObject;
    fn app_indicator_set_menu(
        indicator: *mut glib::gobject_ffi::GObject,
        menu: *mut gtk::ffi::GtkMenu,
    );
    fn app_indicator_set_secondary_activate_target(
        indicator: *mut glib::gobject_ffi::GObject,
        widget: *mut gtk::ffi::GtkWidget,
    );
    fn app_indicator_set_status(indicator: *mut glib::gobject_ffi::GObject, status: i32);
}

// Public parser API: returns a full reference to the cached GObject; GTK thread only.
#[link(
    name = "libdbusmenu-gtk3.so.4",
    kind = "dylib",
    modifiers = "+verbatim"
)]
unsafe extern "C" {
    fn dbusmenu_gtk_parse_menu_structure(
        widget: *mut gtk::ffi::GtkWidget,
    ) -> *mut glib::gobject_ffi::GObject;
}
