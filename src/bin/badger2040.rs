//! The gallery on a Pimoroni Badger 2040.
//!
//! 296x128 of monochrome e-ink behind a UC8151, five buttons along the bottom
//! edge, 2 MB of flash and no touchscreen. A full refresh is close to a
//! second, which is why the loop paints only what changed and why `LUT::Fast`
//! is worth the ghosting it leaves behind.
//!
//! # Pins
//!
//! | | |
//! |---|---|
//! | Buttons | A GP12, B GP13, C GP14, up GP15, down GP11 |
//! | SPI0 | clock GP18, data GP19 — the panel is write-only, so MISO is unused |
//! | Panel | chip select GP17, data/command GP20, reset GP21, busy GP26 |
//! | Board | 3V3 enable GP10 |

#![no_std]
#![no_main]

use xpui_boards_pimoroni as pimoroni;

use {
    embassy_executor::Spawner,
    embassy_rp::gpio::{Input, Level, Output, Pull},
    embassy_rp::spi::{Config as SpiConfig, Spi},
    embassy_time::Delay,
    gallery::Menu,
    uc8151::{LUT, Uc8151},
    xpui_eg::Palette,
    xpui_rp2040::{ButtonPins, init_heap, init_log, run},
};

/// What Pimoroni's own driver clocks this panel at. It moves the 4,736-byte
/// framebuffer in about 3 ms, which is nothing beside the refresh that follows
/// — the bus is not the thing to tune here.
const SPI_FREQUENCY: u32 = 12_000_000;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // Before the heap, so a panic inside `init_heap` still has somewhere to go.
    init_log();
    // Second, because `App::new` allocates on its first line.
    init_heap();

    let p = embassy_rp::init(Default::default());

    // Held high for as long as the firmware runs. On USB the 3V3 rail is fed
    // anyway, but on battery this pin *is* the rail: Pimoroni's driver raises
    // it at boot and drops it to power the badge down.
    let _power = Output::new(p.PIN_10, Level::High);

    let pins = ButtonPins {
        a: Input::new(p.PIN_12, Pull::Down),
        b: Input::new(p.PIN_13, Pull::Down),
        c: Input::new(p.PIN_14, Pull::Down),
        up: Input::new(p.PIN_15, Pull::Down),
        down: Input::new(p.PIN_11, Pull::Down),
    };

    let mut spi_config = SpiConfig::default();
    spi_config.frequency = SPI_FREQUENCY;
    let spi = Spi::new_blocking_txonly(p.SPI0, p.PIN_18, p.PIN_19, spi_config);

    let mut display = Uc8151::new(
        spi,
        Output::new(p.PIN_17, Level::High), // chip select, idle high
        Output::new(p.PIN_20, Level::Low),  // data/command
        Input::new(p.PIN_26, Pull::Up),     // busy, asserted low
        Output::new(p.PIN_21, Level::Low),  // reset, driven by the driver
    );

    // `Fast` is the quickest waveform that still leaves text crisp: roughly
    // 0.8 s against `Normal`'s 4.5. A failure here is an SPI fault with nothing
    // to fall back on, and a blank panel says as much as a panic would.
    let _ = display.setup(&mut Delay, LUT::Fast);

    run(
        display,
        pimoroni::BADGER_2040,
        // `Off` is black. The driver inverts deliberately, so that a 1-bit
        // bitmap loads the way it was drawn — which means ink and background
        // follow the driver here, not `embedded_graphics`' usual reading of
        // `On` as the lit pixel.
        Palette::INK_IS_OFF,
        pins,
        Menu::new(),
        |display: &mut _| {
            // Blocks until the panel has finished, around a second. Nothing
            // else shares this executor, and a frame started mid-refresh would
            // be waited out here anyway.
            let _ = display.update();
        },
    )
    .await
}
