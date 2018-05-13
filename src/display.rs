use hal::gpio::gpioa::{PA5, PA6, PA7};
use hal::gpio::gpiob::PB1;
use hal::gpio::{Alternate, Floating, Input, Output, PushPull};
use hal::spi::Spi;
use hal::stm32f103xx::SPI1;
use embedded_graphics::fonts::{Font, Font6x8};
use embedded_graphics::*;
use embedded_graphics::prelude::Transform;
use ssd1306::interface::SpiInterface;
use ssd1306::mode::GraphicsMode;

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

pub struct DisplayData {
        // Power
        pub local_bat: u8, // %
        pub remote_bat: u8, // %
        pub current: f32, // amperes
        pub mode: u8, // beginner, eco etc

        // Distance
        pub speed: f32, // m/s
        pub distance_travelled: u32, // metres
        pub distance_remaining: u32, // metres

        // Signal
        pub signal_strength: u8, // %
        pub packet_loss: u8, // %
}

pub fn write_display(disp: &mut OledDisplay, data: DisplayData) {
    disp.draw(
        Font6x8::render_str(&format!("{}%", data.packet_loss))
            .translate((0, 0))
            .into_iter(),
    );

    disp.draw(
        Font6x8::render_str("TEST")
            .translate((0, 20))
            .into_iter(),
    );

}