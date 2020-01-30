#![no_std]
#![no_main]

extern crate itsybitsy_m0 as hal;
extern crate panic_halt;
extern crate usbd_hid;
extern crate usb_device;
extern crate cortex_m;

use hal::clock::GenericClockController;
use hal::prelude::*;
use hal::entry;
use hal::pac::{interrupt, CorePeripherals, Peripherals};

use hal::usb::UsbBus;
use usb_device::bus::UsbBusAllocator;
use usb_device::prelude::*;
use usbd_hid::hid_class::{HIDClass};
use usbd_hid::descriptor::{SerializedDescriptor, MouseReport};

use cortex_m::asm::delay as cycle_delay;
use cortex_m::peripheral::NVIC;
use cortex_m::interrupt::free as disable_interrupts;

#[entry]
fn main() -> ! {
    let mut peripherals = Peripherals::take().unwrap();
    let mut core = CorePeripherals::take().unwrap();
    let mut clocks = GenericClockController::with_internal_32kosc(
        peripherals.GCLK,
        &mut peripherals.PM,
        &mut peripherals.SYSCTRL,
        &mut peripherals.NVMCTRL,
    );
    let mut pins = hal::Pins::new(peripherals.PORT);
    let mut red_led = pins.d13.into_open_drain_output(&mut pins.port);

    let bus_allocator = unsafe {
        USB_ALLOCATOR = Some(hal::usb_allocator(
            peripherals.USB,
            &mut clocks,
            &mut peripherals.PM,
            pins.usb_dm,
            pins.usb_dp,
            &mut pins.port,
        ));
        USB_ALLOCATOR.as_ref().unwrap()
    };

    unsafe {
        USB_HID = Some(HIDClass::new(&bus_allocator, MouseReport::desc(), 60));
        USB_BUS = Some(
            UsbDeviceBuilder::new(&bus_allocator, UsbVidPid(0x16c0, 0x27dd))
                .manufacturer("Fake company")
                .product("Twitchy Mousey")
                .serial_number("TEST")
                .device_class(0xEF) // misc
                .build(),
        );
    }

    unsafe {
        core.NVIC.set_priority(interrupt::USB, 1);
        NVIC::unmask(interrupt::USB);
    }

    loop {
        flash_red_led(&mut red_led);
        push_mouse_movement(MouseReport{buttons: 0, x: 0, y: 5}).unwrap();
        cycle_delay(25 * 1024 * 1024);

        flash_red_led(&mut red_led);
        push_mouse_movement(MouseReport{buttons: 0, x: 0, y: -5}).unwrap();
        cycle_delay(25 * 1024 * 1024);
    }
}

fn push_mouse_movement(report: MouseReport) -> Result<usize, usb_device::UsbError> {
    disable_interrupts(|_| {
        unsafe {
            USB_HID.as_mut().map(|hid| {
                hid.push_input(&core::mem::transmute::<MouseReport, [u8; 3]>(report))
            })
        }
    }).unwrap()
}

fn flash_red_led(red_led: &mut hal::gpio::Pa17<hal::gpio::Output<hal::gpio::OpenDrain>>) {
    red_led.set_high().unwrap();
    cycle_delay(50 * 1024);
    red_led.set_low().unwrap();
}

static mut USB_ALLOCATOR: Option<UsbBusAllocator<UsbBus>> = None;
static mut USB_BUS: Option<UsbDevice<UsbBus>> = None;
static mut USB_HID: Option<HIDClass<UsbBus>> = None;

fn poll_usb() {
    unsafe {
        USB_BUS.as_mut().map(|usb_dev| {
            USB_HID.as_mut().map(|hid| {
                usb_dev.poll(&mut [hid]);
            });
        });
    };
}

#[interrupt]
fn USB() {
    poll_usb();
}
