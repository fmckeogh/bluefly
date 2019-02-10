#![no_std]
#![no_main]
#![feature(alloc)]
#![feature(lang_items)]

#[macro_use]
extern crate alloc;
extern crate panic_semihosting;

use alloc_cortex_m::CortexMHeap;
use common::Request;
use cortex_m_semihosting::hprintln;
use embedded_graphics::{coord::Coord, fonts::Font6x8, prelude::*};
use embedded_hal::spi::{Mode, Phase, Polarity};
use nb::block;
use rtfm::app;
use ssd1306::{
    interface::spi::SpiInterface, mode::graphics::GraphicsMode, prelude::DisplaySize, Builder,
};
use stm32f103xx_hal::{
    gpio::{
        gpioa::PA9,
        gpiob::{PB13, PB14, PB15},
        gpioc::PC13,
        Alternate, Floating, Input, Output, PushPull,
    },
    prelude::*,
    serial::Serial,
    spi::Spi,
    stm32f103xx::{self as device, ADC1, SPI2, USART1},
};

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

const PERIOD: u32 = 500_000;

#[app(device = stm32f103xx_hal::stm32f103xx)]
const APP: () = {
    static mut LED: PC13<Output<PushPull>> = ();

    static mut ADC1: ADC1 = ();
    static mut RADIO: (
        stm32f103xx_hal::serial::Tx<USART1>,
        stm32f103xx_hal::serial::Rx<USART1>,
    ) = ();
    static mut DISPLAY: Display = ();

    #[init(spawn = [periodic])]
    fn init() {
        hprintln!("init").unwrap();

        // Allocator
        {
            let start = cortex_m_rt::heap_start() as usize;
            let size = 512; // in bytes
            unsafe { ALLOCATOR.init(start, size) }
        }

        let device: device::Peripherals = device;

        // Setup ADC1 on PA0
        let adc1 = {
            // Enable ADC clocks
            device.RCC.apb2enr.write(|w| w.adc1en().set_bit());
            // Power on ADC
            device.ADC1.cr2.write(|w| w.adon().set_bit());

            device.ADC1
        };

        let mut flash = device.FLASH.constrain();
        let mut rcc = device.RCC.constrain();

        // Try a different clock configuration
        let clocks = rcc.cfgr.freeze(&mut flash.acr);

        let mut afio = device.AFIO.constrain(&mut rcc.apb2);

        let mut gpioa = device.GPIOA.split(&mut rcc.apb2);
        let mut gpiob = device.GPIOB.split(&mut rcc.apb2);
        let mut gpioc = device.GPIOC.split(&mut rcc.apb2);

        // Radio being emulated by connecting USART1s of both controller and receiver together
        let radio = {
            let tx = gpiob.pb6.into_alternate_push_pull(&mut gpiob.crl);
            let rx = gpiob.pb7;

            let serial = Serial::usart1(
                device.USART1,
                (tx, rx),
                &mut afio.mapr,
                115_200.bps(),
                clocks,
                &mut rcc.apb2,
            );

            serial.split()
        };

        // SSD1306 connected to SPI2
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

        spawn.periodic().unwrap();

        hprintln!("init complete").unwrap();

        LED = led;
        ADC1 = adc1;
        RADIO = radio;
        DISPLAY = display;
    }

    #[task(schedule = [periodic], resources = [LED, DISPLAY, RADIO, ADC1])]
    fn periodic() {
        let request = {
            resources.ADC1.cr2.write(|w| w.adon().set_bit());
            while resources.ADC1.sr.read().eoc().bit_is_clear() {}
            let val = resources.ADC1.dr.read().data().bits();

            Request { val }
        };

        let mut buf = [0u8; 32];
        let len = ssmarshal::serialize(&mut buf, &request).unwrap();

        for i in 0..len {
            block!(resources.RADIO.0.write(buf[i])).unwrap();
        }
        block!(resources.RADIO.0.flush()).unwrap();

        resources.DISPLAY.draw(
            Font6x8::render_str(&format!("{:?}", request))
                .translate(Coord::new(0, 0))
                .into_iter(),
        );
        resources.DISPLAY.draw(
            Font6x8::render_str(&format!("{:?}", buf[..len].to_vec()))
                .translate(Coord::new(0, 12))
                .into_iter(),
        );
        resources.DISPLAY.flush().unwrap();

        schedule.periodic(scheduled + PERIOD.cycles()).unwrap();
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
