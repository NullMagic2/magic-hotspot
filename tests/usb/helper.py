# Module: magic_hotspot.tests.usb_helper
# Purpose: Simulate authentication outcomes and cleanup without hardware access.
# Created: 2026-10-09
# Architecture: Mock fixed helper boundaries using a private temporary directory.
import importlib.util
from pathlib import Path
from tempfile import TemporaryDirectory
from types import SimpleNamespace
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('helper', 'packaging/usb_helper.py')
h = importlib.util.module_from_spec(spec)
spec.loader.exec_module(h)

for code in ('0', '-8', '-19'):
    with TemporaryDirectory(dir='work') as directory:
        root = Path(directory)
        runtime = root / 'runtime'
        runtime.mkdir()
        phone, interface = root / '1-4', root / '1-4:4.2'
        phone.mkdir(); interface.mkdir()
        (phone / 'busnum').write_text('1')
        (phone / 'devnum').write_text('12')
        driver = interface / 'driver'; driver.mkdir()
        carrier = root / 'net' / 'enxphone' / 'carrier'
        carrier.parent.mkdir(parents=True); carrier.write_text('0')
        changes, child = [], SimpleNamespace(poll=lambda:None, terminate=lambda:changes.append('engine-stop'), wait=lambda timeout=None:0)
        class Driver:
            def __truediv__(self, action):
                def write(name):
                    assert name == interface.name
                    changes.append(action)
                    if action == 'unbind': driver.rmdir()
                    else: driver.mkdir(exist_ok=True); carrier.write_text('1')
                return SimpleNamespace(write_text=write)
        def paths(value):
            if str(value) == '/sys/class/net': return root / 'net'
            if str(value) == '/sys/bus/usb/devices': return root
            if str(value) == '/run/systemd/system/usbmuxd.service': return root / 'mask'
            return Path(value)
        def run(args, timeout=10, check=True, env=None):
            changes.append(tuple(args))
            output = ''
            if 'is-active' in args: output = 'active'
            if 'GENERAL.NM-MANAGED' in args: output = 'yes'
            if '--handshake' in args: output = code
            return SimpleNamespace(returncode=0, stdout=output)
        with patch.multiple(h, RUNTIME=runtime, PRIVATE=runtime/'private', STATE=runtime/'private'/'restore.json', STATUS=runtime/'status', DRIVER=Driver(), Path=paths, run=run, same_phone=lambda state:True, find_phone=lambda:(phone, interface,'enxphone')), patch.object(h.subprocess, 'Popen', return_value=child):
            try: h.recover()
            except SystemExit as error: assert error.code == 1 and code != '0'
            else: assert code == '0'
            assert driver.exists(), 'driver not restored'
            assert not h.STATE.exists(), 'cleanup state remained'
            assert any('unmask' in x for x in changes if isinstance(x, tuple))
            assert any('start' in x and h.SERVICE in x for x in changes if isinstance(x, tuple))
            assert 'engine-stop' in changes
            assert h.PRIVATE.stat().st_mode & 0o777 == 0o700
            assert h.STATUS.stat().st_mode & 0o777 == 0o644
print('USB helper simulations passed: success, transport failure, pending Trust; driver/service cleanup and permissions.')

# Cleanup must not hide the authentication error when signaling a dropped-UID
# child is denied. The unit also needs CAP_KILL for that ownership transition.
import errno
unit = Path('packaging/magic-hotspot-usb.service').read_text()
assert 'CAP_KILL' in unit
assert h.io_error(PermissionError(errno.EPERM, 'private device path'), 'backend cleanup') == 'USB recovery failed during backend cleanup (EPERM).'
completed = SimpleNamespace(poll=lambda:0, terminate=lambda:(_ for _ in ()).throw(AssertionError('already exited')))
h.stop_engine(completed)
changes = []
def expired(timeout=None):
    if len(changes) == 1:
        raise h.subprocess.TimeoutExpired('engine', 3)
    return 0
child = SimpleNamespace(poll=lambda:None, terminate=lambda:changes.append('term'), kill=lambda:changes.append('kill'), wait=expired)
h.stop_engine(child)
assert changes == ['term', 'kill']
print('Cleanup checks passed: required cross-user signal capability, exited child, bounded kill escalation and sanitized errno.')
