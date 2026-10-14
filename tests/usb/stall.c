// Module: magic_hotspot.tests.usb_stall
// Purpose: Exercise the private backend receive-stall state machine.
// Created: 2026-10-09
// Architecture: Compile with the patched USB source and mock USB operations.
#include <assert.h>
#include <stdlib.h>
#include <stdio.h>
#include "usb.c"
unsigned int log_level;
static int inside_callback, cleared, submitted, fail_clear;
void usbmuxd_log(enum loglevel level, const char *fmt, ...) {}
void device_data_input(struct usb_device *dev, unsigned char *buf, uint32_t len) { assert(0); }
int libusb_clear_halt(libusb_device_handle *handle, unsigned char endpoint) {
    assert(!inside_callback); cleared++; return fail_clear ? -1 : 0;
}
int libusb_submit_transfer(struct libusb_transfer *transfer) { submitted++; return 0; }
static void stall(struct usb_device *dev) {
    struct libusb_transfer *xfer = libusb_alloc_transfer(0);
    xfer->user_data = dev; xfer->status = LIBUSB_TRANSFER_STALL;
    xfer->buffer = malloc(1); collection_add(&dev->rx_xfers, xfer);
    inside_callback = 1; rx_callback(xfer); inside_callback = 0;
}
static void release(struct usb_device *dev) {
    FOREACH(struct libusb_transfer *xfer, &dev->rx_xfers) {
        free(xfer->buffer); libusb_free_transfer(xfer);
    } ENDFOREACH
    collection_free(&dev->rx_xfers);
    collection_init(&dev->rx_xfers);
}
int main(void) {
    struct usb_device dev = { .alive=1, .ep_in=0x85 };
    collection_init(&device_list); collection_init(&dev.rx_xfers);
    collection_add(&device_list,&dev);
    for (int i=0; i<3; i++) {
        stall(&dev); assert(dev.alive && dev.stalled_rx);
        assert(cleared==i); recover_stalled_rx();
        assert(dev.alive && !dev.stalled_rx && dev.stall_recoveries==(unsigned)i+1);
        assert(collection_count(&dev.rx_xfers)==NUM_RX_LOOPS); release(&dev);
    }
    stall(&dev); assert(!dev.alive); assert(cleared==3);
    dev.alive=1; dev.stall_recoveries=0; fail_clear=1;
    stall(&dev); recover_stalled_rx(); assert(!dev.alive);
    release(&dev); collection_free(&device_list);
    puts("C USB stall checks passed: callbacks defer clear-halt; receive loop restored; three-retry bound; clear-halt failure terminates.");
}
