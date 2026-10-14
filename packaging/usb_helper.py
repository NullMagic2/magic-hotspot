# Module: magic_hotspot.usb_helper
# Purpose: Authenticate and recover one iPhone USB link through a private mux backend.
# Created: 2026-10-09
# Architecture: Fixed systemd operation; root-only protocol socket and normal Apple Trust.
import ctypes as C
import errno
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

RUNTIME = Path('/run/magic-hotspot-usb')
PRIVATE = RUNTIME / 'private'
STATE = PRIVATE / 'restore.json'
STATUS = RUNTIME / 'status'
DRIVER = Path('/sys/bus/usb/drivers/ipheth')
ENGINE = '/usr/libexec/magic-hotspot-usb-engine'
SERVICE = 'usbmuxd.service'
ENV = {'PATH': '/usr/sbin:/usr/bin:/sbin:/bin', 'LC_ALL': 'C'}


def run(args, timeout=10, check=True, env=None):
    result = subprocess.run(args, env=env or ENV, capture_output=True,
                            text=True, timeout=timeout)
    if check and result.returncode:
        raise RuntimeError('A required system operation failed.')
    return result


def write_status(message):
    temporary = STATUS.with_suffix('.tmp')
    temporary.write_text(message + '\n')
    temporary.chmod(0o644)
    temporary.replace(STATUS)
    print(message, flush=True)


def read(path):
    return path.read_text().strip()


def io_error(error, stage):
    reason = errno.errorcode.get(error.errno, 'IO_ERROR')
    # Operation and errno are useful without revealing paths, identifiers or keys.
    return f'USB recovery failed during {stage} ({reason}).'


def stop_engine(engine):
    if engine.poll() is not None:
        return
    engine.terminate()
    try:
        engine.wait(timeout=3)
    except subprocess.TimeoutExpired:
        engine.kill()
        engine.wait(timeout=3)


def same_phone(state):
    path = Path('/sys/bus/usb/devices') / state['phone']
    try:
        return (read(path / 'idVendor') == '05ac'
                and read(path / 'busnum') == state['bus']
                and read(path / 'devnum') == state['address'])
    except OSError:
        return False


def restore():
    if not STATE.exists():
        return
    state = json.loads(STATE.read_text())
    errors = []
    try:
        if same_phone(state):
            interface = Path('/sys/bus/usb/devices') / state['interface']
            if interface.exists() and not (interface / 'driver').exists():
                (DRIVER / 'bind').write_text(state['interface'])
    except OSError:
        errors.append('The iPhone driver could not be restored; reconnect the cable.')
    try:
        if state['unmask']:
            run(['/usr/bin/systemctl', 'unmask', '--runtime', SERVICE])
        if state['active']:
            run(['/usr/bin/systemctl', 'start', SERVICE])
    except (RuntimeError, subprocess.TimeoutExpired):
        errors.append('Ubuntu’s phone service could not be restored.')
    if not errors:
        STATE.unlink(missing_ok=True)
    else:
        raise RuntimeError(' '.join(errors))


def find_phone():
    candidates = []
    for interface in Path('/sys/bus/usb/devices').glob('*:*'):
        driver = interface / 'driver'
        if not driver.exists() or driver.resolve().name != 'ipheth':
            continue
        phone = interface.resolve().parent
        if read(phone / 'idVendor') != '05ac':
            continue
        names = list((interface / 'net').glob('*'))
        if len(names) == 1:
            candidates.append((phone, interface, names[0].name))
    if len(candidates) != 1:
        raise RuntimeError('Connect exactly one iPhone by USB, then try again.')
    return candidates[0]


def handshake():
    # Library maintains Apple's pairing/session checks. No phone files are requested.
    lib = C.CDLL('libimobiledevice-1.0.so.6')
    lib.idevice_new_with_options.argtypes = [C.POINTER(C.c_void_p), C.c_char_p, C.c_int]
    lib.lockdownd_client_new_with_handshake.argtypes = [C.c_void_p, C.POINTER(C.c_void_p), C.c_char_p]
    lib.idevice_free.argtypes = [C.c_void_p]
    lib.lockdownd_client_free.argtypes = [C.c_void_p]
    device, client = C.c_void_p(), C.c_void_p()
    deadline = time.monotonic() + 5
    result = -3
    while time.monotonic() < deadline:
        result = lib.idevice_new_with_options(C.byref(device), None, 2)
        if result == 0:
            break
        time.sleep(0.25)
    try:
        if result == 0:
            result = lib.lockdownd_client_new_with_handshake(device, C.byref(client), b'Magic Hotspot')
        print(result)
    finally:
        if client.value:
            lib.lockdownd_client_free(client)
        if device.value:
            lib.idevice_free(device)


def recover():
    PRIVATE.mkdir(mode=0o700, parents=True, exist_ok=True)
    PRIVATE.chmod(0o700)
    restore()  # Recover unfinished cleanup before starting another attempt.
    phone, interface, name = find_phone()
    managed = run(['/usr/bin/nmcli', '--get-values', 'GENERAL.NM-MANAGED',
                   'device', 'show', name]).stdout.strip()
    if managed != 'yes':
        raise RuntimeError('The iPhone interface is not managed by NetworkManager.')
    if read(Path('/sys/class/net') / name / 'carrier') == '1':
        write_status('iPhone USB link is ready.')
        return
    active = run(['/usr/bin/systemctl', 'is-active', SERVICE], check=False).stdout.strip() == 'active'
    runtime_mask = Path('/run/systemd/system/usbmuxd.service')
    already_masked = runtime_mask.is_symlink() and runtime_mask.resolve() == Path('/dev/null')
    if runtime_mask.exists() and not already_masked:
        raise RuntimeError('Ubuntu’s phone service has a custom runtime configuration.')
    state = {'phone': phone.name, 'interface': interface.name,
             'bus': read(phone / 'busnum'), 'address': read(phone / 'devnum'),
             'active': active, 'unmask': not already_masked}
    STATE.write_text(json.dumps(state))
    STATE.chmod(0o600)
    engine = None
    message = 'USB recovery did not finish.'
    failure = False
    stage = 'preparation'
    try:
        write_status('Recovering the iPhone USB connection…')
        if not already_masked:
            run(['/usr/bin/systemctl', 'mask', '--runtime', SERVICE])
        run(['/usr/bin/systemctl', 'stop', SERVICE])
        if not same_phone(state):
            raise RuntimeError('The iPhone was disconnected. Reconnect it and try again.')
        stage = 'driver detach'
        (DRIVER / 'unbind').write_text(interface.name)
        env = dict(ENV, MAGIC_HOTSPOT_USB_RUNTIME=str(PRIVATE),
                   MAGIC_HOTSPOT_USB_ADDRESS=state['bus'] + ':' + state['address'],
                   USBMUXD_SOCKET_ADDRESS='UNIX:' + str(PRIVATE / 'socket'))
        # Create the root-only socket, then let upstream drop USB protocol parsing
        # to Ubuntu's dedicated usbmux account. Pairing files retain its ownership.
        stage = 'backend startup'
        engine = subprocess.Popen([ENGINE, '--foreground', '--no-preflight', '--user', 'usbmux'], env=env,
                                  stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        stage = 'authentication'
        result = run(['/usr/bin/python3', '-I', __file__, '--handshake'], timeout=25, env=env)
        code = result.stdout.strip()
        if code != '0':
            details = {'-17': 'Unlock your iPhone and try again.',
                       '-18': 'Trust was declined on the iPhone.',
                       '-19': 'Accept Trust on the iPhone, then try again.',
                       '-8': 'The USB transport still fails during authentication.',
                       '-3': 'The private backend could not discover the iPhone.'}
            raise RuntimeError(details.get(code, 'The iPhone did not authenticate with the private backend.'))
        if not same_phone(state):
            raise RuntimeError('The iPhone disconnected during authentication.')
        stage = 'driver restore'
        (DRIVER / 'bind').write_text(interface.name)
        # udev may rename eth0 after the synchronous driver bind returns.
        for _ in range(20):
            if (Path('/sys/class/net') / name).exists():
                break
            time.sleep(0.1)
        else:
            raise RuntimeError('The iPhone network interface did not return after recovery.')
        run(['/usr/sbin/ip', 'link', 'set', 'dev', name, 'up'])
        for _ in range(24):
            if not same_phone(state):
                raise RuntimeError('The iPhone disconnected during USB recovery.')
            if read(Path('/sys/class/net') / name / 'carrier') == '1':
                message = 'iPhone authenticated. USB link is ready.'
                break
            time.sleep(0.25)
        else:
            raise RuntimeError('iPhone authenticated, but the USB network link is still inactive.')
    except subprocess.TimeoutExpired:
        failure = True
        message = 'USB authentication timed out. Unlock your iPhone and try again.'
    except (RuntimeError, OSError):
        failure = True
        # Only our controlled messages are public; OS paths and device IDs stay private.
        error = sys.exc_info()[1]
        message = str(error) if isinstance(error, RuntimeError) else io_error(error, stage)
    finally:
        if engine is not None:
            try:
                stop_engine(engine)
            except (OSError, subprocess.TimeoutExpired):
                failure = True
                message += ' The private USB backend could not be stopped.'
        try:
            restore()
        except RuntimeError as error:
            failure = True
            message += ' ' + str(error)
        write_status(message)
    if failure:
        raise SystemExit(1)


if __name__ == '__main__':
    if os.geteuid() != 0:
        raise SystemExit('This helper must run through its system service.')
    if sys.argv[1:] == ['--handshake']:
        handshake()
    elif sys.argv[1:] == ['--restore']:
        restore()
    elif not sys.argv[1:]:
        def interrupted(signum, frame):
            raise RuntimeError('USB recovery was interrupted.')
        signal.signal(signal.SIGTERM, interrupted)
        signal.signal(signal.SIGINT, interrupted)
        try:
            recover()
        except RuntimeError as error:
            write_status(str(error))
            raise SystemExit(1)
        except OSError as error:
            write_status(io_error(error, 'device discovery'))
            raise SystemExit(1)
    else:
        raise SystemExit(2)
