# Module: magic_hotspot.usb_backend_build
# Purpose: Apply narrowly scoped recovery and isolation changes to upstream usbmuxd.
# Created: 2026-10-09
# Architecture: Patches a build copy; vendor sources and their licenses remain intact.
from pathlib import Path
import sys

root = Path(sys.argv[1])
def replace(file, old, new):
    path = root / 'src' / file
    text = path.read_text()
    if old not in text:
        raise SystemExit(f'Upstream source changed: {file}')
    path.write_text(text.replace(old, new))

replace('main.c', 'parse_opts(argc, argv);', '''const char *runtime = getenv("MAGIC_HOTSPOT_USB_RUNTIME");
    static char private_socket[108], private_lock[256];
    if (!runtime || strlen(runtime) > 80) return 2;
    snprintf(private_socket, sizeof(private_socket), "%s/socket", runtime);
    snprintf(private_lock, sizeof(private_lock), "%s/engine.pid", runtime);
    socket_path = private_socket;
    lockfile = private_lock;
    umask(0077);
    parse_opts(argc, argv);''')
replace('main.c', 'chmod(socket_path, 0666);', 'chmod(socket_path, 0600);')
# Filter before opening any USB device. The privileged helper selects this address.
replace('usb.c', 'uint8_t address = libusb_get_device_address(dev);', '''uint8_t address = libusb_get_device_address(dev);
    const char *selected = getenv("MAGIC_HOTSPOT_USB_ADDRESS");
    unsigned selected_bus, selected_address;
    if (!selected || sscanf(selected, "%u:%u", &selected_bus, &selected_address) != 2 ||
        bus != selected_bus || address != selected_address) return -1;''')
replace('usb.c', 'int alive;', 'int alive;\n    int stalled_rx;\n    unsigned stall_recoveries;')
replace('usb.c', 'dev->alive = 0;\n\t}\n}\n\n// Start a read-callback loop for this device\nstatic int start_rx_loop', '''if (xfer_status == LIBUSB_TRANSFER_STALL && dev->stall_recoveries < 3)
            dev->stalled_rx = 1;
        else
            dev->alive = 0;
    }
}

// Start a read-callback loop for this device
static int start_rx_loop''')
replace('usb.c', 'free(xfer->buffer);\n\t\tcollection_remove(&dev->rx_xfers, xfer);', 'enum libusb_transfer_status xfer_status = xfer->status;\n        free(xfer->buffer);\n\t\tcollection_remove(&dev->rx_xfers, xfer);')
# libusb_clear_halt is synchronous: call after callbacks return, never inside them.
replace('usb.c', 'int usb_process(void)', '''static void recover_stalled_rx(void)
{
    FOREACH(struct usb_device *dev, &device_list) {
        if (!dev->alive || !dev->stalled_rx) continue;
        dev->stalled_rx = 0;
        dev->stall_recoveries++;
        if (libusb_clear_halt(dev->dev, dev->ep_in) != 0) {
            dev->alive = 0;
            continue;
        }
        while (collection_count(&dev->rx_xfers) < NUM_RX_LOOPS) {
            if (start_rx_loop(dev) < 0) { dev->alive = 0; break; }
        }
    } ENDFOREACH
}
int usb_process(void)''')
path = root / 'src' / 'usb.c'
text = path.read_text()
before, after = text.split('int usb_process(void)', 1)
path.write_text(before + 'int usb_process(void)' + after.replace('reap_dead_devices();', 'recover_stalled_rx();\n\treap_dead_devices();'))
# Restore endpoint data toggles on first claim, before sending the version packet.
replace('usb.c', 'transfer = libusb_alloc_transfer(0);', '''libusb_clear_halt(handle, usbdev->ep_in);
    libusb_clear_halt(handle, usbdev->ep_out);
    transfer = libusb_alloc_transfer(0);''')
# New libplist exports these names itself; rename the older private enum.
for name in ('utils.h', 'utils.c', 'conf.c'):
    path = root / 'src' / name
    text = path.read_text().replace('plist_format_t', 'mux_plist_format_t')
    for suffix in ('XML', 'BINARY'):
        text = text.replace('PLIST_FORMAT_' + suffix, 'MUX_PLIST_FORMAT_' + suffix)
    path.write_text(text)
