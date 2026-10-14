// Module: magic_hotspot.application
// Purpose: Present a focused hotspot switcher and nearby Wi-Fi shortcuts.
// Created: 2026-10-09
// Architecture: GTK owns the interface; network.rs delegates network actions to NetworkManager.

mod autostart;
mod bluetooth;
mod connection_dialog;
mod connections;
mod i18n;
mod language;
mod network;
mod pan;
mod tray;
mod usb;
mod window;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gdk_pixbuf::PixbufLoader;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Button, Image, Label, ListBox, Orientation,
    ScrolledWindow, Switch,
};

const APPLICATION_ID: &str = "com.example.MagicHotspot";
const WINDOW_WIDTH: i32 = 480;
const WINDOW_HEIGHT: i32 = 420;
const USB_STATUS_INTERVAL_SECONDS: u32 = 2;
const DEVICE_ROW_INSET: i32 = 5;
const DEVICE_LIST_TOP_MARGIN: i32 = 16;

const WIFI_ICON: &str = include_str!("../assets/fluent/wifi_1_24_regular.svg");
const APP_ICON: &str = include_str!("../assets/magic-hotspot.svg");

fn main() {
    i18n::init();
    // GNOME's companion positions native Wayland windows. Other desktops retain X11 placement.
    // No GTK initialization or worker thread exists yet, so environment mutation is safe.
    if !window::is_gnome_wayland()
        && std::env::var_os("GDK_BACKEND").is_none()
        && std::env::var_os("DISPLAY").is_some()
    {
        unsafe {
            std::env::set_var("GDK_BACKEND", "x11,wayland");
        }
    }
    let application = Application::new(
        Some(APPLICATION_ID),
        gtk::gio::ApplicationFlags::HANDLES_COMMAND_LINE,
    );
    application.add_main_option(
        "background",
        0.into(),
        glib::OptionFlags::NONE,
        glib::OptionArg::None,
        "Start in the tray (default)",
        None,
    );
    application.add_main_option(
        "show",
        0.into(),
        glib::OptionFlags::NONE,
        glib::OptionArg::None,
        "Show the window beside the tray icon",
        None,
    );
    let background = Rc::new(Cell::new(false));
    let command_background = background.clone();
    application.connect_command_line(move |application, command| {
        // Ordinary launches stay quiet, including repeated launcher clicks.
        // Explicit Open/dock activation remains a foreground action.
        command_background.set(
            !command.options_dict().contains("show")
                || command.options_dict().contains("background"),
        );
        application.activate();
        0
    });
    application.connect_activate(move |application| {
        let start_in_tray = background.replace(false);
        if let Some(window) = application
            .windows()
            .into_iter()
            .find_map(|window| window.downcast::<ApplicationWindow>().ok())
        {
            if !start_in_tray {
                window::present_application(&window);
            }
        } else {
            build_window(application, start_in_tray);
        }
        // Tray startup maps no window, so GTK cannot automatically finish
        // the desktop launch notification. Do not leave its busy cursor active.
        gtk::gdk::notify_startup_complete();
    });
    let quit = gtk::gio::SimpleAction::new("quit", None);
    let owner = application.downgrade();
    quit.connect_activate(move |_, _| {
        if let Some(app) = owner.upgrade() {
            app.quit();
        }
    });
    application.add_action(&quit);
    application.connect_startup(|app| {
        window::enable_gnome_companion();
        app.set_accels_for_action("app.quit", &["<Primary>q"]);
    });
    application.run();
}

fn build_window(application: &Application, start_in_tray: bool) {
    install_styles();

    let window = ApplicationWindow::new(application);
    window.set_title("Magic Hotspot");
    window.set_default_size(WINDOW_WIDTH, WINDOW_HEIGHT);
    window.set_resizable(true);
    let app_icon = fluent_icon(APP_ICON).pixbuf().expect("embedded app icon");
    window.set_icon(Some(&app_icon));
    let header = gtk::HeaderBar::new();
    header.set_title(Some("Magic Hotspot"));
    header.set_show_close_button(true);
    header.set_decoration_layout(Some(":minimize,maximize,close"));
    window.set_titlebar(Some(&header));

    let page = GtkBox::new(Orientation::Vertical, 14);
    page.set_margin_top(22);
    page.set_margin_bottom(20);
    page.set_margin_start(22);
    page.set_margin_end(22);
    page.style_context().add_class("page");

    let phone_card = card();
    let startup_row = GtkBox::new(Orientation::Horizontal, 10);
    let startup_label = i18n::label("Start when I sign in");
    startup_label.set_xalign(0.0);
    startup_label.set_valign(gtk::Align::Center);
    startup_label.set_hexpand(true);
    startup_label.style_context().add_class("body");
    let startup_switch = Switch::new();
    startup_switch.set_valign(gtk::Align::Center);
    let startup = autostart::Autostart::current();
    startup_switch.set_active(
        startup
            .as_ref()
            .ok()
            .and_then(|item| item.is_enabled().ok())
            .unwrap_or(false),
    );
    startup_row.pack_start(&startup_label, true, true, 0);
    startup_row.pack_end(&startup_switch, false, false, 0);
    phone_card.pack_start(&startup_row, false, false, 0);

    let wifi_card = card();
    let wifi_header = GtkBox::new(Orientation::Horizontal, 8);
    wifi_header.set_margin_start(DEVICE_ROW_INSET);
    wifi_header.set_margin_end(DEVICE_ROW_INSET);
    wifi_header.pack_start(&fluent_icon(WIFI_ICON), false, false, 0);
    let wifi_title = i18n::label("Nearby devices");
    wifi_title.set_xalign(0.0);
    wifi_title.style_context().add_class("section-title");
    wifi_header.pack_start(&wifi_title, true, true, 0);
    wifi_card.pack_start(&wifi_header, false, false, 0);

    let wifi_list = ListBox::new();
    wifi_list.set_selection_mode(gtk::SelectionMode::None);
    let wifi_scroll = ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    wifi_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    wifi_scroll.set_min_content_height(100);
    wifi_scroll.set_margin_top(DEVICE_LIST_TOP_MARGIN);
    wifi_scroll.add(&wifi_list);
    wifi_card.pack_start(&wifi_scroll, true, true, 0);
    page.pack_start(&wifi_card, true, true, 0);
    page.pack_start(&phone_card, false, false, 0);

    let status = Label::new(None);
    // Connection state belongs to its row; reserve the footer for actionable errors.
    status.set_no_show_all(true);
    status.connect_label_notify(|label| {
        let text = label.text();
        label.set_visible(
            text == i18n::tr("Connection failed")
                || text.starts_with(&i18n::tr("Could not"))
                || text.starts_with("Could ")
                || text.contains("failed"),
        );
    });
    status.set_xalign(0.0);
    status.set_line_wrap(true);
    status.style_context().add_class("status");
    page.pack_start(&status, false, false, 0);
    let startup_status = status.clone();
    let updating_startup = Rc::new(Cell::new(false));
    startup_switch.connect_state_set(move |switch, enabled| {
        if updating_startup.replace(true) {
            return glib::Propagation::Proceed;
        }
        let result = startup
            .as_ref()
            .map_err(|error| error.clone())
            .and_then(|item| item.set_enabled(enabled));
        let propagation = match result {
            Ok(()) => glib::Propagation::Proceed,
            Err(error) => {
                switch.set_active(!enabled);
                startup_status.set_text(&i18n::trf(
                    "Could not update startup: {error}",
                    &[("error", &error)],
                ));
                glib::Propagation::Stop
            }
        };
        updating_startup.set(false);
        propagation
    });

    let device_connecting = Rc::new(Cell::new(false));
    let cable_connecting = device_connecting.clone();
    let selected = Rc::new(RefCell::new(None::<ConnectionTarget>));
    let controls = ConnectionControls::default();
    let scanning = Rc::new(Cell::new(false));
    let language_controls = controls.clone();
    let language_selected = selected.clone();
    let language_busy = device_connecting.clone();
    let language_status = status.clone();
    let language_button = language::picker(move |result| {
        if let Err(error) = result {
            language_status.set_text(&i18n::trf(
                "Could not save language: {error}",
                &[("error", &error)],
            ));
        }
        let active = language_controls.active.borrow().clone();
        language_controls.update(
            active,
            language_selected
                .borrow()
                .as_ref()
                .map(ConnectionTarget::identity)
                .as_ref(),
            language_busy.get(),
        );
    });
    header.pack_start(&language_button);

    // Re-enumeration changes the USB address even when the cable uses the same
    // port. Poll metadata/status off-thread, including when hidden in the tray.
    let usb_window = window.downgrade();
    let usb_status = status.clone();
    let usb_rows = wifi_list.clone();
    let usb_scanning = scanning.clone();
    let usb_selected = selected.clone();
    let usb_controls = controls.clone();
    let usb_polling = Rc::new(Cell::new(false));
    let usb_attachment = Rc::new(std::cell::RefCell::new(None::<usb::Snapshot>));
    glib::timeout_add_seconds_local(USB_STATUS_INTERVAL_SECONDS, move || {
        if usb_window.upgrade().is_none() {
            return glib::ControlFlow::Break;
        }
        if cable_connecting.get() || usb_polling.get() {
            return glib::ControlFlow::Continue;
        }
        usb_polling.set(true);
        let polling = usb_polling.clone();
        let connecting = cable_connecting.clone();
        let status = usb_status.clone();
        let weak_window = usb_window.clone();
        let saved = usb_attachment.clone();
        let rows = usb_rows.clone();
        let scanning = usb_scanning.clone();
        let selected = usb_selected.clone();
        let controls = usb_controls.clone();
        glib::MainContext::default().spawn_local(async move {
            let result =
                gtk::gio::spawn_blocking(|| (usb::snapshot(), connections::snapshot())).await;
            if let Ok((snapshot, active)) = result {
                controls.update(
                    active.unwrap_or_default(),
                    selected
                        .borrow()
                        .as_ref()
                        .map(ConnectionTarget::identity)
                        .as_ref(),
                    connecting.get(),
                );
                if weak_window.upgrade().is_some() && !connecting.get() {
                    let changed = saved
                        .borrow()
                        .as_ref()
                        .map(|old| !snapshot.same_attachment(old))
                        .unwrap_or(false);
                    if changed {
                        if let Some(window) = weak_window.upgrade() {
                            refresh_networks(
                                &rows,
                                &status,
                                &window,
                                &scanning,
                                &selected,
                                &connecting,
                                &controls,
                            );
                        }
                    }
                }
                *saved.borrow_mut() = Some(snapshot);
            }
            polling.set(false);
        });
        glib::ControlFlow::Continue
    });

    refresh_networks(
        &wifi_list,
        &status,
        &window,
        &scanning,
        &selected,
        &device_connecting,
        &controls,
    );
    let weak_window = window.downgrade();
    let automatic_list = wifi_list.clone();
    let automatic_status = status.clone();
    let automatic_scanning = scanning.clone();
    let automatic_selected = selected.clone();
    let automatic_connecting = device_connecting.clone();
    let automatic_controls = controls.clone();
    glib::timeout_add_seconds_local(30, move || {
        let Some(window) = weak_window.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if window.is_visible() {
            refresh_networks(
                &automatic_list,
                &automatic_status,
                &window,
                &automatic_scanning,
                &automatic_selected,
                &automatic_connecting,
                &automatic_controls,
            );
        }
        glib::ControlFlow::Continue
    });
    window.add(&page);
    let tray = Rc::new(tray::Tray::new(&window));
    window.connect_delete_event(move |window, _| {
        // Closing hides the existing window even while the desktop tray starts.
        window::hide(window);
        glib::Propagation::Stop
    });
    window::minimize_to_tray(&window, &header, &tray);
    application.connect_shutdown(move |_| tray.stop());
    if start_in_tray {
        // Never map the window during login, even while the tray host starts.
        window::hide(&window);
    } else {
        window::present_from_tray(&window, None);
    }
}

#[derive(Clone)]
enum ConnectionTarget {
    Usb,
    Wifi(String),
    Bluetooth(bluetooth::Phone),
}

impl ConnectionTarget {
    fn identity(&self) -> connections::Target {
        match self {
            Self::Usb => connections::Target::Usb,
            Self::Wifi(name) => connections::Target::Wifi(name.clone()),
            Self::Bluetooth(phone) => {
                connections::Target::Bluetooth(phone.address.to_ascii_uppercase())
            }
        }
    }
}

struct ConnectionRow {
    connect: glib::WeakRef<Button>,
    disconnect: glib::WeakRef<Button>,
    targets: Vec<connections::Target>,
}

#[derive(Clone, Default)]
struct ConnectionControls {
    rows: Rc<RefCell<Vec<ConnectionRow>>>,
    active: Rc<RefCell<Vec<connections::ActiveConnection>>>,
    preference: std::sync::Arc<std::sync::Mutex<connections::Preference>>,
}

impl ConnectionControls {
    fn update(
        &self,
        active: Vec<connections::ActiveConnection>,
        preferred: Option<&connections::Target>,
        busy: bool,
    ) {
        for row in self.rows.borrow().iter() {
            let (Some(connect), Some(disconnect)) =
                (row.connect.upgrade(), row.disconnect.upgrade())
            else {
                continue;
            };
            let matching: Vec<_> = active
                .iter()
                .filter(|link| row.targets.contains(&link.target))
                .collect();
            let current = matching
                .iter()
                .find(|link| Some(&link.target) == preferred)
                .copied()
                .or_else(|| matching.first().copied());
            connect.set_label(
                &current
                    .map(|link| i18n::trf("Connected ({mode})", &[("mode", link.target.mode())]))
                    .unwrap_or_else(|| i18n::tr("Connect")),
            );
            connect.set_sensitive(!busy);
            disconnect.set_sensitive(current.is_some() && !busy);
        }
        *self.active.borrow_mut() = active;
    }

    fn add(
        &self,
        row: &GtkBox,
        connect: &Button,
        targets: Vec<connections::Target>,
        window: &ApplicationWindow,
        status: &Label,
        busy: &Rc<Cell<bool>>,
        selected: &Rc<RefCell<Option<ConnectionTarget>>>,
    ) {
        let disconnect = i18n::button("Disconnect");
        disconnect.style_context().add_class("action-button");
        disconnect.set_sensitive(false);
        let window = window.clone();
        let status = status.clone();
        let busy = busy.clone();
        let selected = selected.clone();
        let controls = self.clone();
        let disconnect_targets = targets.clone();
        disconnect.connect_clicked(move |_| {
            if busy.replace(true) {
                return;
            }
            // A manual disconnect clears this device as the selected transport.
            if selected
                .borrow()
                .as_ref()
                .map(|target| disconnect_targets.contains(&target.identity()))
                .unwrap_or(false)
            {
                *selected.borrow_mut() = None;
            }
            let active = controls.active.borrow().clone();
            controls.update(
                active,
                selected
                    .borrow()
                    .as_ref()
                    .map(ConnectionTarget::identity)
                    .as_ref(),
                true,
            );
            let dialog = connection_dialog::ConnectionDialog::disconnecting(&window);
            let targets = disconnect_targets.clone();
            let status = status.clone();
            let busy = busy.clone();
            let selected = selected.clone();
            let controls = controls.clone();
            glib::MainContext::default().spawn_local(async move {
                let result = gtk::gio::spawn_blocking(move || {
                    let result = connections::disconnect(&targets).map(|_| String::new());
                    (result, connections::snapshot())
                })
                .await;
                busy.set(false);
                let (result, active) = result.unwrap_or_else(|_| {
                    (
                        Err("Could not disconnect. Try again.".into()),
                        Ok(Vec::new()),
                    )
                });
                controls.update(
                    active.unwrap_or_default(),
                    selected
                        .borrow()
                        .as_ref()
                        .map(ConnectionTarget::identity)
                        .as_ref(),
                    false,
                );
                dialog.finish(result, &status);
            });
        });
        row.pack_end(&disconnect, false, false, 0);
        row.pack_end(connect, false, false, 0);
        self.rows.borrow_mut().push(ConnectionRow {
            connect: connect.downgrade(),
            disconnect: disconnect.downgrade(),
            targets,
        });
    }
}

fn connect_target(
    window: &ApplicationWindow,
    status: &Label,
    busy: &Rc<Cell<bool>>,
    controls: &ConnectionControls,
    target: ConnectionTarget,
) {
    if busy.replace(true) {
        return;
    }
    let name = match &target {
        ConnectionTarget::Usb => i18n::tr("iPhone via USB"),
        ConnectionTarget::Wifi(name) => i18n::trf("{name} via Wi-Fi", &[("name", name)]),
        ConnectionTarget::Bluetooth(phone) => {
            i18n::trf("{name} via Bluetooth", &[("name", &phone.name)])
        }
    };
    let identity = target.identity();
    let dialog = connection_dialog::ConnectionDialog::new(window, &name);
    let status = status.clone();
    let busy = busy.clone();
    let controls = controls.clone();
    let active = controls.active.borrow().clone();
    controls.update(active, Some(&identity), true);
    let preference = controls.preference.clone();
    glib::MainContext::default().spawn_local(async move {
        let result = gtk::gio::spawn_blocking(move || {
            let result = preference
                .lock()
                .map_err(|_| "Could not select the preferred connection.".to_string())
                .and_then(|mut preference| {
                    preference.connect(&identity, || match target {
                        ConnectionTarget::Usb => network::activate_hotspot(""),
                        ConnectionTarget::Wifi(name) => network::activate_profile(&name),
                        ConnectionTarget::Bluetooth(phone) => bluetooth::connect(phone),
                    })
                })
                .map(|_| String::new());
            (result, connections::snapshot(), identity)
        })
        .await;
        busy.set(false);
        let (result, active, identity) = result.unwrap_or_else(|_| {
            (
                Err("Could not complete the connection. Try again.".into()),
                Ok(Vec::new()),
                connections::Target::Usb,
            )
        });
        controls.update(active.unwrap_or_default(), Some(&identity), false);
        dialog.finish(result, &status);
    });
}

fn refresh_networks(
    list: &ListBox,
    status: &Label,
    window: &ApplicationWindow,
    scanning: &Rc<Cell<bool>>,
    selected: &Rc<RefCell<Option<ConnectionTarget>>>,
    connecting: &Rc<Cell<bool>>,
    controls: &ConnectionControls,
) {
    if connecting.get() || scanning.replace(true) {
        return;
    }
    list.set_sensitive(false);
    status.set_text(&i18n::tr("Looking for devices…"));
    let list = list.clone();
    let status = status.clone();
    let window = window.downgrade();
    let scanning = scanning.clone();
    let selected = selected.clone();
    let connecting = connecting.clone();
    let controls = controls.clone();
    glib::MainContext::default().spawn_local(async move {
        let result = gtk::gio::spawn_blocking(|| {
            std::thread::scope(|scope| {
                let wifi = scope.spawn(network::scan_wifi);
                let phones = bluetooth::scan();
                (
                    wifi.join()
                        .unwrap_or_else(|_| Err("Could not scan Wi-Fi".into())),
                    phones,
                    usb::snapshot().is_connected(),
                    connections::snapshot(),
                )
            })
        })
        .await;
        if let Some(window) = window.upgrade() {
            match result {
                Ok((wifi, phones, usb_available, active)) => {
                    let ssids: Vec<String> = wifi
                        .as_ref()
                        .map(|items| items.iter().map(|n| n.ssid.clone()).collect())
                        .unwrap_or_default();
                    // Keep active connection controls/results intact if the user
                    // started connecting while this background scan was running.
                    if !connecting.get() {
                        controls.rows.borrow_mut().clear();
                        render_networks(
                            &list,
                            &status,
                            &window,
                            wifi,
                            &connecting,
                            &selected,
                            &controls,
                        );
                        render_bluetooth(
                            &list,
                            &status,
                            phones,
                            &window,
                            &connecting,
                            &ssids,
                            usb_available,
                            &selected,
                            &controls,
                        );
                        controls.update(
                            active.unwrap_or_default(),
                            selected
                                .borrow()
                                .as_ref()
                                .map(ConnectionTarget::identity)
                                .as_ref(),
                            false,
                        );
                    }
                }
                Err(_) => status.set_text(&i18n::tr("Could not refresh devices")),
            }
            list.set_sensitive(true);
        }
        scanning.set(false);
    });
}

fn render_networks(
    list: &ListBox,
    status: &Label,
    window: &ApplicationWindow,
    result: Result<Vec<network::WifiNetwork>, String>,
    connecting: &Rc<Cell<bool>>,
    selected: &Rc<RefCell<Option<ConnectionTarget>>>,
    controls: &ConnectionControls,
) {
    for child in list.children() {
        list.remove(&child);
    }
    match result {
        Ok(networks) => {
            for item in networks {
                let row = GtkBox::new(Orientation::Horizontal, 8);
                let name = Label::new(Some(&item.ssid));
                name.set_xalign(0.0);
                name.set_ellipsize(gtk::pango::EllipsizeMode::End);
                name.set_max_width_chars(24);
                name.set_hexpand(true);
                let signal = Label::new(Some(&format!("{}%", item.signal)));
                signal.style_context().add_class("muted");
                let network_security = if matches!(item.security.trim(), "" | "--") {
                    "Open · not encrypted".to_string()
                } else {
                    item.security
                };
                let security = if network_security == "Open · not encrypted" {
                    i18n::label("Open · not encrypted")
                } else {
                    Label::new(Some(&network_security))
                };
                security.style_context().add_class("muted");
                let connect = i18n::button("Connect");
                connect.style_context().add_class("action-button");
                connect.style_context().add_class("primary");
                controls.add(
                    &row,
                    &connect,
                    vec![connections::Target::Wifi(item.ssid.clone())],
                    window,
                    status,
                    connecting,
                    selected,
                );
                let controls = controls.clone();
                let ssid = item.ssid;
                let row_status = status.clone();
                let row_window = window.clone();
                let busy = connecting.clone();
                let selected = selected.clone();
                let is_open = network_security.starts_with("Open");
                connect.connect_clicked(move |_| {
                    if is_open && !connection_dialog::confirm_open_wifi(&row_window) {
                        return;
                    }
                    if busy.get() {
                        return;
                    }
                    let target = ConnectionTarget::Wifi(ssid.clone());
                    *selected.borrow_mut() = Some(target.clone());
                    connect_target(&row_window, &row_status, &busy, &controls, target);
                });
                row.pack_start(&name, true, true, 0);
                row.pack_start(&security, false, false, 0);
                row.pack_start(&signal, false, false, 5);

                list.add(&row);
            }
            status.set_text(&i18n::tr("Select a saved network to connect."));
        }
        Err(_) => {}
    }
    list.show_all();
}

fn render_bluetooth(
    list: &ListBox,
    status: &Label,
    phones: Result<Vec<bluetooth::Phone>, String>,
    window: &ApplicationWindow,
    connecting: &Rc<Cell<bool>>,
    ssids: &[String],
    usb_available: bool,
    selected: &Rc<RefCell<Option<ConnectionTarget>>>,
    controls: &ConnectionControls,
) {
    if usb_available
        && phones
            .as_ref()
            .map(|items| items.is_empty())
            .unwrap_or(true)
    {
        let row = GtkBox::new(Orientation::Horizontal, 8);
        let name = Label::new(Some("iPhone (USB)"));
        name.set_xalign(0.0);
        let button = i18n::button("Connect");
        button.style_context().add_class("action-button");
        button.style_context().add_class("primary");
        controls.add(
            &row,
            &button,
            vec![connections::Target::Usb],
            window,
            status,
            connecting,
            selected,
        );
        let controls = controls.clone();
        let window = window.clone();
        let status = status.clone();
        let busy = connecting.clone();
        let selected = selected.clone();
        button.connect_clicked(move |_| {
            if busy.get() {
                return;
            }
            *selected.borrow_mut() = Some(ConnectionTarget::Usb);
            connect_target(&window, &status, &busy, &controls, ConnectionTarget::Usb);
        });
        row.pack_start(&name, true, true, 0);

        list.add(&row);
    }
    match phones {
        Ok(phones) => {
            for phone in phones {
                let row = GtkBox::new(Orientation::Horizontal, 8);
                let name = Label::new(Some(&phone.name));
                name.set_xalign(0.0);
                name.set_hexpand(true);
                name.set_ellipsize(gtk::pango::EllipsizeMode::End);
                let button = gtk::MenuButton::new();
                i18n::bind_button(button.upcast_ref::<Button>(), "Connect");
                button.style_context().add_class("action-button");
                button.style_context().add_class("primary");
                let mut targets = vec![connections::Target::Bluetooth(
                    phone.address.to_ascii_uppercase(),
                )];
                if usb_available {
                    targets.push(connections::Target::Usb);
                }
                if ssids.contains(&phone.name) {
                    targets.push(connections::Target::Wifi(phone.name.clone()));
                }
                controls.add(
                    &row,
                    button.upcast_ref::<Button>(),
                    targets,
                    window,
                    status,
                    connecting,
                    selected,
                );
                let menu = gtk::Menu::new();
                for (label, available) in [
                    ("USB", usb_available),
                    ("Wi-Fi", ssids.contains(&phone.name)),
                    ("Bluetooth", true),
                ] {
                    let item = gtk::MenuItem::with_label(label);
                    item.set_sensitive(available);
                    let status = status.clone();
                    let busy = connecting.clone();
                    let phone = phone.clone();
                    let window = window.clone();
                    let selected = selected.clone();
                    let controls = controls.clone();
                    item.connect_activate(move |_| {
                        if busy.get() {
                            return;
                        }
                        let target = match label {
                            "USB" => ConnectionTarget::Usb,
                            "Wi-Fi" => ConnectionTarget::Wifi(phone.name.clone()),
                            _ => ConnectionTarget::Bluetooth(phone.clone()),
                        };
                        *selected.borrow_mut() = Some(target.clone());
                        connect_target(&window, &status, &busy, &controls, target);
                    });
                    menu.append(&item);
                }
                menu.show_all();
                button.set_popup(Some(&menu));
                row.pack_start(&name, true, true, 0);

                list.add(&row);
            }
        }
        Err(_) => {}
    }
    status.set_text(&i18n::tr(""));
    list.show_all();
}

fn card() -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 12);
    container.style_context().add_class("card");
    container
}

/// Load an official Microsoft Fluent System Icon embedded in this executable.
fn fluent_icon(svg: &str) -> Image {
    let loader = PixbufLoader::with_type("svg").expect("SVG loader is available through GTK");
    loader.write(svg.as_bytes()).expect("embedded SVG is valid");
    loader.close().expect("embedded SVG can be loaded");
    Image::from_pixbuf(loader.pixbuf().as_ref())
}

fn install_styles() {
    let provider = gtk::CssProvider::new();
    provider
        .load_from_data(
            b"window { background-color: #f5f5f5; color: #242424; font-family: 'Segoe UI', 'Ubuntu', sans-serif; }\
              .page { background-color: #f5f5f5; }\
              button.language-button { padding: 2px 4px; min-height: 24px; min-width: 30px; background: transparent; border-color: transparent; }\
              popover.language-popover { padding: 0; border-radius: 12px; }\
              button.language-row { padding: 8px 12px; border-radius: 8px; min-width: 200px; }\
              button.language-row:hover { background-color: #eef5fc; }\
              .section-title { font-size: 15px; font-weight: 600; color: #242424; }\
              .body { font-size: 13px; color: #242424; }\
              .muted { font-size: 12px; color: #616161; }\
              .card { background-color: #ffffff; border: 1px solid #e1dfdd; border-radius: 8px; padding: 16px; }\
              .page button { min-height: 34px; padding: 0 12px; border-radius: 4px; }\
              .page button.action-button { color: #242424; background: #ffffff; border: 1px solid #d1d1d1; box-shadow: none; text-shadow: none; }\
              .page button.action-button:hover { background: #f5f5f5; border-color: #bdbdbd; }\
              .page button.action-button:active { background: #eeeeee; }\
              .page button.action-button:focus { border-color: #0f6cbd; box-shadow: 0 0 0 1px #0f6cbd; }\
              .page button.action-button.primary { color: #ffffff; background: #0f6cbd; border-color: #0f6cbd; }\
              .page button.action-button.primary:hover { background: #115ea3; border-color: #115ea3; }\
              .page button.action-button.primary:active { background: #0f548c; border-color: #0f548c; }\
              .page button.action-button:disabled { color: #999999; background: #f5f5f5; border-color: #e1dfdd; box-shadow: none; }\
              .status { color: #616161; font-size: 12px; padding: 2px 2px; }\
              list { background: #ffffff; }\
              .page list row { padding: 5px; border-bottom: 1px solid #f0f0f0; }",
        )
        .expect("embedded GTK CSS is valid");
    if let Some(screen) = gtk::gdk::Screen::default() {
        gtk::StyleContext::add_provider_for_screen(
            &screen,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

#[cfg(test)]
#[path = "../tests/ui/controls.rs"]
mod ui_tests;
