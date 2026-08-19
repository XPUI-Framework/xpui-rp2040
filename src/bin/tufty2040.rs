//! The gallery on a Pimoroni Tufty 2040.
//!
//! 320x240 of colour IPS behind an ST7789v, driven over an 8-bit parallel bus
//! rather than SPI. `xpui` is a 1-bit framework and stays one here: the backend
//! maps ink and background onto two `Rgb565` values, and the screens never
//! learn that the panel could do more.
//!
//! # Pins
//!
//! | | |
//! |---|---|
//! | Buttons | A GP7, B GP8, C GP9, up GP22, down GP6 |
//! | Panel | chip select GP10, data/command GP11, write GP12, read GP13 |
//! | Data bus | DB0–DB7 on GP14–GP21, in order |
//! | Board | backlight GP2, LED GP25 |
//!
//! GP27 is the battery-sense reference enable, not a panel supply — Pimoroni's
//! own examples raise it only while reading the ADC — so it is left alone.

#![cfg_attr(device, no_std)]
#![cfg_attr(device, no_main)]

#[cfg(device)]
use {
    embassy_executor::Spawner,
    embassy_rp::gpio::{Input, Level, Output, Pull},
    embassy_time::Delay,
    embedded_graphics::draw_target::DrawTarget,
    embedded_graphics::pixelcolor::{Rgb565, RgbColor},
    gallery::Menu,
    mipidsi::Builder,
    mipidsi::interface::{Generic8BitBus, ParallelInterface},
    mipidsi::models::ST7789,
    mipidsi::options::{ColorInversion, ColorOrder, Orientation, Rotation},
    xpui_boards::Board,
    xpui_eg::Palette,
    xpui_rp2040::{ButtonPins, Buttons, PacedFill, init_heap, init_log, run},
};

#[cfg(device)]
#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // Before the heap, so a panic inside `init_heap` still has somewhere to go.
    init_log();
    // Second, because `App::new` allocates on its first line.
    init_heap();

    let p = embassy_rp::init(Default::default());

    let buttons = Buttons::new(ButtonPins {
        a: Input::new(p.PIN_7, Pull::Down),
        b: Input::new(p.PIN_8, Pull::Down),
        c: Input::new(p.PIN_9, Pull::Down),
        up: Input::new(p.PIN_22, Pull::Down),
        down: Input::new(p.PIN_6, Pull::Down),
    });

    // Selected once and held, rather than per transfer: the panel is the only
    // device on this bus.
    let _chip_select = Output::new(p.PIN_10, Level::Low);
    // The read strobe shares the data pins with the write strobe. Nothing here
    // ever reads the controller back, so it is parked high for good.
    let _read = Output::new(p.PIN_13, Level::High);

    // DB0 first: `Generic8BitBus` takes the least significant bit at the head
    // of the tuple.
    let bus = Generic8BitBus::new((
        Output::new(p.PIN_14, Level::Low),
        Output::new(p.PIN_15, Level::Low),
        Output::new(p.PIN_16, Level::Low),
        Output::new(p.PIN_17, Level::Low),
        Output::new(p.PIN_18, Level::Low),
        Output::new(p.PIN_19, Level::Low),
        Output::new(p.PIN_20, Level::Low),
        Output::new(p.PIN_21, Level::Low),
    ));
    let interface = ParallelInterface::new(
        bus,
        Output::new(p.PIN_11, Level::Low),  // data/command
        Output::new(p.PIN_12, Level::High), // write strobe, latches on the rise
    );

    // The ST7789's framebuffer is 240x320 portrait and the Tufty's glass is
    // mounted a quarter turn from it, so the panel is described in its own
    // orientation and then rotated. `Display::size` reports 320x240 after that,
    // which is what `Board::TUFTY_2040` says and what the chrome is sized for.
    //
    // Inverted colour is the panel's polarity, not a stylistic choice: without
    // it black and white come out the wrong way round.
    //
    // Destructured rather than unwrapped: every pin on this interface is
    // infallible, which leaves `InitError` with no inhabited variant, so the
    // pattern is irrefutable. Should a future mipidsi give it one, this stops
    // compiling instead of silently swallowing a failure.
    let Ok(display) = Builder::new(ST7789, interface)
        .display_size(240, 320)
        .orientation(Orientation::new().rotate(Rotation::Deg270))
        .color_order(ColorOrder::Rgb)
        .invert_colors(ColorInversion::Inverted)
        .init(&mut Delay);

    // Wrapped before anything is drawn through it. `mipidsi` shortens a run of
    // one colour into a bare strobe loop that outruns this controller, and on
    // this palette that is every fill and every clear — see [`PacedFill`].
    let mut display = PacedFill::new(display);

    // Cleared before the backlight comes on. The controller's RAM holds
    // whatever survived reset, and lighting that shows a frame of noise.
    let _ = display.clear(Rgb565::WHITE);
    let _backlight = Output::new(p.PIN_2, Level::High);

    run(
        display,
        Board::TUFTY_2040,
        // Black on white. The framework paints in ink and background and has no
        // way to ask for a third colour, so running it on colour hardware is a
        // matter of choosing which two — this pair reads like the e-ink panel
        // the same screens run on.
        Palette::new(Rgb565::BLACK, Rgb565::WHITE),
        buttons,
        Menu::new(),
        // The parallel interface writes straight to the controller, so drawing
        // has already reached the glass. There is nothing left to push.
        |_display: &mut _| {},
    )
    .await
}

/// Off the device this file is an empty binary; see the crate documentation.
#[cfg(not(device))]
fn main() {}
