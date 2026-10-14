// Module: magic_hotspot.tests.gnome_probe
// Purpose: Inspect and photograph actual placement in an isolated GNOME session.
// Created: 2026-10-11
// Architecture: Test-only wrapper; never shipped as the installed extension.
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Shell from 'gi://Shell';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import Placement from './placement.js';

export default class Probe extends Placement {
    enable() {
        super.enable();
        this._probe = Gio.DBusExportedObject.wrapJSObject(`<node>
          <interface name="com.example.MagicHotspot.Test">
            <method name="Visible"><arg type="b" direction="out"/></method>
            <method name="Inspect"><arg type="s" direction="out"/></method>
            <method name="Snapshot"><arg type="s" direction="in"/><arg type="b" direction="out"/></method>
          </interface></node>`, this);
        this._probe.export(Gio.DBus.session, '/com/example/MagicHotspot/Test');
        Main.overview.hide();
    }

    disable() {
        this._probe?.unexport();
        super.disable();
    }

    Visible() {
        return global.get_window_actors().some(actor =>
            actor.meta_window.get_gtk_application_id() === 'com.example.MagicHotspot' &&
            !actor.meta_window.is_hidden());
    }

    Inspect() {
        const window = global.get_window_actors().map(a => a.meta_window).find(
            w => w.get_gtk_application_id() === 'com.example.MagicHotspot');
        const icon = Object.values(Main.panel.statusArea).find(
            a => a._indicator?.id === 'magic-hotspot');
        const monitor = Main.layoutManager.findMonitorForActor(icon);
        const rectangle = r => ({x: r.x, y: r.y, width: r.width, height: r.height});
        return JSON.stringify({frame: rectangle(window.get_frame_rect()),
            icon: [...icon.get_transformed_position(), ...icon.get_transformed_size()],
            area: rectangle(Main.layoutManager.getWorkAreaForMonitor(monitor.index))});
    }

    async SnapshotAsync([path], invocation) {
        Main.overview.hide();
        const stream = Gio.File.new_for_path(path).replace(null, false, Gio.FileCreateFlags.NONE, null);
        await new Shell.Screenshot().screenshot(false, stream);
        stream.close(null);
        invocation.return_value(new GLib.Variant('(b)', [true]));
    }
}
