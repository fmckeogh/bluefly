#![no_std]
#![no_main]
#![feature(alloc)]
#![feature(lang_items)]

#[macro_use]
extern crate alloc;
extern crate panic_semihosting;

use alloc_cortex_m::CortexMHeap;
use cortex_m_semihosting::hprintln;
use embedded_graphics::fonts::Font6x8;
use embedded_graphics::prelude::*;
use embedded_hal::spi::{Mode, Phase, Polarity};
use nrf24l01::NRF24L01;
use rtfm::app;
use ssd1306::{
    interface::spi::SpiInterface, mode::graphics::GraphicsMode, prelude::DisplaySize, Builder,
};
use stm32f103xx_hal::{
    gpio::{
        gpioa::{PA4, PA5, PA6, PA7, PA9},
        gpiob::{PB1, PB10, PB13, PB14, PB15},
        gpioc::PC13,
        Alternate, Floating, Input, Output, PushPull,
    },
    prelude::*,
    spi::Spi,
    stm32f103xx::{self as device, SPI1, SPI2},
};

type Radio = nrf24l01::NRF24L01<
    stm32f103xx_hal::spi::Spi<
        SPI1,
        (
            stm32f103xx_hal::gpio::gpioa::PA5<
                stm32f103xx_hal::gpio::Alternate<stm32f103xx_hal::gpio::PushPull>,
            >,
            stm32f103xx_hal::gpio::gpioa::PA6<
                stm32f103xx_hal::gpio::Input<stm32f103xx_hal::gpio::Floating>,
            >,
            stm32f103xx_hal::gpio::gpioa::PA7<
                stm32f103xx_hal::gpio::Alternate<stm32f103xx_hal::gpio::PushPull>,
            >,
        ),
    >,
    stm32f103xx_hal::gpio::gpiob::PB10<
        stm32f103xx_hal::gpio::Output<stm32f103xx_hal::gpio::PushPull>,
    >,
    stm32f103xx_hal::gpio::gpiob::PB1<
        stm32f103xx_hal::gpio::Output<stm32f103xx_hal::gpio::PushPull>,
    >,
>;

type Display = GraphicsMode<
    SpiInterface<
        Spi<
            SPI2,
            (
                PB13<Alternate<PushPull>>,
                PB14<Input<Floating>>,
                PB15<Alternate<PushPull>>,
            ),
        >,
        PA9<Output<PushPull>>,
    >,
>;

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

const PERIOD: u32 = 8_000_000;

#[app(device = stm32f103xx_hal::stm32f103xx)]
const APP: () = {
    static mut ON: bool = false;
    static mut LED: PC13<Output<PushPull>> = ();

    static mut RADIO: Radio = ();
    static mut DISPLAY: Display = ();

    #[init(spawn = [blinky])]
    fn init() {
        hprintln!("init").unwrap();

        // Allocator
        {
            let start = cortex_m_rt::heap_start() as usize;
            let size = 512; // in bytes
            unsafe { ALLOCATOR.init(start, size) }
        }

        let device: device::Peripherals = device;

        let mut flash = device.FLASH.constrain();
        let mut rcc = device.RCC.constrain();

        // Try a different clock configuration
        let clocks = rcc.cfgr.freeze(&mut flash.acr);

        let mut afio = device.AFIO.constrain(&mut rcc.apb2);

        let mut gpioa = device.GPIOA.split(&mut rcc.apb2);
        let mut gpiob = device.GPIOB.split(&mut rcc.apb2);
        let mut gpioc = device.GPIOC.split(&mut rcc.apb2);

        let radio = {
            let sck = gpioa.pa5.into_alternate_push_pull(&mut gpioa.crl);
            let miso = gpioa.pa6;
            let mosi = gpioa.pa7.into_alternate_push_pull(&mut gpioa.crl);

            let spi = Spi::spi1(
                device.SPI1,
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

            let ce = gpiob.pb1.into_push_pull_output(&mut gpiob.crl);
            let mut csn = gpiob.pb10.into_push_pull_output(&mut gpiob.crh);
            csn.set_high();

            NRF24L01::new(spi, csn, ce, 1, 4).unwrap()
        };

        let display = {
            let sck = gpiob.pb13.into_alternate_push_pull(&mut gpiob.crh);
            let miso = gpiob.pb14;
            let mosi = gpiob.pb15.into_alternate_push_pull(&mut gpiob.crh);
            let mut rst = gpioa.pa8.into_push_pull_output(&mut gpioa.crh);
            let dc = gpioa.pa9.into_push_pull_output(&mut gpioa.crh);

            let spi = Spi::spi2(
                device.SPI2,
                (sck, miso, mosi),
                Mode {
                    polarity: Polarity::IdleLow,
                    phase: Phase::CaptureOnFirstTransition,
                },
                8.mhz(),
                clocks,
                &mut rcc.apb1,
            );

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

        let led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);

        spawn.blinky().unwrap();

        hprintln!("init complete").unwrap();

        LED = led;
        RADIO = radio;
        DISPLAY = display;
    }

    #[task(schedule = [blinky], resources = [ON, LED, DISPLAY, RADIO])]
    fn blinky() {
        match *resources.ON {
            true => resources.LED.set_high(),
            false => resources.LED.set_low(),
        }
        *resources.ON ^= true;

        resources.DISPLAY.clear();
        resources
            .DISPLAY
            .draw(Font6x8::render_str(&format!("state: {:?}", resources.ON)).into_iter());
        resources.DISPLAY.flush().unwrap();

        schedule.blinky(scheduled + PERIOD.cycles()).unwrap();
    }

    extern "C" {
        fn TAMPER();
    }
};

#[lang = "oom"]
#[no_mangle]
pub fn rust_oom(layout: core::alloc::Layout) -> ! {
    panic!("{:?}", layout);
}
