#![no_std]
#![no_main]
#![feature(alloc)]
#![feature(lang_items)]

#[macro_use]
extern crate alloc;
extern crate panic_semihosting;

use alloc_cortex_m::CortexMHeap;
use cortex_m_semihosting::hprintln;
use embedded_graphics::{coord::Coord, fonts::Font6x8, prelude::*};
use nrf52810_hal::{
    self as hal,
    gpio::{Level, Output, Pin, PushPull},
    nrf52810_pac::{self as device, SPIM0},
    prelude::{GpioExt, SpimExt, _embedded_hal_digital_OutputPin},
    Spim,
};
use rtfm::app;
use ssd1306::{interface::spi::SpiInterface, mode::graphics::GraphicsMode, prelude::*, Builder};

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

#[app(device = nrf52810_hal::nrf52810_pac)]
const APP: () = {
    static mut COUNT: u64 = ();
    static mut DISPLAY: GraphicsMode<SpiInterface<Spim<SPIM0>, Pin<Output<PushPull>>>> = ();

    #[init]
    fn init() {
        hprintln!("init...").unwrap();

        // Allocator
        {
            let start = cortex_m_rt::heap_start() as usize;
            let size = 512; // in bytes
            unsafe { ALLOCATOR.init(start, size) }
        }

        let device: device::Peripherals = device;
        let p0 = device.P0.split();

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

                device.SPIM0.constrain(pins)
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

        hprintln!("init complete\n").unwrap();

        COUNT = 0;
        DISPLAY = display;
    }

    #[idle(resources = [COUNT, DISPLAY])]
    fn idle() -> ! {
        loop {
            *resources.COUNT += 1;

            resources.DISPLAY.draw(
                Font6x8::render_str(&format!("count: {}", resources.COUNT))
                    .translate(Coord::new(0, 0))
                    .into_iter(),
            );
            resources.DISPLAY.flush().unwrap();

            //hprintln!("idle, count: {}", resources.COUNT).unwrap();
        }
    }

    extern "C" {
        fn PDM();
    }
};

#[lang = "oom"]
#[no_mangle]
pub fn rust_oom(layout: core::alloc::Layout) -> ! {
    panic!("{:?}", layout);
}
