# Module: magic_hotspot.tests.ui_smoke
# Purpose: Verify real GTK tray click/placement and Bluetooth discovery in an isolated display/bus.
# Created: 2026-10-11
# Architecture: Xvfb/XFCE and fake BlueZ/nmcli; never touches the user's radios or desktop.
import os, pathlib, subprocess, tempfile, time
import gi
gi.require_version('Gtk', '3.0')
from gi.repository import Gio, GLib

root = pathlib.Path(__file__).resolve().parents[2]
scale = int(os.environ.get('UI_SCALE', '1'))
screen_width, screen_height = 1280 * scale, 900 * scale
children = []
started = stopped = 0
bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
props = lambda **kw: {k: GLib.Variant(t, v) for k,(t,v) in kw.items()}
data = {
 '/org/bluez/hci0': {'org.bluez.Adapter1': props(Powered=('b', True))},
 '/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF': {'org.bluez.Device1': props(Address=('s','AA:BB:CC:DD:EE:FF'), Alias=('s','Test Phone'), Class=('u',0x0204), UUIDs=('as',['00001116-0000-1000-8000-00805f9b34fb']), Paired=('b',True))},
}
def method(connection, sender, path, interface, name, args, invocation):
    global started, stopped
    if name == 'GetManagedObjects': invocation.return_value(GLib.Variant('(a{oa{sa{sv}}})', (data,)))
    elif name == 'StartDiscovery': started += 1; invocation.return_value(None)
    elif name == 'StopDiscovery': stopped += 1; invocation.return_value(None)
info = Gio.DBusNodeInfo.new_for_xml('<node><interface name="org.freedesktop.DBus.ObjectManager"><method name="GetManagedObjects"><arg type="a{oa{sa{sv}}}" direction="out"/></method></interface></node>')
bus.register_object('/', info.interfaces[0], method, None, None)
info2 = Gio.DBusNodeInfo.new_for_xml('<node><interface name="org.bluez.Adapter1"><method name="StartDiscovery"/><method name="StopDiscovery"/></interface></node>')
bus.register_object('/org/bluez/hci0', info2.interfaces[0], method, None, None)
owner = Gio.bus_own_name_on_connection(bus, 'org.bluez', Gio.BusNameOwnerFlags.NONE, None, None)
context = GLib.MainContext.default()
def wait(seconds):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        while context.pending(): context.iteration(False)
        time.sleep(.02)
def run(*args): return subprocess.check_output(args, env=env, text=True, stderr=subprocess.DEVNULL)
def visible_app():
    try: ids = run('xdotool','search','--onlyvisible','--name','^Magic Hotspot$').splitlines()
    except subprocess.CalledProcessError: return None
    for id in ids:
        values = dict(line.split('=',1) for line in run('xdotool','getwindowgeometry','--shell',id).splitlines() if '=' in line)
        if int(values.get('WIDTH',0)) > 300: return values
    return None
try:
    with tempfile.TemporaryDirectory(prefix='magic-hotspot-ui-', dir=root/'work') as temp:
        temp = pathlib.Path(temp)
        config = temp/'config'; config.mkdir()
        gtk_config = config/'gtk-3.0'; gtk_config.mkdir()
        (gtk_config/'settings.ini').write_text('[Settings]\ngtk-decoration-layout=icon:minimize,maximize,close\n')
        xml = config/'xfce4/xfconf/xfce-perchannel-xml'; xml.mkdir(parents=True)
        (xml/'xfce4-panel.xml').write_text('''<?xml version="1.0" encoding="UTF-8"?><channel name="xfce4-panel" version="1.0"><property name="configver" type="int" value="2"/><property name="panels" type="array"><value type="int" value="1"/><property name="panel-1" type="empty"><property name="position" type="string" value="p=6;x=640;y=16;"/><property name="position-locked" type="bool" value="true"/><property name="size" type="uint" value="32"/><property name="length" type="uint" value="5"/><property name="plugin-ids" type="array"><value type="int" value="1"/></property></property></property><property name="plugins" type="empty"><property name="plugin-1" type="string" value="systray"/></property></channel>''')
        nmcli = temp/'nmcli'
        nmcli.write_text('''#!/usr/bin/python3
import sys, os, time, pathlib
args=sys.argv[1:]
if 'UUID,TYPE' in args: print('test-pan:bluetooth')
elif 'bluetooth.bdaddr,bluetooth.type' in args:
    assert args[:2] == ['--escape','no']
    print('AA:BB:CC:DD:EE:FF\\npanu')
elif 'up' in args and os.environ.get('UI_CONNECT_TEST'):
    time.sleep(2)
    print('Bluetooth connected, but hotspot IP setup failed again. Turn Personal Hotspot off and on, then try again. If it still fails, restart the phone.' if os.environ.get('UI_PAN_GUIDANCE') else 'Error: Connection activation failed: The Bluetooth connection failed or timed out\\nHint: use journalctl for details.',file=sys.stderr)
    sys.exit(10)
elif 'status' in args:
    if os.environ.get('UI_SLOW_SCAN'): print('wlan0:wifi:disconnected')
elif args == ['radio','wifi']: print('enabled')
elif 'SSID,SIGNAL,SECURITY' in args:
    if os.environ.get('UI_SLOW_SCAN'):
        marker = pathlib.Path(os.environ['UI_SCAN_MARKER'])
        marker.touch()
        time.sleep(20)
        marker.with_suffix('.done').touch()
    print('Test Hotspot:90:WPA2')
elif 'connectivity' in args: print('full')
'''); nmcli.chmod(0o755)
        env = os.environ.copy(); env['LANGUAGE']='en'; env['LC_ALL']='C.UTF-8'; env['GDK_SCALE'] = str(scale); env['XDG_CURRENT_DESKTOP'] = 'XFCE'; env['UI_SCAN_MARKER'] = str(temp/'scan'); env.update(DISPLAY=':93', GDK_BACKEND='x11', XDG_CONFIG_HOME=str(config), XDG_CACHE_HOME=str(temp/'cache'), GTK_USE_PORTAL='0', NO_AT_BRIDGE='1', DBUS_SYSTEM_BUS_ADDRESS=os.environ['DBUS_SESSION_BUS_ADDRESS'], PATH=str(temp)+':'+os.environ['PATH'])
        bus.call_sync('org.freedesktop.DBus', '/org/freedesktop/DBus', 'org.freedesktop.DBus', 'UpdateActivationEnvironment', GLib.Variant('(a{ss})', ({k: env[k] for k in ['DISPLAY','GDK_BACKEND','XDG_CONFIG_HOME','XDG_CACHE_HOME','DBUS_SYSTEM_BUS_ADDRESS','GTK_USE_PORTAL','NO_AT_BRIDGE']},)), None, Gio.DBusCallFlags.NONE, 3000, None)
        log = open(root/'work/ui-smoke.log', 'w')
        children.append(subprocess.Popen(['Xvfb', ':93', '-screen', '0', f'{screen_width}x{screen_height}x24'], env=env, stdout=log, stderr=log)); wait(1)
        children.append(subprocess.Popen(['xfwm4', '--compositor=off'], env=env, stdout=log, stderr=log)); wait(1)
        children.append(subprocess.Popen([str(root/'bin/magic-hotspot')], env=env, stdout=log, stderr=log))
        app_process = children[-1]
        for _ in range(60):
            wait(.1)
            assert visible_app() is None, 'background startup mapped a window before the tray was ready'
        children.append(subprocess.Popen(['xfce4-panel', '--disable-wm-check'], env=env, stdout=log, stderr=log)); wait(2)
        assert visible_app() is None, 'background startup showed the app after the tray appeared'
        assert started == 1 and stopped == 1, (started, stopped)
        # The real XFCE panel must own the native StatusNotifier host.
        items = bus.call_sync('org.kde.StatusNotifierWatcher','/StatusNotifierWatcher',
            'org.freedesktop.DBus.Properties','Get',GLib.Variant('(ss)',('org.kde.StatusNotifierWatcher','RegisteredStatusNotifierItems')),
            None,Gio.DBusCallFlags.NONE,3000,None).unpack()[0]
        assert items, 'XFCE did not register the native indicator'
        destination, item_path = None, '/org/ayatana/NotificationItem/magic_hotspot'
        for entry in items:
            candidate = entry.split('@')[0].split('/')[0]
            try:
                properties = bus.call_sync(candidate,item_path,'org.freedesktop.DBus.Properties','GetAll',
                    GLib.Variant('(s)',('org.kde.StatusNotifierItem',)),None,Gio.DBusCallFlags.NONE,1000,None).unpack()[0]
                if properties.get('Id') == 'magic-hotspot': destination = candidate; break
            except GLib.Error: continue
        assert destination, items
        assert pathlib.Path(properties['IconName']).read_bytes() == (root/'assets/magic-hotspot.svg').read_bytes(), 'panel did not receive the Wi-Fi app icon'
        menu_path = properties['Menu']
        layout = bus.call_sync(destination,menu_path,'com.canonical.dbusmenu','GetLayout',
            GLib.Variant('(iias)',(0,-1,[])),None,Gio.DBusCallFlags.NONE,2000,None).unpack()[1]
        actions = {child[1].get('label'): child[0] for child in layout[2]}
        assert set(actions) == {'Open','Quit'}, actions
        def activate(label):
            bus.call_sync(destination,menu_path,'com.canonical.dbusmenu','Event',
                GLib.Variant('(isvu)',(actions[label],'clicked',GLib.Variant('i',0),0)),None,Gio.DBusCallFlags.NONE,2000,None)
            wait(.5)
        run('xdotool','mousemove',str(12*scale),str(16*scale),'click','3'); wait(.3)
        # Exercise the desktop-owned context menu, not a GTK application grab.
        run('xdotool','key','Escape'); wait(.2)
        bus.call_sync(destination,menu_path,'com.canonical.dbusmenu','AboutToShow',GLib.Variant('(i)',(0,)),None,Gio.DBusCallFlags.NONE,2000,None)
        # Selection can occur far from the icon, especially with keyboard input.
        run('xdotool','mousemove',str(800*scale),str(400*scale))
        activate('Open')
        run('xdotool','mousemove',str(700*scale),str(800*scale)); wait(.1)
        app = visible_app()
        assert app, 'native menu Open did not open application'
        gi.require_version('Gdk', '3.0')
        from gi.repository import Gdk
        display = Gdk.Display.open(':93')
        screen = display.get_default_screen()
        pixbuf = Gdk.pixbuf_get_from_window(screen.get_root_window(), 0, 0, screen_width, screen_height)
        pixbuf.savev(str(root/f'outputs/magic-hotspot-native-{scale}x.png'), 'png', [], [])
        display.close()
        if os.environ.get('UI_LANGUAGE_TEST'):
            run('xdotool','mousemove',str(int(app['X'])+35*scale),str(int(app['Y'])+20*scale),'click','1');wait(.3)
            display=Gdk.Display.open(':93')
            image=Gdk.pixbuf_get_from_window(display.get_default_screen().get_root_window(),0,0,screen_width,screen_height)
            image.savev(str(root/'outputs/magic-hotspot-language-menu.png'),'png',[],[])
            display.close()
            run('xdotool','key','Escape');wait(.2)
        # X window geometry includes theme-dependent client shadows.
        assert 0 < int(app['X']) < 40*scale, app
        assert 0 < int(app['Y']) < 90*scale, app
        assert int(app['X']) + int(app['WIDTH']) < screen_width, app
        assert int(app['Y']) + int(app['HEIGHT']) < screen_height, app
        bus.call_sync('com.example.MagicHotspot','/com/example/MagicHotspot','org.freedesktop.Application','Activate',GLib.Variant('(a{sv})',({},)),None,Gio.DBusCallFlags.NONE,2000,None); wait(.3)
        restored = visible_app()
        assert restored and (restored['X'],restored['Y']) == (app['X'],app['Y']), 'dock activation recentered the tray window'
        if os.environ.get('UI_CONNECT_TEST'):
            # Open the Test Phone transport menu, then choose its enabled Bluetooth entry.
            run('xdotool','windowactivate','--sync',app['WINDOW'])
            run('xdotool','key','Tab','Tab','space')
            wait(.2); run('xdotool','key','End','Return'); wait(.4)
            def capture_dialog(name):
                ids = run('xdotool','search','--onlyvisible','--name','^Connection( failed)?$').splitlines()
                assert ids, 'connection popup did not appear'
                display = Gdk.Display.open(':93')
                pixbuf = Gdk.pixbuf_get_from_window(display.get_default_screen().get_root_window(),0,0,screen_width,screen_height)
                pixbuf.savev(str(root/f'outputs/magic-hotspot-{name}.png'),'png',[],[])
                display.close()
            capture_dialog('connecting')
            wait(2.2); capture_dialog('connection-error')
            run('xdotool','key','Escape'); wait(.2)
        run(str(root/'bin/magic-hotspot')); wait(.5)
        centered = visible_app()
        assert centered, 'manual activation did not show the existing application'
        assert (centered['X'], centered['Y']) == (app['X'], app['Y']), 'manual launch recentered the tray window'
        # Send the same WM_DELETE_WINDOW event as the desktop close button.
        import ctypes
        xlib = ctypes.CDLL('libX11.so.6')
        class Client(ctypes.Structure):
            _fields_ = [('type',ctypes.c_int),('serial',ctypes.c_ulong),('send_event',ctypes.c_int),('display',ctypes.c_void_p),('window',ctypes.c_ulong),('message_type',ctypes.c_ulong),('format',ctypes.c_int),('data',ctypes.c_long*5)]
        xlib.XOpenDisplay.restype=ctypes.c_void_p
        xd=xlib.XOpenDisplay(b':93')
        xlib.XInternAtom.argtypes=[ctypes.c_void_p,ctypes.c_char_p,ctypes.c_int];xlib.XInternAtom.restype=ctypes.c_ulong
        event=Client();event.type=33;event.display=xd;event.window=int(centered['WINDOW']);event.format=32
        event.message_type=xlib.XInternAtom(xd,b'WM_PROTOCOLS',0);event.data[0]=xlib.XInternAtom(xd,b'WM_DELETE_WINDOW',0)
        xlib.XSendEvent.argtypes=[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_long,ctypes.c_void_p]
        # XEvent is a larger union; reserve its full ABI storage.
        storage=ctypes.create_string_buffer(192);ctypes.memmove(storage,ctypes.byref(event),ctypes.sizeof(event))
        xlib.XSendEvent(xd,event.window,0,0,storage)
        xlib.XCloseDisplay.argtypes=[ctypes.c_void_p];xlib.XCloseDisplay(xd);wait(.5)
        assert app_process.poll() is None and visible_app() is None, 'Close did not keep the application in the tray'
        run(str(root/'bin/magic-hotspot')); wait(.4)
        assert visible_app() is None, 'repeated default launch showed the hidden window'
        if os.environ.get('UI_SLOW_SCAN'):
            assert (temp/'scan').exists() and not (temp/'scan.done').exists(), 'slow scan was not in progress during UI checks'
        activate('Open')
        assert visible_app(), 'native tray did not reopen the hidden app'
        activate('Quit')
        assert app_process.poll() == 0, 'native Quit did not exit'
        print(f'PASS: XFCE native Wi-Fi tray icon/menu at {scale}x; default hidden startup; repeated launch hidden; tray margin; manual restoration; Close/reopen; Quit; scoped Bluetooth discovery.')

finally:
    for child in reversed(children):
        if child.poll() is None: child.terminate()
        try: child.wait(timeout=3)
        except subprocess.TimeoutExpired: child.kill(); child.wait()
    Gio.bus_unown_name(owner)
