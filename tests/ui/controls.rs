// Module: magic_hotspot.tests.ui.controls
// Purpose: Check native GTK labels and sensitivity against live transport snapshots.
// Created: 2026-10-11
// Architecture: An isolated Xvfb test; no system networking or desktop interaction.
use super::*;

#[test]
#[ignore = "requires an isolated graphical display"]
fn row_controls_follow_connection_state_and_selected_mode() {
    gtk::init().unwrap();
    let controls = ConnectionControls::default();
    let connect = Button::with_label("Connect");
    let disconnect = Button::with_label("Disconnect");
    let bluetooth = connections::Target::Bluetooth("AA:BB:CC:DD:EE:FF".into());
    let wifi = connections::Target::Wifi("Phone".into());
    controls.rows.borrow_mut().push(ConnectionRow {
        connect: connect.downgrade(),
        disconnect: disconnect.downgrade(),
        targets: vec![bluetooth.clone(), wifi.clone()],
    });
    controls.update(Vec::new(), None, false);
    assert_eq!(connect.label().as_deref(), Some("Connect"));
    assert!(!disconnect.is_sensitive());
    let pan = connections::ActiveConnection {
        uuid: "pan".into(),
        device: "bt0".into(),
        target: bluetooth.clone(),
    };
    let wlan = connections::ActiveConnection {
        uuid: "wifi".into(),
        device: "wlan0".into(),
        target: wifi.clone(),
    };
    controls.update(vec![pan.clone()], None, false);
    assert_eq!(connect.label().as_deref(), Some("Connected (Bluetooth)"));
    assert!(disconnect.is_sensitive());
    controls.update(vec![pan.clone(), wlan.clone()], Some(&wifi), false);
    assert_eq!(connect.label().as_deref(), Some("Connected (Wi-Fi)"));
    controls.update(vec![pan, wlan], Some(&bluetooth), true);
    assert_eq!(connect.label().as_deref(), Some("Connected (Bluetooth)"));
    assert!(!connect.is_sensitive());
    assert!(!disconnect.is_sensitive());
    controls.update(Vec::new(), None, false);
    assert_eq!(connect.label().as_deref(), Some("Connect"));
    assert!(connect.is_sensitive());
    assert!(!disconnect.is_sensitive());
}

#[test]
#[ignore = "requires an isolated graphical display and settings directory"]
fn language_picker_updates_live_controls_and_remembers_selection() {
    gtk::init().unwrap();
    let controls = ConnectionControls::default();
    let connect = i18n::button("Connect");
    let disconnect = i18n::button("Disconnect");
    let target = connections::Target::Usb;
    controls.rows.borrow_mut().push(ConnectionRow {
        connect: connect.downgrade(),
        disconnect: disconnect.downgrade(),
        targets: vec![target.clone()],
    });
    let active = connections::ActiveConnection {
        uuid: "test-usb".into(),
        device: "usb-test".into(),
        target: target.clone(),
    };
    controls.update(vec![active.clone()], Some(&target), true);
    let update = controls.clone();
    let picker = language::picker(move |result| {
        result.unwrap();
        update.update(vec![active.clone()], Some(&target), true);
    });
    let title = i18n::label("Nearby devices");
    let settings = i18n::label("Start when I sign in");
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    let header = gtk::HeaderBar::new();
    header.pack_end(&picker);
    window.set_titlebar(Some(&header));
    let content = GtkBox::new(Orientation::Vertical, 8);
    content.pack_start(&title, false, false, 0);
    content.pack_start(&settings, false, false, 0);
    content.pack_start(&connect, false, false, 0);
    content.pack_start(&disconnect, false, false, 0);
    window.add(&content);
    window.show_all();
    let popover = picker.popover().unwrap();
    let list = popover.child().unwrap().downcast::<GtkBox>().unwrap();
    for (code, expected) in [
        ("pt", "Dispositivos próximos"),
        ("es", "Dispositivos cercanos"),
        ("el", "Κοντινές συσκευές"),
        ("en", "Nearby devices"),
    ] {
        let row = list
            .children()
            .into_iter()
            .find(|widget| widget.widget_name() == format!("language-{code}"))
            .unwrap()
            .downcast::<Button>()
            .unwrap();
        row.emit_clicked();
        assert_eq!(title.text(), expected);
        assert_eq!(i18n::lang().code(), code);
        assert_eq!(
            std::fs::read_to_string(glib::user_config_dir().join("magic-hotspot/language"))
                .unwrap(),
            code
        );
        assert!(
            !connect.is_sensitive() && !disconnect.is_sensitive(),
            "language switch changed busy controls"
        );
        assert_eq!(
            connect.label().unwrap(),
            i18n::trf("Connected ({mode})", &[("mode", "USB")])
        );
        i18n::init();
        assert_eq!(
            i18n::lang().code(),
            code,
            "saved choice was lost on reinitialization"
        );
    }
    unsafe {
        window.destroy();
    }
}
