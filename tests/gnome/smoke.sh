#!/bin/bash
set -eu
# Module: magic_hotspot.tests.gnome_smoke
# Purpose: Verify native Wayland tray placement and render the result.
# Created: 2026-10-11
# Architecture: Run inside dbus-run-session with private GNOME configuration.
ROOT=$(cd -- "$(dirname -- "$0")/../.." && pwd)
TASK_DIR=$(mktemp -d "$ROOT/work/gnome-check/session.XXXXXX")
export XDG_CONFIG_HOME="$TASK_DIR/config" XDG_DATA_HOME="$TASK_DIR/data" XDG_CACHE_HOME="$TASK_DIR/cache" XDG_RUNTIME_DIR="$TASK_DIR/runtime"
mkdir -p "$XDG_CONFIG_HOME" "$XDG_DATA_HOME/gnome-shell/extensions/magic-hotspot@local" "$XDG_CACHE_HOME" "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"
cp "$ROOT/gnome/"* "$XDG_DATA_HOME/gnome-shell/extensions/magic-hotspot@local/"
mv "$XDG_DATA_HOME/gnome-shell/extensions/magic-hotspot@local/extension.js" "$XDG_DATA_HOME/gnome-shell/extensions/magic-hotspot@local/placement.js"
cp "$ROOT/tests/gnome/probe.js" "$XDG_DATA_HOME/gnome-shell/extensions/magic-hotspot@local/extension.js"
export XDG_SESSION_TYPE=wayland XDG_CURRENT_DESKTOP=GNOME GNOME_SHELL_SESSION_MODE=user WAYLAND_DISPLAY=magic-hotspot-test GTK_USE_PORTAL=0 NO_AT_BRIDGE=1
unset DISPLAY GDK_BACKEND
dbus-update-activation-environment XDG_CONFIG_HOME XDG_DATA_HOME XDG_CACHE_HOME XDG_RUNTIME_DIR XDG_SESSION_TYPE XDG_CURRENT_DESKTOP GNOME_SHELL_SESSION_MODE WAYLAND_DISPLAY GTK_USE_PORTAL NO_AT_BRIDGE
printf '%s\n' "$TASK_DIR"
gsettings set org.gnome.shell enabled-extensions "['ubuntu-appindicators@ubuntu.com']"
gsettings set org.gnome.shell disable-user-extensions false
gnome-shell --headless --wayland --virtual-monitor "${UI_RESOLUTION:-1600x1000}" --wayland-display magic-hotspot-test > "$TASK_DIR/shell.log" 2>&1 &
SHELL_PID=$!
trap 'kill "$SHELL_PID" 2>/dev/null || true' EXIT
for attempt in $(seq 1 40); do
 if gdbus call --session --dest org.gnome.Shell.Extensions --object-path /org/gnome/Shell/Extensions --method org.gnome.Shell.Extensions.GetExtensionInfo magic-hotspot@local 2>/dev/null | rg -q 'uuid'; then break; fi
 sleep 0.5
done
gdbus call --session --dest org.gnome.Shell.Extensions --object-path /org/gnome/Shell/Extensions --method org.gnome.Shell.Extensions.GetExtensionInfo magic-hotspot@local
if [ "${UI_SCALE:-1}" = 2 ]; then
/usr/bin/python3 - <<'PYSCALE'
import dbus
bus=dbus.SessionBus()
config=dbus.Interface(bus.get_object('org.gnome.Mutter.DisplayConfig','/org/gnome/Mutter/DisplayConfig'),'org.gnome.Mutter.DisplayConfig')
serial, monitors, _, _ = config.GetCurrentState()
spec,modes,_=monitors[0]
mode=next(m for m in modes if m[-1].get('is-current'))
config.ApplyMonitorsConfig(serial,1,[(0,0,2.0,0,True,[(spec[0],mode[0],dbus.Dictionary({},signature='sv'))])],dbus.Dictionary({},signature='sv'))
PYSCALE
fi
DBUS_SYSTEM_BUS_ADDRESS="$DBUS_SESSION_BUS_ADDRESS" GDK_BACKEND=wayland "$ROOT/bin/magic-hotspot" > "$TASK_DIR/app.log" 2>&1 &
APP_PID=$!
trap 'kill "$APP_PID" "$SHELL_PID" 2>/dev/null || true' EXIT
sleep 4
/usr/bin/python3 - <<'PYTEST'
import dbus, json, time, subprocess, os
from pathlib import Path
bus = dbus.SessionBus()
probe = dbus.Interface(bus.get_object('org.gnome.Shell', '/com/example/MagicHotspot/Test'), 'com.example.MagicHotspot.Test')
assert not probe.Visible(), 'default launch showed the window'
print('PASS: default startup stays hidden')
probe.Snapshot(str(Path.cwd()/'outputs/magic-hotspot-default-tray.png'))
watcher = bus.get_object('org.kde.StatusNotifierWatcher', '/StatusNotifierWatcher')
items = watcher.Get('org.kde.StatusNotifierWatcher', 'RegisteredStatusNotifierItems', dbus_interface='org.freedesktop.DBus.Properties')
for entry in items:
    dest, _, path = str(entry).partition('/')
    dest = dest.rstrip('@')
    obj = bus.get_object(dest, '/'+path)
    props = obj.GetAll('org.kde.StatusNotifierItem', dbus_interface='org.freedesktop.DBus.Properties')
    if props.get('Id') != 'magic-hotspot': continue
    menu = dbus.Interface(bus.get_object(dest, props['Menu']), 'com.canonical.dbusmenu')
    _, tree = menu.GetLayout(0, -1, [])
    open_id = next(child[0] for child in tree[2] if child[1].get('label') == 'Open')
    menu.AboutToShow(0)
    menu.Event(open_id, 'clicked', dbus.Int32(0, variant_level=1), dbus.UInt32(0))
    break
else: raise AssertionError('No native indicator')
time.sleep(1)
probe = dbus.Interface(bus.get_object('org.gnome.Shell', '/com/example/MagicHotspot/Test'), 'com.example.MagicHotspot.Test')
def check_placement(label):
    state = json.loads(probe.Inspect())
    f, a, icon = state['frame'], state['area'], state['icon']
    assert f['y'] == a['y'] + 12, state
    expected = max(a['x']+12, min(icon[0]+icon[2]/2-f['width']/2, a['x']+a['width']-f['width']-12))
    assert abs(f['x']-expected) <= 1, state
    print('PASS:', label, state)
check_placement('native menu Open below actual icon')
app = bus.get_object('com.example.MagicHotspot','/com/example/MagicHotspot')
app.Activate({}, dbus_interface='org.freedesktop.Application')
time.sleep(.5)
check_placement('dock activation remains below icon')
subprocess.run([str(Path.cwd()/'bin/magic-hotspot')], check=True)
time.sleep(.5)
check_placement('manual relaunch remains below icon')
app.Activate('quit', [], {}, dbus_interface='org.gtk.Actions')
time.sleep(.5)
subprocess.run(['gsettings','set','org.gnome.shell','enabled-extensions',"['ubuntu-appindicators@ubuntu.com']"], check=True)
time.sleep(.5)
with open(os.environ['XDG_CACHE_HOME']+'/foreground.log','w') as log:
    child = subprocess.Popen([str(Path.cwd()/'bin/magic-hotspot'), '--show'], stdout=log, stderr=log)
time.sleep(4)
check_placement('fresh foreground launch below icon')
probe.Snapshot(str(Path.cwd()/'outputs/magic-hotspot-gnome-native.png'))
app = bus.get_object('com.example.MagicHotspot','/com/example/MagicHotspot')
app.Activate('quit', [], {}, dbus_interface='org.gtk.Actions')
child.wait(timeout=5)

PYTEST
