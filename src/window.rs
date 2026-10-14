// Module: magic_hotspot.window
// Purpose: Hide minimized windows in the tray and restore normal window state.
// Created: 2026-10-09
// Architecture: GTK lifecycle helpers shared by the application and tray module.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{ApplicationWindow, Button, HeaderBar, Widget, Window};

use crate::tray::Tray;

const SCREEN_MARGIN: i32 = 12;

pub fn is_gnome_wayland() -> bool {
    std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("wayland")
        && std::env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .split(':')
            .any(|desktop| desktop.eq_ignore_ascii_case("gnome"))
}

pub fn present(window: &impl IsA<Window>) {
    let window = window.as_ref();
    let was_visible = window.is_visible();
    window.set_skip_taskbar_hint(false);
    window.set_skip_pager_hint(false);
    window.deiconify();
    window.show_all();
    window.present();
    if !was_visible {
        // GTK otherwise focuses the first switch when mapping the window.
        // Leave Tab navigation intact without selecting a setting on Open.
        gtk::prelude::GtkWindowExt::set_focus(window, None::<&Widget>);
        window.set_focus_visible(false);
    }
}

/// The user requested tray placement for every foreground activation.
pub fn present_application(window: &ApplicationWindow) {
    if is_gnome_wayland() {
        present_from_tray(window, None);
    } else {
        // X11 retains the position established by the native tray menu.
        present(window);
    }
}

pub fn enable_gnome_companion() {
    if !is_gnome_wayland() {
        return;
    }
    let Some(schema) = gtk::gio::SettingsSchemaSource::default()
        .and_then(|source| source.lookup("org.gnome.shell", true))
    else {
        return;
    };
    let settings = gtk::gio::Settings::new_full(&schema, None::<&gtk::gio::SettingsBackend>, None);
    let mut enabled: Vec<String> = settings
        .strv("enabled-extensions")
        .iter()
        .map(ToString::to_string)
        .collect();
    if !enabled.iter().any(|name| name == "magic-hotspot@local") {
        enabled.push("magic-hotspot@local".into());
        if let Err(error) = settings.set_strv("enabled-extensions", enabled) {
            eprintln!("Could not enable Magic Hotspot tray placement: {error}");
        }
    }
}

fn place_with_companion(window: &ApplicationWindow, attempts: u8) {
    let Some(bus) = window.application().and_then(|app| app.dbus_connection()) else {
        return;
    };
    let weak = window.downgrade();
    bus.call(
        Some("org.gnome.Shell"),
        "/com/example/MagicHotspot/Tray",
        "com.example.MagicHotspot.Tray",
        "Place", None, None, gtk::gio::DBusCallFlags::NONE, 1000,
        None::<&gtk::gio::Cancellable>,
        move |result| {
            match result {
                Ok(reply) if reply.get::<(bool,)>() == Some((true,)) => {},
                _ if attempts > 1 => {
                    // Extension activation, the panel icon and the surface map independently.
                    glib::timeout_add_local_once(std::time::Duration::from_millis(150), move || {
                        if let Some(window) = weak.upgrade().filter(|w| w.is_visible()) {
                            place_with_companion(&window, attempts - 1);
                        }
                    });
                },
                Ok(_) => eprintln!("Magic Hotspot tray placement: panel icon or window is not ready"),
                Err(error) => eprintln!("Magic Hotspot tray placement unavailable; the installed GNOME companion may require signing out and back in: {error}"),
            }
        },
    );
}

pub fn pointer_position(window: &ApplicationWindow) -> Option<(i32, i32)> {
    let pointer = window.display().default_seat()?.pointer()?;
    let (_, x, y) = pointer.position();
    Some((x, y))
}

/// Use the panel-menu anchor, in GTK's logical monitor coordinates.
pub fn present_from_tray(window: &ApplicationWindow, anchor: Option<(i32, i32)>) {
    if is_gnome_wayland() {
        // Only the compositor knows where its native panel icon is. Xwayland's
        // pointer may be stale; preserve position if the companion is unavailable.
        window.set_position(gtk::WindowPosition::None);
        present(window);
        let weak = window.downgrade();
        glib::timeout_add_local_once(std::time::Duration::from_millis(100), move || {
            let Some(window) = weak.upgrade().filter(|w| w.is_visible()) else {
                return;
            };
            place_with_companion(&window, 20);
        });
        return;
    }
    let display = window.display();
    let Some((x, y)) = anchor.or_else(|| pointer_position(window)) else {
        present(window);
        return;
    };
    let Some(monitor) = display.monitor_at_point(x, y) else {
        return;
    };
    let area = monitor.workarea();
    window.set_position(gtk::WindowPosition::None);
    present(window);
    let weak = window.downgrade();
    glib::timeout_add_local_once(std::time::Duration::from_millis(100), move || {
        let Some(window) = weak.upgrade().filter(|w| w.is_visible()) else {
            return;
        };
        let Some(native) = window.window() else {
            return;
        };
        let (width, height) = (native.width(), native.height());
        let left = area.x() + SCREEN_MARGIN;
        let right = (area.x() + area.width() - width - SCREEN_MARGIN).max(left);
        let top = area.y() + SCREEN_MARGIN;
        let bottom = (area.y() + area.height() - height - SCREEN_MARGIN).max(top);
        // The panel host owns its menu. Open beneath a top panel or above a
        // bottom panel, preserving spacing and the monitor's reserved areas.
        window.move_(
            (x - width / 2).clamp(left, right),
            if y < area.y() + area.height() / 2 {
                top
            } else {
                bottom
            },
        );
    });
}

pub fn hide(window: &ApplicationWindow) {
    window.set_skip_taskbar_hint(true);
    window.set_skip_pager_hint(true);
    window.hide();
}

pub fn minimize_to_tray(window: &ApplicationWindow, header: &HeaderBar, tray: &Rc<Tray>) {
    let state_tray = tray.clone();
    window.connect_window_state_event(move |window, event| {
        if event
            .changed_mask()
            .contains(gtk::gdk::WindowState::ICONIFIED)
            && event
                .new_window_state()
                .contains(gtk::gdk::WindowState::ICONIFIED)
            && state_tray.is_visible()
        {
            hide(window);
        }
        glib::Propagation::Proceed
    });

    // Wayland does not report an iconified state. Keep GTK's native titlebar
    // buttons, and also handle its minimize button's click directly. Revisit
    // internal children after layout changes because GTK can rebuild controls.
    let watched = Rc::new(RefCell::new(Vec::<glib::WeakRef<Button>>::new()));
    let weak_window = window.downgrade();
    let native_tray = tray.clone();
    header.connect_size_allocate(move |header, _| {
        visit(header.upcast_ref(), &mut |widget| {
            let Some(button) = widget.downcast_ref::<Button>() else {
                return;
            };
            if !button.style_context().has_class("minimize") {
                return;
            }
            let mut watched = watched.borrow_mut();
            watched.retain(|item| item.upgrade().is_some());
            if watched
                .iter()
                .any(|item| item.upgrade().as_ref() == Some(button))
            {
                return;
            }
            watched.push(button.downgrade());
            let window = weak_window.clone();
            let tray = native_tray.clone();
            button.connect_clicked(move |_| {
                if tray.is_visible() {
                    if let Some(window) = window.upgrade() {
                        hide(&window);
                    }
                }
            });
        });
    });
}

fn visit(widget: &Widget, callback: &mut impl FnMut(&Widget)) {
    callback(widget);
    if let Some(container) = widget.downcast_ref::<gtk::Container>() {
        container.forall(|child| visit(child, callback));
    }
}
