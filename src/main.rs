#![cfg_attr(feature = "cargo-clippy", warn(clippy_pedantic))]
#![no_std]
#![no_main]
#![feature(alloc)]
#![feature(lang_items)]

#[macro_use]
extern crate alloc;
extern crate alloc_cortex_m;
extern crate cortex_m;
#[macro_use]
extern crate cortex_m_rt as rt;
extern crate embedded_graphics;
extern crate embedded_hal;
extern crate panic_semihosting;
extern crate ssd1306;
extern crate stm32f103xx_hal as hal;

use alloc_cortex_m::CortexMHeap;
use embedded_graphics::fonts::Font6x8;
use embedded_graphics::prelude::*;
use embedded_hal::spi::{Mode, Phase, Polarity};
use hal::delay::Delay;
use hal::prelude::*;
use hal::spi::Spi;
use hal::stm32f103xx;
use rt::ExceptionFrame;
use ssd1306::prelude::*;
use ssd1306::Builder;

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

entry!(main);

fn main() -> ! {
    let start = rt::heap_start() as usize;
    let size = 1024; // in bytes
    unsafe { ALLOCATOR.init(start, size) }

    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = stm32f103xx::Peripherals::take().unwrap();

    // Enable ADC clocks
    dp.RCC.apb2enr.write(|w| w.adc1en().set_bit());

    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();

    let clocks = rcc.cfgr.freeze(&mut flash.acr);
    let mut delay = Delay::new(cp.SYST, clocks);

    let mut gpio_a = dp.GPIOA.split(&mut rcc.apb2);
    let mut gpio_b = dp.GPIOB.split(&mut rcc.apb2);
    let mut gpio_c = dp.GPIOC.split(&mut rcc.apb2);

    let mut _led = gpio_c.pc13.into_push_pull_output(&mut gpio_c.crh);

    // DISPLAY
    let sck = gpio_b.pb13.into_alternate_push_pull(&mut gpio_b.crh);
    let miso = gpio_b.pb14;
    let mosi = gpio_b.pb15.into_alternate_push_pull(&mut gpio_b.crh);
    let mut rst = gpio_a.pa9.into_push_pull_output(&mut gpio_a.crh);
    let dc = gpio_a.pa8.into_push_pull_output(&mut gpio_a.crh);

    let spi = Spi::spi2(
        dp.SPI2,
        (sck, miso, mosi),
        Mode {
            polarity: Polarity::IdleLow,
            phase: Phase::CaptureOnFirstTransition,
        },
        8.mhz(),
        clocks,
        &mut rcc.apb1,
    );

    let mut disp: GraphicsMode<_> = Builder::new()
        .with_size(DisplaySize::Display128x64)
        .connect_spi(spi, dc)
        .into();

    disp.reset(&mut rst, &mut delay);
    disp.init().unwrap();
    disp.clear();
    disp.flush().unwrap();

    // Power on ADC
    dp.ADC1.cr2.write(|w| w.adon().set_bit());

    let mut sensor_val: u32;
    loop {
        // Perform single conversion
        dp.ADC1.cr2.write(|w| w.adon().set_bit());
        while dp.ADC1.sr.read().eoc().bit_is_clear() {}
        sensor_val = dp.ADC1.dr.read().data().bits().into();

        let sensor_pcnt = (sensor_val * 100) / 4096;

        disp.clear();
        disp.draw(
            Font6x8::render_str(&format!("{}%", sensor_pcnt))
                .translate((0, 0))
                .into_iter(),
        );
        disp.flush().unwrap();
    }
}

exception!(HardFault, hard_fault);

fn hard_fault(ef: &ExceptionFrame) -> ! {
    panic!("{:#?}", ef);
}

exception!(*, default_handler);

fn default_handler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}

#[lang = "oom"]
#[no_mangle]
pub fn rust_oom(layout: core::alloc::Layout) -> ! {
    panic!("{:?}", layout);
}
