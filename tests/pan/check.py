# Module: magic_hotspot.tests.pan
# Purpose: Exercise real nmcli subprocess/DBus observation against isolated fixtures.
# Run: dbus-run-session -- python3 tests/pan/check.py <Rust unit-test executable>
import gi, os, pathlib, subprocess, sys, tempfile, time
from gi.repository import Gio, GLib
bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
owner = Gio.bus_own_name_on_connection(bus, 'org.freedesktop.NetworkManager', Gio.BusNameOwnerFlags.NONE, None, None)
context = GLib.MainContext.default()
with tempfile.TemporaryDirectory(prefix='magic-pan-test-') as directory:
    root = pathlib.Path(directory)
    state = root/'state'
    state.write_text('30')
    info = Gio.DBusNodeInfo.new_for_xml('''<node><interface name="org.freedesktop.DBus.Properties"><method name="GetAll"><arg type="s" direction="in"/><arg type="a{sv}" direction="out"/></method></interface></node>''')
    def method(conn, sender, path, interface, name, args, invocation):
        stage = state.read_text().strip() or '30'
        if path.endswith('/active'):
            props = {'Uuid': GLib.Variant('s', 'selected')}
        else:
            value = int(stage)
            props = {'State': GLib.Variant('u',value), 'StateReason': GLib.Variant('(uu)',(value,39 if mode=='cancel' and value==30 else 0)), 'ActiveConnection': GLib.Variant('o','/active' if value==70 else '/')}
        invocation.return_value(GLib.Variant('(a{sv})',(props,)))
    bus.register_object('/device', info.interfaces[0], method, None, None)
    bus.register_object('/active', info.interfaces[0], method, None, None)
    cli=root/'nmcli'
    cli.write_text('''#!/usr/bin/python3
import os,pathlib,sys,time
root=pathlib.Path(os.environ['PAN_FIXTURE'])
if 'GENERAL.DBUS-PATH' in sys.argv:
    print('/device');sys.exit(0)
assert sys.argv[1:]==['--wait','90','connection','up','uuid','selected'],sys.argv
count=root/'count'; n=int(count.read_text())+1 if count.exists() else 1;count.write_text(str(n))
mode=os.environ['PAN_CASE']
if mode=='permission':
    print('not authorized',file=sys.stderr);sys.exit(4)
(root/'state').write_text('70');time.sleep(1.2)
(root/'state').write_text('30');time.sleep(.5)
if mode=='recover' and n==2:sys.exit(0)
print('user requested disconnect' if mode=='cancel' else 'The Bluetooth connection failed or timed out',file=sys.stderr)
sys.exit(4)
''')
    cli.chmod(0o755)
    for mode, count in [('recover',2),('persistent',2),('permission',1),('cancel',1)]:
        state.write_text('30');(root/'count').unlink(missing_ok=True)
        env=dict(os.environ,PATH=str(root)+':'+os.environ['PATH'], DBUS_SYSTEM_BUS_ADDRESS=os.environ['DBUS_SESSION_BUS_ADDRESS'], MAGIC_PAN_TEST='isolated', PAN_CASE=mode, PAN_FIXTURE=str(root))
        proc=subprocess.Popen([sys.argv[1],'--ignored','--exact','pan::tests::isolated_network_manager_recovery','--nocapture'],env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
        deadline=time.monotonic()+20
        while proc.poll() is None and time.monotonic()<deadline:
            while context.pending():context.iteration(False)
            time.sleep(.01)
        if proc.poll() is None:proc.kill()
        output=proc.communicate()[0]
        assert proc.returncode==0,output
        assert int((root/'count').read_text())==count,(mode,(root/'count').read_text())
        print('PASS:',mode,'activation attempts:',count,flush=True)
Gio.bus_unown_name(owner)
