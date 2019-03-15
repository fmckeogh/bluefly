#![no_std]
#![no_main]

extern crate panic_semihosting;

use core::fmt::Write;
use cortex_m_semihosting::hprintln;
use nrf52810_hal::{
    self as hal,
    gpio::{Level, Output, Pin, PushPull},
    nrf52810_pac::{self as device, SPIM0, UARTE0},
    prelude::*,
    spim::{Frequency, Spim, MODE_0},
    uarte::{Baudrate, Parity, Uarte},
};
use rtfm::app;

const PERIOD: u32 = 1000;

#[app(device = nrf52810_hal::nrf52810_pac)]
const APP: () = {
    #[init]
    fn init() {
        let device: device::Peripherals = device;
        let p0 = device.P0.split();

        let mut serial = {
            let rxd = p0.p0_08.into_floating_input().degrade();
            let txd = p0.p0_06.into_push_pull_output(Level::Low).degrade();

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
    }

    #[idle]
    fn idle() -> ! {
        loop {
            hprintln!("idle").unwrap();
        }
    }

    /*
    #[task(schedule = [periodic])]
    fn periodic() {
        hprintln!("periodic: scheduled:", scheduled).unwrap();

        schedule.periodic(scheduled + PERIOD.cycles()).unwrap();
    }
    */

    extern "C" {
        fn PDM();
    }
};
