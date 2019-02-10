#![no_std]
#![no_main]

extern crate panic_semihosting;

use common::Request;
use cortex_m::singleton;
use cortex_m_semihosting::hprintln;
use nb::block;
use rtfm::app;
use stm32f103xx_hal::{
    gpio::{gpioc::PC13, Output, PushPull},
    prelude::*,
    serial::Serial,
    stm32f103xx::{self as device, USART1},
};

const PERIOD: u32 = 8_000_000;

#[app(device = stm32f103xx_hal::stm32f103xx)]
const APP: () = {
    static mut ON: bool = false;
    static mut LED: PC13<Output<PushPull>> = ();
    static mut C5: stm32f103xx_hal::dma::dma1::C5 = ();
    static mut RADIO: (
        stm32f103xx_hal::serial::Tx<USART1>,
        stm32f103xx_hal::serial::Rx<USART1>,
    ) = ();

    #[init(spawn = [blinky])]
    fn init() {
        hprintln!("init").unwrap();

        let device: device::Peripherals = device;

        let mut flash = device.FLASH.constrain();
        let mut rcc = device.RCC.constrain();

        // Try a different clock configuration
        let clocks = rcc.cfgr.freeze(&mut flash.acr);

        let mut afio = device.AFIO.constrain(&mut rcc.apb2);
        let channels = device.DMA1.split(&mut rcc.ahb);
        let c5 = channels.5;

        let _gpioa = device.GPIOA.split(&mut rcc.apb2);
        let mut gpiob = device.GPIOB.split(&mut rcc.apb2);
        let mut gpioc = device.GPIOC.split(&mut rcc.apb2);

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

        let led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);

        spawn.blinky().unwrap();

        LED = led;
        C5 = c5;
        RADIO = radio;
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
