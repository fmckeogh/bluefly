#![no_std]
#![no_main]

extern crate panic_semihosting;

use core::fmt::Write;
use embedded_graphics::{coord::Coord, fonts::Font6x8, prelude::*};
use nrf52810_hal::{
    self as hal,
    gpio::{Level, Output, Pin, PushPull},
    nrf52810_pac::{self as device, SPIM0, UARTE0},
    prelude::*,
    spim::{Frequency, Spim, MODE_0},
    uarte::{Baudrate, Parity, Uarte},
};
use rtfm::app;

#[app(device = nrf52810_hal::nrf52810_pac)]
const APP: () = {
    static mut COUNT: u64 = ();
    static mut SERIAL: Uarte<UARTE0> = ();

    #[init]
    fn init() {
        let device: device::Peripherals = device;
        let p0 = device.P0.split();

        let mut serial = {
            let rxd = p0.p0_08.into_floating_input().degrade();
            let txd = p0.p0_31.into_push_pull_output(Level::Low).degrade();

            let pins = hal::uarte::Pins {
                rxd,
                txd,
                cts: None,
                rts: None,
            };

            device
                .UARTE0
                .constrain(pins, Parity::EXCLUDED, Baudrate::BAUD1M)
        };
        writeln!(serial, "init").unwrap();

        COUNT = 0;
        SERIAL = serial;
    }

    #[idle(resources = [COUNT, SERIAL])]
    fn idle() -> ! {
        loop {
            *resources.COUNT += 1;

            writeln!(resources.SERIAL, "idle, count: {}", resources.COUNT).unwrap();
        }
    }

    extern "C" {
        fn PDM();
    }
};
