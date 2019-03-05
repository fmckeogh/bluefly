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
use ssd1306::{interface::spi::SpiInterface, mode::graphics::GraphicsMode, prelude::*, Builder};

#[app(device = nrf52810_hal::nrf52810_pac)]
const APP: () = {
    static mut COUNT: u64 = ();
    static mut SERIAL: Uarte<UARTE0> = ();
    static mut DISPLAY: GraphicsMode<SpiInterface<Spim<SPIM0>, Pin<Output<PushPull>>>> = ();

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

        let display = {
            let spi = {
                let sck = p0.p0_25.into_push_pull_output(Level::Low).degrade();
                let miso = p0.p0_24.into_floating_input().degrade();
                let mosi = p0.p0_23.into_push_pull_output(Level::Low).degrade();

                let pins = hal::spim::Pins {
                    sck,
                    miso: Some(miso),
                    mosi: Some(mosi),
                };

                device.SPIM0.constrain(pins, Frequency::M8, MODE_0, 0)
            };

            let dc = p0.p0_09.into_push_pull_output(Level::Low).degrade();
            let mut rst = p0.p0_02.into_push_pull_output(Level::Low).degrade();

            let mut display: GraphicsMode<_> = Builder::new()
                .with_size(DisplaySize::Display128x64)
                .connect_spi(spi, dc)
                .into();

            rst.set_high();
            rst.set_low();
            rst.set_high();

            display.init().unwrap();
            display.clear();
            display.flush().unwrap();

            display
        };

        COUNT = 0;
        SERIAL = serial;
        DISPLAY = display;
    }

    #[idle(resources = [COUNT, DISPLAY, SERIAL])]
    fn idle() -> ! {
        loop {
            *resources.COUNT += 1;

            resources.DISPLAY.draw(
                Font6x8::render_str("test")
                    .translate(Coord::new(0, 0))
                    .into_iter(),
            );
            resources.DISPLAY.flush().unwrap();

            writeln!(resources.SERIAL, "idle, count: {}", resources.COUNT).unwrap();
        }
    }

    extern "C" {
        fn PDM();
    }
};
