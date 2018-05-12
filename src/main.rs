#![no_std]
#![deny(warnings)]
#![feature(const_fn)]
#![feature(proc_macro)]
#![feature(global_allocator)]
#![feature(alloc)]
#![feature(used)]
#![feature(lang_items)]

extern crate alloc_cortex_m;
extern crate cortex_m;
extern crate cortex_m_rtfm as rtfm;
extern crate cortex_m_rtfm_macros;
extern crate panic_abort;
extern crate ssd1306;
extern crate stm32f103xx_hal as hal;
#[macro_use]
extern crate alloc;
extern crate embedded_graphics;

use alloc_cortex_m::CortexMHeap;
use cortex_m::peripheral::syst::SystClkSource;
use cortex_m_rtfm_macros::app;
use hal::delay::Delay;
use hal::gpio::gpioa::{PA5, PA6, PA7};
use hal::gpio::gpiob::PB1;
use hal::gpio::{Alternate, Floating, Input, Output, PushPull};
use hal::prelude::*;
use hal::spi::{Spi, Mode, Phase, Polarity};
use hal::stm32f103xx::SPI1;
use rtfm::Threshold;
use ssd1306::prelude::*;
use ssd1306::Builder;
use embedded_graphics::Drawing;
use embedded_graphics::fonts::Font6x8;
use embedded_graphics::fonts::Font;
use embedded_graphics::prelude::Transform;

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

extern "C" {
    static mut _sheap: u32;
}

pub type OledDisplay = GraphicsMode<
    SpiInterface<
        Spi<
            SPI1,
            (
                PA5<Alternate<PushPull>>,
                PA6<Input<Floating>>,
                PA7<Alternate<PushPull>>,
            ),
        >,
        PB1<Output<PushPull>>, // B1 -> DC
    >,
>;

// Tasks and resources
app! {
    device: hal::stm32f103xx,

    resources: {
        static DISPLAY: OledDisplay;
        static STATE: bool;
        static COUNT: u64;
    },

    idle: {
        resources: [
            DISPLAY,
            STATE,
        ],
    },

    tasks: {
        SYS_TICK: {
            path: sys_tick,
            resources: [
                DISPLAY,
                STATE,
                COUNT,
            ],
        },
    },
}

fn init(p: init::Peripherals) -> init::LateResources {
    let heap_start = unsafe { &mut _sheap as *mut u32 as usize };
    unsafe { ALLOCATOR.init(heap_start, 1024) }

    let mut flash = p.device.FLASH.constrain();
    let mut rcc = p.device.RCC.constrain();

    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    let mut afio = p.device.AFIO.constrain(&mut rcc.apb2);

    let mut gpioa = p.device.GPIOA.split(&mut rcc.apb2);
    let mut gpiob = p.device.GPIOB.split(&mut rcc.apb2);

    let sck = gpioa.pa5.into_alternate_push_pull(&mut gpioa.crl);
    let miso = gpioa.pa6;
    let mosi = gpioa.pa7.into_alternate_push_pull(&mut gpioa.crl);

    let mut rst = gpiob.pb0.into_push_pull_output(&mut gpiob.crl);
    let dc = gpiob.pb1.into_push_pull_output(&mut gpiob.crl);

    let mut delay = Delay::new(p.core.SYST, clocks);

    // SPI1
    let spi = Spi::spi1(
        p.device.SPI1,
        (sck, miso, mosi),
        &mut afio.mapr,
        Mode {
            polarity: Polarity::IdleLow,
            phase: Phase::CaptureOnFirstTransition,
        },
        8.mhz(),
        clocks,
        &mut rcc.apb2,
    );

    let mut display: GraphicsMode<_> = Builder::new()
        .with_size(DisplaySize::Display128x64)
        .connect_spi(spi, dc)
        .into();

    display.reset(&mut rst, &mut delay);
    display.init().unwrap();
    display.flush().unwrap();

    let mut syst = delay.free();
    syst.set_clock_source(SystClkSource::Core);
    syst.set_reload(1_000_000);
    syst.enable_interrupt();
    syst.enable_counter();

    init::LateResources {
        DISPLAY: display,
        STATE: false,
        COUNT: 0,
    }
}

fn idle(_t: &mut Threshold, _r: idle::Resources) -> ! {
    loop {
        rtfm::wfi();
    }
}

fn sys_tick(_t: &mut Threshold, mut r: SYS_TICK::Resources) {
    r.DISPLAY.clear();

    write_display(&mut *r.DISPLAY, *r.STATE, *r.COUNT);

    r.DISPLAY.flush().unwrap();
    *r.COUNT += 1;
    *r.STATE = !*r.STATE;
}

fn write_display(display: &mut OledDisplay, state: bool, count: u64) {
    display.draw(
        Font6x8::render_str(&format!("STATE: {}", state))
            .translate((0, 0))
            .into_iter(),
    );
    display.draw(
        Font6x8::render_str(&format!("COUNT: {}", count))
            .translate((0, 12))
            .into_iter(),
    );
}

#[lang = "oom"]
#[no_mangle]
pub fn rust_oom() -> ! {
    panic!()
}
