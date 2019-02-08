#![no_std]
#![no_main]
#![allow(unused)]

extern crate panic_semihosting;

use cortex_m_semihosting::{hprint, hprintln};
use embedded_hal::spi::{Mode, Phase, Polarity};
use nb::block;
use nrf24l01::NRF24L01;
use rtfm::app;
use stm32f103xx_hal::{
    gpio::{
        gpioa::{PA4, PA5, PA6, PA7},
        gpiob::{PB1, PB10, PB13, PB14, PB15},
        gpioc::PC13,
        Alternate, Floating, Input, Output, PushPull,
    },
    prelude::*,
    serial::Serial,
    spi::Spi,
    stm32f103xx::{self as device, SPI1},
};

const PERIOD: u32 = 8_000_000;

#[app(device = stm32f103xx_hal::stm32f103xx)]
const APP: () = {
    static mut ON: bool = false;
    static mut LED: PC13<Output<PushPull>> = ();
    static mut RADIO: stm32f103xx_hal::serial::Rx<device::USART1> = ();

    #[init(spawn = [blinky])]
    fn init() {
        hprintln!("init").unwrap();

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
            let tx = gpiob.pb6.into_alternate_push_pull(&mut gpiob.crl);
            let rx = gpiob.pb7;

            let serial = Serial::usart1(
                device.USART1,
                (tx, rx),
                &mut afio.mapr,
                19_200.bps(),
                clocks,
                &mut rcc.apb2,
            );

            let (mut tx, mut rx) = serial.split();

            for byte in b"ER_CMD#U?" {
                block!(tx.write(*byte)).unwrap();
                block!(tx.flush()).unwrap();
            }

            for _ in 0..100 {
                hprint!("{:?}", block!(rx.read()).unwrap());
            }

            rx
        };

        let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);

        spawn.blinky().unwrap();

        LED = led;
        RADIO = radio;
    }

    #[idle(resources = [RADIO])]
    fn idle() -> ! {
        hprintln!("idle").unwrap();
        let mut buf = [0u8; 32];

        loop {
            hprint!("{:?}", block!(resources.RADIO.read()).unwrap());
        }
    }

    #[task(schedule = [blinky], resources = [ON, LED])]
    fn blinky() {
        match *resources.ON {
            true => resources.LED.set_high(),
            false => resources.LED.set_low(),
        }
        *resources.ON ^= true;

        schedule.blinky(scheduled + PERIOD.cycles()).unwrap();
    }

    extern "C" {
        fn TAMPER();
    }
};
