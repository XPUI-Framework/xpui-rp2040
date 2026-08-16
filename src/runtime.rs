//! A heap, somewhere for a panic to go, and a way to stop.
//!
//! Nothing a hosted program would have to think about, and all of it required
//! before the first line of UI code runs.

use core::mem::MaybeUninit;
use core::panic::PanicInfo;

use embedded_alloc::LlffHeap;

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

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    park()
}
