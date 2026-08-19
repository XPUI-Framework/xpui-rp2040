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

/// How much of the RP2040's 264 kB of SRAM `xpui` gets.
///
/// It has to hold the leaked backend — which on the Badger carries the panel's
/// own 4,736-byte framebuffer — the stack of live screens, and the view tree
/// `body()` rebuilds on every frame that carries input. The busiest gallery
/// screen is a few kilobytes of that, so 64 kB is around an order of magnitude
/// of headroom while still leaving some 190 kB for the stack, the executor and
/// statics.
///
/// Raise it before adding a screen that buffers an image. There is no PSRAM on
/// either board to fall back on, and a first-fit allocator handed too little
/// memory fragments long before it runs out.
const HEAP_SIZE: usize = 64 * 1024;

/// Opens the channel `probe-rs run` reads, so the board can be heard.
///
/// Before this, every line below is written into a ring buffer nobody is
/// holding and dropped, which is also what happens when the firmware is
/// running from flash with no probe attached — the cost of a line nobody reads
/// is a memcpy into a buffer that never fills.
///
/// Call it first, so a panic in `init_heap` still has somewhere to go.
pub fn init_log() {
    rtt_init_print!();
}

/// What the allocator has handed out and what is left, in bytes.
///
/// The only view either board has of its own memory. A first-fit allocator
/// with 64 kB fragments long before it runs out, so the interesting number is
/// how `free` moves frame to frame, not what it is once.
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
/// The message costs `core::fmt`, which the frame loop is kept clear of on
/// purpose — but this path runs once and never again, and a board that dies
/// without saying why is indistinguishable from one that is merely idle. That
/// ambiguity is expensive: a half-drawn panel and a silent park look exactly
/// like a layout bug from across a desk.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rprintln!("PANIC: {}", info);
    park()
}
