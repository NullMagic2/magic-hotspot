// Module: magic_hotspot.gnome
// Purpose: Place our window using native panel and monitor geometry on Wayland.
// Created: 2026-10-11
// Architecture: A single session D-Bus method; only Magic Hotspot is movable.
import Gio from 'gi://Gio';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const INTERFACE = `<node><interface name="com.example.MagicHotspot.Tray">
    <method name="Place"><arg type="b" direction="out"/></method>
</interface></node>`;
const MARGIN = 12;

export default class TrayPlacement extends Extension {
    enable() {
        this._export = Gio.DBusExportedObject.wrapJSObject(INTERFACE, this);
        this._export.export(Gio.DBus.session, '/com/example/MagicHotspot/Tray');
    }

    disable() {
        this._export?.unexport();
        this._export = null;
    }

    Place() {
        // Ubuntu AppIndicators owns the actor, including its scaled allocation.
        const icon = Object.values(Main.panel.statusArea).find(
            item => item._indicator?.id === 'magic-hotspot');
        const window = global.get_window_actors().map(actor => actor.meta_window).find(
            item => item.get_gtk_application_id() === 'com.example.MagicHotspot' &&
                !item.get_transient_for());
        if (!icon?.visible || !window || !window.allows_move())
            return false;
        const monitor = Main.layoutManager.findMonitorForActor(icon);
        if (!monitor)
            return false;
        const area = Main.layoutManager.getWorkAreaForMonitor(monitor.index);
        const [iconX, iconY] = icon.get_transformed_position();
        const [iconWidth, iconHeight] = icon.get_transformed_size();
        const frame = window.get_frame_rect();
        const left = area.x + MARGIN;
        const right = Math.max(left, area.x + area.width - frame.width - MARGIN);
        const top = area.y + MARGIN;
        const bottom = Math.max(top, area.y + area.height - frame.height - MARGIN);
        const x = Math.max(left, Math.min(iconX + iconWidth / 2 - frame.width / 2, right));
        const y = iconY + iconHeight / 2 < area.y + area.height / 2 ? top : bottom;
        window.move_frame(true, Math.round(x), Math.round(y));
        Main.activateWindow(window);
        return true;
    }
}
