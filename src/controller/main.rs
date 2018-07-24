#![cfg_attr(feature = "cargo-clippy", warn(clippy_pedantic))]
#![no_std]
#![no_main]
#![feature(alloc)]
#![feature(lang_items)]

#[macro_use]
extern crate alloc;
extern crate alloc_cortex_m;
extern crate cortex_m;
#[macro_use]
extern crate cortex_m_rt as rt;
extern crate byteorder;
extern crate cc1101;
extern crate embedded_graphics;
extern crate embedded_hal;
extern crate panic_semihosting;
extern crate pruefung;
extern crate ssd1306;
extern crate stm32f103xx_hal as hal;

use alloc_cortex_m::CortexMHeap;
use byteorder::{ByteOrder, LittleEndian};
use cc1101::{config, Cc1101, Command};
use embedded_graphics::fonts::Font6x8;
use embedded_graphics::prelude::*;
use embedded_hal::spi::{Mode, Phase, Polarity};
use hal::delay::Delay;
use hal::prelude::*;
use hal::spi::Spi;
use hal::stm32f103xx;
use pruefung::crc::Crc16;
use pruefung::Hasher;
use rt::ExceptionFrame;
use ssd1306::prelude::*;
use ssd1306::Builder;

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

entry!(main);

fn main() -> ! {
    let start = rt::heap_start() as usize;
    let size = 1024; // in bytes
    unsafe { ALLOCATOR.init(start, size) }

    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = stm32f103xx::Peripherals::take().unwrap();

    // Enable ADC clocks
    dp.RCC.apb2enr.write(|w| w.adc1en().set_bit());
    // Power on ADC
    dp.ADC1.cr2.write(|w| w.adon().set_bit());

    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();

    let clocks = rcc.cfgr.freeze(&mut flash.acr);
    let mut delay = Delay::new(cp.SYST, clocks);

    let mut afio = dp.AFIO.constrain(&mut rcc.apb2);

    let mut gpio_a = dp.GPIOA.split(&mut rcc.apb2);
    let mut gpio_b = dp.GPIOB.split(&mut rcc.apb2);
    let mut gpio_c = dp.GPIOC.split(&mut rcc.apb2);

    let mut led = gpio_c.pc13.into_push_pull_output(&mut gpio_c.crh);

    // RADIO
    let sck1 = gpio_a.pa5.into_alternate_push_pull(&mut gpio_a.crl);
    let miso1 = gpio_a.pa6;
    let mosi1 = gpio_a.pa7.into_alternate_push_pull(&mut gpio_a.crl);
    let cs1 = gpio_a.pa4.into_push_pull_output(&mut gpio_a.crl);

    let spi1 = Spi::spi1(
        dp.SPI1,
        (sck1, miso1, mosi1),
        &mut afio.mapr,
        Mode {
            polarity: Polarity::IdleLow,
            phase: Phase::CaptureOnFirstTransition,
        },
        8.mhz(),
        clocks,
        &mut rcc.apb2,
    );

    let mut radio = Cc1101::new(spi1, cs1).unwrap();
    radio.preset_msk_500kb().unwrap();
    radio.set_frequency(433_000_000).unwrap();
    radio.set_power_level(10).unwrap();
    radio
        .write_register(config::Register::PKTCTRL1, 0b_0000_1110)
        .unwrap();
    radio.write_strobe(Command::SFRX).unwrap();

    // DISPLAY
    let sck2 = gpio_b.pb13.into_alternate_push_pull(&mut gpio_b.crh);
    let miso2 = gpio_b.pb14;
    let mosi2 = gpio_b.pb15.into_alternate_push_pull(&mut gpio_b.crh);
    let mut rst2 = gpio_a.pa9.into_push_pull_output(&mut gpio_a.crh);
    let dc2 = gpio_a.pa8.into_push_pull_output(&mut gpio_a.crh);

    let spi2 = Spi::spi2(
        dp.SPI2,
        (sck2, miso2, mosi2),
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
        .connect_spi(spi2, dc2)
        .into();

    display.reset(&mut rst2, &mut delay);
    display.init().unwrap();
    display.clear();
    display.flush().unwrap();

    loop {
        display.clear();

        // Perform single conversion
        dp.ADC1.cr2.write(|w| w.adon().set_bit());
        while dp.ADC1.sr.read().eoc().bit_is_clear() {}
        let sensor_val: u32 = dp.ADC1.dr.read().data().bits().into();
        let sensor_pcnt = ((sensor_val * 100) / 4096) as u8;

        if sensor_pcnt > 50 {
            led.set_low();
        } else {
            led.set_high();
        }

        let mut hasher: Crc16 = Default::default();
        hasher.write_u8(sensor_pcnt);
        let mut hash: [u8; 2] = [0; 2];
        LittleEndian::write_u16(&mut hash, hasher.finish() as u16);

        let mut packet = [0x01, 0x02, sensor_pcnt, hash[0]];
        radio.transmit(&mut packet).unwrap();

        display.draw(
            Font6x8::render_str(&format!("{:?}", hash))
                .translate((0, 24))
                .into_iter(),
        );

        display.draw(
            Font6x8::render_str(&format!("{}%", sensor_pcnt))
                .translate((0, 0))
                .into_iter(),
        );

        display.draw(
            Font6x8::render_str(&format!("RSSI: {}", radio.get_rssi_dbm().unwrap()))
                .translate((0, 12))
                .into_iter(),
        );

        display.flush().unwrap();
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

#[lang = "oom"]
#[no_mangle]
pub fn rust_oom(layout: core::alloc::Layout) -> ! {
    panic!("{:?}", layout);
}
