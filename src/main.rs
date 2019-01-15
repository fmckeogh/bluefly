#![no_main]
#![no_std]
#![feature(alloc)]
#![feature(lang_items)]

#[macro_use]
extern crate alloc;
extern crate panic_halt;

use alloc_cortex_m::CortexMHeap;
use cortex_m_semihosting::hprintln;
use embedded_graphics::{fonts::Font6x8, prelude::*};
use embedded_hal::spi::{Mode, Phase, Polarity};
use rtfm::app;
use ssd1306::{prelude::*, Builder};
use stm32f103xx_hal::{
    gpio::{
        gpioa::PA8,
        gpiob::{PB13, PB14, PB15},
        gpioc::PC13,
        Alternate, Floating, Input, Output, PushPull,
    },
    prelude::*,
    spi::Spi,
    stm32f103xx::{self, EXTI, SPI2},
};

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

const HEARTBEAT_PERIOD: u32 = 8_000_000;
const DISPLAY_PERIOD: u32 = 4_000_000;

#[app(device = stm32f103xx_hal::stm32f103xx)]
const APP: () = {
    static mut ON: bool = false;
    static mut LED: PC13<Output<PushPull>> = ();
    static mut EXTI: EXTI = ();
    static mut DISPLAY: ssd1306::mode::graphics::GraphicsMode<
        SpiInterface<
            Spi<
                SPI2,
                (
                    PB13<Alternate<PushPull>>,
                    PB14<Input<Floating>>,
                    PB15<Alternate<PushPull>>,
                ),
            >,
            PA8<Output<PushPull>>,
        >,
    > = ();
    static mut COUNT: u64 = 0u64;

    #[init(spawn = [heartbeat, display])]
    fn init() {
        hprintln!("init...").unwrap();

        let start = cortex_m_rt::heap_start() as usize;
        let size = 1024; // in bytes
        unsafe { ALLOCATOR.init(start, size) }

        let _core: rtfm::Peripherals = core;
        let device: stm32f103xx::Peripherals = device;

        let mut rcc = device.RCC.constrain();
        let mut flash = device.FLASH.constrain();
        let mut afio = device.AFIO.constrain(&mut rcc.apb2);
        let clocks = rcc.cfgr.freeze(&mut flash.acr);

        let mut gpio_a = device.GPIOA.split(&mut rcc.apb2);
        let mut gpio_b = device.GPIOB.split(&mut rcc.apb2);
        let mut gpio_c = device.GPIOC.split(&mut rcc.apb2);

        // LED
        let led = gpio_c.pc13.into_push_pull_output(&mut gpio_c.crh);

        // DISPLAY
        let display = {
            let spi = {
                let sck = gpio_b.pb13.into_alternate_push_pull(&mut gpio_b.crh);
                let miso = gpio_b.pb14;
                let mosi = gpio_b.pb15.into_alternate_push_pull(&mut gpio_b.crh);

                Spi::spi2(
                    device.SPI2,
                    (sck, miso, mosi),
                    Mode {
                        polarity: Polarity::IdleLow,
                        phase: Phase::CaptureOnFirstTransition,
                    },
                    8.mhz(),
                    clocks,
                    &mut rcc.apb1,
                )
            };

            let dc = gpio_a.pa8.into_push_pull_output(&mut gpio_a.crh);

            let mut display: GraphicsMode<_> = Builder::new()
                .with_size(DisplaySize::Display128x64)
                .connect_spi(spi, dc)
                .into();

            let mut rst = gpio_a.pa9.into_push_pull_output(&mut gpio_a.crh);
            rst.set_low();
            rst.set_high();

            display.init().unwrap();
            display.clear();
            display.flush().unwrap();

            display.draw(
                Font6x8::render_str("test")
                    .translate(Coord::new(0, 0))
                    .into_iter(),
            );
            display.flush().unwrap();

            display
        };

        // Setup external interrupts
        gpio_b.pb11.into_floating_input(&mut gpio_b.crh);
        let rcc = unsafe { &*stm32f103xx_hal::stm32f103xx::RCC::ptr() };
        rcc.apb2enr.modify(|_, w| w.afioen().enabled());
        afio.exticr3
            .exticr3()
            .modify(|_, w| unsafe { w.exti11().bits(0x01) });

        // Enable interrupt on EXTI11
        device.EXTI.imr.modify(|_, w| w.mr11().set_bit());
        // Set rising trigger selection for EXTI11
        device.EXTI.ftsr.modify(|_, w| w.tr11().set_bit());

        // Start tasks
        spawn.heartbeat().unwrap();
        spawn.display().unwrap();

        hprintln!("init complete").unwrap();

        LED = led;
        EXTI = device.EXTI;
        DISPLAY = display;
    }

    #[idle]
    fn idle() -> ! {
        hprintln!("idle").unwrap();
        loop {}
    }

    #[task(priority = 4, schedule = [heartbeat], resources = [ON, LED])]
    fn heartbeat() {
        match *resources.ON {
            true => resources.LED.set_low(),
            false => resources.LED.set_high(),
        }
        *resources.ON ^= true;

        schedule
            .heartbeat(scheduled + HEARTBEAT_PERIOD.cycles())
            .unwrap();
    }

    #[task(priority = 3, schedule = [display], resources = [DISPLAY, COUNT, ON])]
    fn display() {
        let mut count_local = 0;
        let mut on_local = false;

        resources.COUNT.lock(|count| {
            count_local = *count;
        });
        resources.ON.lock(|on| {
            on_local = *on;
        });

        resources.DISPLAY.clear();
        resources.DISPLAY.draw(
            Font6x8::render_str(&format!("count: {}", count_local))
                .translate(Coord::new(0, 0))
                .into_iter(),
        );
        match on_local {
            true => {
                resources.DISPLAY.draw(
                    Font6x8::render_str("on: true")
                        .translate(Coord::new(0, 12))
                        .into_iter(),
                );
            }
            false => {
                resources.DISPLAY.draw(
                    Font6x8::render_str("on: false")
                        .translate(Coord::new(0, 12))
                        .into_iter(),
                );
            }
        }
        resources.DISPLAY.flush().unwrap();

        schedule
            .display(scheduled + DISPLAY_PERIOD.cycles())
            .unwrap();
    }

    #[interrupt(priority = 5, resources = [COUNT, EXTI])]
    fn EXTI15_10() {
        *resources.COUNT += 1;
        // clear flag?
        resources.EXTI.pr.modify(|_, w| w.pr11().set_bit());
    }

    extern "C" {
        fn USART1();
        fn USART2();
        fn USART3();
    }
};

#[lang = "oom"]
#[no_mangle]
pub fn rust_oom(layout: core::alloc::Layout) -> ! {
    panic!("{:?}", layout);
}
