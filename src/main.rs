#![deny(warnings)]
#![feature(proc_macro)]
#![feature(global_allocator)]
#![feature(alloc)]
#![feature(lang_items)]
#![no_std]

//extern crate alloc_cortex_m;
extern crate cortex_m;
extern crate cortex_m_rtfm as rtfm;
//#[macro_use]
//extern crate alloc;
extern crate embedded_graphics;
extern crate embedded_hal;
extern crate panic_abort;
extern crate ssd1306;
extern crate stm32f103xx_hal as hal;

//use alloc_cortex_m::CortexMHeap;
use cortex_m::peripheral::syst::SystClkSource;
use rtfm::{app, Threshold};

use embedded_hal::spi::{Mode, Phase, Polarity};
use hal::delay::Delay;
use hal::gpio::gpioa::{PA5, PA6, PA7};
use hal::gpio::gpiob::PB1;
use hal::gpio::{Alternate, Floating, Input, Output, PushPull};
use hal::spi::Spi;
use hal::prelude::*;
use hal::stm32f103xx;
use hal::stm32f103xx::SPI1;

//use embedded_graphics::fonts::{Font, Font6x8};
//use embedded_graphics::prelude::*;
//use embedded_graphics::Drawing;
use ssd1306::mode::GraphicsMode;
use ssd1306::prelude::*;
use ssd1306::Builder;

//#[global_allocator]
//static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

//extern "C" {
//    static mut _sheap: u32;
//}

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

app! {
    device: stm32f103xx,

    resources: {
        static STATE: bool;
        static COUNT: u64;
        static DISPLAY: OledDisplay;
    },

    tasks: {
        SYS_TICK: {
            path: sys_tick,
            resources: [STATE, COUNT, DISPLAY],
        },
    },
}

fn init(p: init::Peripherals) -> init::LateResources {
    //let heap_start = unsafe { &mut _sheap as *mut u32 as usize };
    //unsafe { ALLOCATOR.init(heap_start, 1024) }

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

    let mut display: GraphicsMode<_> = Builder::new().connect_spi(spi, dc).into();


    display.reset(&mut rst, &mut delay);
    display.init().unwrap();
    display.flush().unwrap();

    let mut syst = delay.free();
    syst.set_clock_source(SystClkSource::Core);
    syst.set_reload(8_000_000);
    syst.enable_interrupt();
    syst.enable_counter();

    init::LateResources {
        DISPLAY: display,
        STATE: false,
        COUNT: 0,
    }
}

fn idle() -> ! {
    loop {
        rtfm::wfi();
    }
}

fn sys_tick(_t: &mut Threshold, mut r: SYS_TICK::Resources) {
    /*
    let state: &'static mut bool = r.STATE;
    let mut display: &'static mut OledDisplay = r.DISPLAY;

    let mut display: GraphicsMode<_> = Builder::new()
        .with_size(DisplaySize::Display128x32)
        .with_i2c_addr(0x3C)
        .connect_i2c(i2c1)
        .into();
    */

    r.DISPLAY.clear();

    match *r.STATE {
        true => draw_square(&mut r.DISPLAY, 0, 0),
        false => draw_square(&mut r.DISPLAY, 6, 0),
    }

    r.DISPLAY.flush().unwrap();
    *r.COUNT += 1;
    *r.STATE = !*r.STATE;
}
/*
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
*/

fn draw_square(disp: &mut OledDisplay, xoffset: u32, yoffset: u32) {
    // Top side
    disp.set_pixel(xoffset + 0, yoffset + 0, 1);
    disp.set_pixel(xoffset + 1, yoffset + 0, 1);
    disp.set_pixel(xoffset + 2, yoffset + 0, 1);
    disp.set_pixel(xoffset + 3, yoffset + 0, 1);

    // Right side
    disp.set_pixel(xoffset + 3, yoffset + 0, 1);
    disp.set_pixel(xoffset + 3, yoffset + 1, 1);
    disp.set_pixel(xoffset + 3, yoffset + 2, 1);
    disp.set_pixel(xoffset + 3, yoffset + 3, 1);

    // Bottom side
    disp.set_pixel(xoffset + 0, yoffset + 3, 1);
    disp.set_pixel(xoffset + 1, yoffset + 3, 1);
    disp.set_pixel(xoffset + 2, yoffset + 3, 1);
    disp.set_pixel(xoffset + 3, yoffset + 3, 1);

    // Left side
    disp.set_pixel(xoffset + 0, yoffset + 0, 1);
    disp.set_pixel(xoffset + 0, yoffset + 1, 1);
    disp.set_pixel(xoffset + 0, yoffset + 2, 1);
    disp.set_pixel(xoffset + 0, yoffset + 3, 1);
}

/*
#[lang = "oom"]
#[no_mangle]
pub fn rust_oom() -> ! {
    loop { }
}
*/