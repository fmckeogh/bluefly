#![cfg_attr(feature = "cargo-clippy", warn(clippy_pedantic))]
#![no_std]
#![no_main]

extern crate cortex_m;
#[macro_use]
extern crate cortex_m_rt as rt;
extern crate cc1101;
extern crate embedded_hal;
extern crate panic_semihosting;
extern crate stm32f103xx_hal as hal;
//extern crate cortex_m_semihosting as sh;
extern crate byteorder;
extern crate pruefung;

use byteorder::{ByteOrder, LittleEndian};
use cc1101::*;
use embedded_hal::spi::{Mode, Phase, Polarity};
use hal::prelude::*;
use hal::spi::Spi;
use hal::stm32f103xx;
use pruefung::crc::Crc32;
use pruefung::Hasher;
use rt::ExceptionFrame;

//use core::fmt::Write;
//use sh::hio;

entry!(main);

fn main() -> ! {
    //let mut hstdout = hio::hstdout().unwrap();

    let _cp = cortex_m::Peripherals::take().unwrap();
    let dp = stm32f103xx::Peripherals::take().unwrap();

    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();
    let mut afio = dp.AFIO.constrain(&mut rcc.apb2);
    let mut gpio_a = dp.GPIOA.split(&mut rcc.apb2);
    let mut gpio_c = dp.GPIOC.split(&mut rcc.apb2);

    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    let cs = gpio_a.pa4.into_push_pull_output(&mut gpio_a.crl);
    let sck = gpio_a.pa5.into_alternate_push_pull(&mut gpio_a.crl);
    let miso = gpio_a.pa6;
    let mosi = gpio_a.pa7.into_alternate_push_pull(&mut gpio_a.crl);

    let mut led = gpio_c.pc13.into_push_pull_output(&mut gpio_c.crh);
    led.set_high();

    let spi = Spi::spi1(
        dp.SPI1,
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

    let mut radio = Cc1101::new(spi, cs).unwrap();
    radio.preset_msk_500kb().unwrap();
    radio.set_frequency(915_000_000).unwrap();
    radio.set_power_level(10).unwrap();
    radio.write_strobe(Command::SFRX).unwrap();
    radio.write_register(config::Register::PKTCTRL1, 0b_0000_1110).unwrap();

    let mut payload: [u8; 12] = [0; 12];
    let mut addr: u8 = 0;
    let mut len: u8 = 0;
    loop {
        radio.set_radio_mode(RadioMode::Receive).unwrap();
        match radio.rx_bytes_available() {
            Ok(_) => {
                radio.read_fifo(&mut addr, &mut len, &mut payload).unwrap();

                let mut hasher: Crc32 = Default::default();
                hasher.write_u8(payload[2]);
                let mut hash: [u8; 4] = [0; 4];
                LittleEndian::write_u32(&mut hash, hasher.finish() as u32);

                if payload[3] == hash[0] {
                    if payload[2] > 50 {
                        led.set_low();
                    } else {
                        led.set_high();
                    }
                }
                //writeln!(hstdout, "calculated_hash: {:?}", hash).unwrap();
                //writeln!(hstdout, "payload: {:?}", payload).unwrap();

                radio.write_strobe(Command::SFRX).unwrap();
            }
            _ => {}
        }
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
