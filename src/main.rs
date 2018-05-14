#![no_std]
#![deny(warnings)]
#![feature(const_fn)]
#![feature(proc_macro)]
#![feature(global_allocator)]
#![feature(alloc)]
#![feature(used)]
#![feature(lang_items)]
#![feature(extern_prelude)]

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
use hal::stm32f103xx::interrupt::Interrupt;
use hal::delay::Delay;
use hal::prelude::*;
use hal::spi::{Mode, Phase, Polarity, Spi};
use rtfm::Threshold;
use ssd1306::mode::GraphicsMode;
use ssd1306::prelude::*;
use ssd1306::Builder;

mod display;
use display::*;

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

extern "C" {
    static mut _sheap: u32;
}

// Tasks and resources
app! {
    device: hal::stm32f103xx,

    resources: {
        static DISPLAY: OledDisplay;
        static COUNT: u64;
        static EXTI: hal::stm32f103xx::EXTI;
    },

    tasks: {
        SYS_TICK: {
            path: sys_tick,
            priority: 2,
            resources: [
                DISPLAY,
                COUNT,
            ],
        },

        EXTI0: {
            path: exti0,
            priority: 1,
            resources: [
                COUNT,
                EXTI,
            ],
        }
    },
}

// Initalisation routine
fn init(mut p: init::Peripherals) -> init::LateResources {
    let heap_start = unsafe { &mut _sheap as *mut u32 as usize };
    unsafe { ALLOCATOR.init(heap_start, 1024) }

    let mut flash = p.device.FLASH.constrain();
    let mut rcc = p.device.RCC.constrain();
    let clocks = rcc.cfgr.freeze(&mut flash.acr);
    let mut delay = Delay::new(p.core.SYST, clocks);

    p.device.AFIO.exticr1.write(|w| unsafe { w.exti0().bits(0) });

    let mut afio = p.device.AFIO.constrain(&mut rcc.apb2);
    let mut gpioa = p.device.GPIOA.split(&mut rcc.apb2);
    let mut gpiob = p.device.GPIOB.split(&mut rcc.apb2);

    let sck = gpioa.pa5.into_alternate_push_pull(&mut gpioa.crl);
    let miso = gpioa.pa6;
    let mosi = gpioa.pa7.into_alternate_push_pull(&mut gpioa.crl);

    let mut rst = gpiob.pb12.into_push_pull_output(&mut gpiob.crh);
    let dc = gpiob.pb1.into_push_pull_output(&mut gpiob.crl);

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

    // Init display
    let mut display: GraphicsMode<_> = Builder::new()
        .with_size(DisplaySize::Display128x64)
        .connect_spi(spi, dc)
        .into();
    display.reset(&mut rst, &mut delay);
    display.init().unwrap();
    display.flush().unwrap();

    // Set up timer interrupt
    let mut syst = delay.free();
    syst.set_clock_source(SystClkSource::Core);
    syst.set_reload(250_000);
    syst.enable_interrupt();
    syst.enable_counter();

    // Set up interrupt on PA0
    let _int0 = gpioa.pa0.into_floating_input(&mut gpioa.crl);
    unsafe {
        p.core.NVIC.set_priority(stm32f103xx_hal::stm32f103xx::interrupt::Interrupt::EXTI0, 1);
    }
    p.core.NVIC.enable(
        stm32f103xx_hal::stm32f103xx::interrupt::Interrupt::EXTI0,
    );
    p.device.EXTI.imr.write(|w| w.mr0().set_bit()); // unmask the interrupt (EXTI)
    p.device.EXTI.emr.write(|w| w.mr0().set_bit());
    p.device.EXTI.rtsr.write(|w| w.tr0().set_bit()); // trigger interrupt on falling edge
    rtfm::set_pending(Interrupt::EXTI0);

    init::LateResources {
        DISPLAY: display,
        COUNT: 0,
        EXTI: p.device.EXTI,
    }
}

fn idle() -> ! {
    loop {
        rtfm::wfi();
    }
}

// Interrupt routine to read sensors and send data to VESC
fn sys_tick(_t: &mut Threshold, mut r: SYS_TICK::Resources) {

    let displaydata: DisplayData = DisplayData {
        local_bat: (*r.COUNT) as u8,
        remote_bat: 43,
        current: 51.12,
        mode: 1,

        // Distance
        speed: 4.7341,
        distance_travelled: 6144,
        distance_remaining: 1236,

        // Signal
        signal_strength: *r.COUNT as u8,
    };

    r.DISPLAY.clear();
    write_display(&mut *r.DISPLAY, displaydata);
    r.DISPLAY.flush().unwrap();

    *r.COUNT += 1;
}

// Interrupt to receive telemetry packets and display
fn exti0(_t: &mut Threshold, mut r: EXTI0::Resources) {
    use rtfm::Resource;
    r.COUNT.claim_mut(_t, |count, _t| {
        *count = 0;
    });

    // clear the pending interrupt flag
    r.EXTI.pr.write(|w| w.pr9().set_bit());
}


#[lang = "oom"]
#[no_mangle]
pub fn rust_oom() -> ! {
    panic!()
}
