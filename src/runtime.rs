//! A heap, somewhere for a panic to go, a way to say why, and a way to stop.
//!
//! Nothing a hosted program would have to think about, and all of it required
//! before the first line of UI code runs.

use core::mem::MaybeUninit;
use core::panic::PanicInfo;

use embedded_alloc::LlffHeap;
use rtt_target::{rprintln, rtt_init_print};

#[global_allocator]
static HEAP: LlffHeap = LlffHeap::empty();

/// How much of the RP2040's 256 kB striped SRAM `xpui` gets.
///
/// It holds the leaked backend — on the Badger, the panel's own 4,736-byte
/// framebuffer — the stack of live screens, and the view tree `body()`
/// rebuilds on every frame carrying input; the busiest gallery screen is a
/// few kilobytes of that. Raise it before adding a screen that buffers an
/// image: there is no PSRAM, and a first-fit allocator handed too little
/// memory fragments long before it runs out.
const HEAP_SIZE: usize = 64 * 1024;

/// Opens the channel `probe-rs run` reads, so the board can be heard.
///
/// Call it first, so a panic in `init_heap` still has somewhere to go. With
/// no probe attached, a line costs a memcpy into a ring buffer that never
/// fills.
pub fn init_log() {
    rtt_init_print!();
}

/// What the allocator has handed out and what is left, in bytes: the only
/// view either board has of its own memory.
pub fn heap_used() -> (usize, usize) {
    (HEAP.used(), HEAP.free())
}

/// Hands the allocator its memory. Call once, before anything allocates.
///
/// [`xpui::App::new`] allocates on its first line, so this is the first line
/// of every binary here.
pub fn init_heap() {
    static mut HEAP_MEMORY: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
    // Safety: `HEAP_MEMORY` is a static of exactly `HEAP_SIZE` bytes, and this
    // raw pointer is the only thing ever taken from it — no reference to it
    // exists to alias what the allocator hands out. A second caller would trip
    // `Heap::init`'s own assertion rather than corrupt anything.
    unsafe { HEAP.init(&raw mut HEAP_MEMORY as usize, HEAP_SIZE) }
}

/// Stops, keeping whatever the panel last showed.
///
/// There is no console on either board, and e-ink holds its image with the
/// power off, so the most useful thing a firmware with nowhere to go can do is
/// leave the last frame up rather than clear it.
pub fn park() -> ! {
    loop {
        cortex_m::asm::wfi();
    }
}

/// Says what happened, then stops.
///
/// The message costs `core::fmt`, which the frame loop is kept clear of — but
/// this path runs once, and a board that dies without saying why is
/// indistinguishable from one that is merely idle.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rprintln!("PANIC: {}", info);
    park()
}
