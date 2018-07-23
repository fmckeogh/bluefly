#![cfg_attr(feature = "cargo-clippy", warn(clippy_pedantic))]
#![no_std]
#![no_main]

#[macro_use(block)]
extern crate nb;
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

use byteorder::{BigEndian, ByteOrder, LittleEndian};
use cc1101::*;
use embedded_hal::spi::{Mode, Phase, Polarity};
use hal::prelude::*;
use hal::serial::Serial;
use hal::spi::Spi;
use hal::stm32f103xx;
use pruefung::crc::Crc16;
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
    let mut gpio_b = dp.GPIOB.split(&mut rcc.apb2);
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
    radio
        .write_register(config::Register::PKTCTRL1, 0b_0000_1110)
        .unwrap();

    let tx = gpio_b.pb6.into_alternate_push_pull(&mut gpio_b.crl);
    let rx = gpio_b.pb7;
    let serial = Serial::usart1(
        dp.USART1,
        (tx, rx),
        &mut afio.mapr,
        230400.bps(),
        clocks,
        &mut rcc.apb2,
    );
    let (mut tx, mut _rx) = serial.split();

    let mut radio_payload: [u8; 12] = [0; 12];
    let mut addr: u8 = 0;
    let mut len: u8 = 0;
    loop {
        radio.set_radio_mode(RadioMode::Receive).unwrap();
        match radio.rx_bytes_available() {
            Ok(_) => {
                radio
                    .read_fifo(&mut addr, &mut len, &mut radio_payload)
                    .unwrap();

                let mut hasher: Crc16 = Default::default();
                hasher.write_u8(radio_payload[2]);
                let mut radio_hash: [u8; 2] = [0; 2];
                LittleEndian::write_u16(&mut radio_hash, hasher.finish() as u16);

                if radio_payload[3] == radio_hash[0] {
                    let mut duty_cycle_buf: [u8; 4] = [0; 4];
                    BigEndian::write_u32(&mut duty_cycle_buf, (radio_payload[2] as u32) * 1000);

                    let mut vesc_payload = [
                        0x05,
                        duty_cycle_buf[0],
                        duty_cycle_buf[1],
                        duty_cycle_buf[2],
                        duty_cycle_buf[3],
                    ];

                    let mut hasher: Crc16 = Default::default();
                    hasher.write(&vesc_payload);
                    let mut vesc_payload_hash: [u8; 2] = [0; 2];
                    BigEndian::write_u16(&mut vesc_payload_hash, hasher.finish() as u16);

                    block!(tx.write(0x02)).ok();
                    block!(tx.write(vesc_payload.len() as u8)).ok();
                    for byte in vesc_payload.iter() {
                        block!(tx.write(*byte)).ok();
                    }
                    block!(tx.write(vesc_payload_hash[0])).ok();
                    block!(tx.write(vesc_payload_hash[1])).ok();
                    block!(tx.write(0x03)).ok();

                    if radio_payload[2] > 50 {
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
