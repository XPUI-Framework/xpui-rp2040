//! The frame loop, written once for both boards.
//!
//! The same shape as the desktop simulator's — begin a frame, feed it input,
//! tick, and paint only if the framework asked — with embassy where SDL was.
//! Read `crates/backend/simulator/src/lib.rs` beside this: what differs is
//! where the input comes from and what a paint costs.

use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::draw_target::DrawTarget;
use rtt_target::rprintln;
use xpui::App;
use xpui::screen::Screen;
use xpui_boards::Board;
use xpui_eg::{Backend, Palette};

use crate::buttons::{ButtonPins, Buttons};
use crate::runtime::park;

/// How often the loop wakes.
///
/// Well under what either panel can show and well over what a thumb can do,
/// and it sets the debounce window: four agreeing samples at this interval is
/// 40 ms.
const FRAME_INTERVAL: Duration = Duration::from_millis(10);

/// Installs `display` as the host and runs `root` on it.
///
/// `present` is what pushes the framebuffer to the panel, and it is called
/// **only** when [`App::render_if_dirty`] painted something. On the Badger
/// that call is close to a second of the display refreshing, so a loop that
/// presented every frame would leave the panel permanently mid-update and the
/// buttons permanently unread. A display that draws straight through — the
/// Tufty's, over its parallel bus — passes a closure that does nothing,
/// because by then the pixels have already reached the glass.
///
/// **Both boards use this one**, because neither has an async flush to wait
/// on: `mipidsi` writes straight through, and `uc8151`'s published release
/// spins on the BUSY pin. Its `asynch` module exists on git and has never been
/// published — the last release was 2023 — so moving the Badger to it would
/// mean a git dependency and an `embedded-hal` 1.0 migration for a driver
/// nobody has cut a release of since. [`run_async`] is there for when that
/// changes, and for any DMA-backed panel today.
pub async fn run<D, S>(
    display: D,
    board: Board,
    palette: Palette<D::Color>,
    pins: ButtonPins,
    root: S,
    mut present: impl FnMut(&mut D),
) -> !
where
    D: DrawTarget + Send + 'static,
    D::Color: Sync,
    S: Screen + 'static,
{
    // One loop, not two. The blocking present is an async one that never
    // suspends.
    //
    // Not free: this change is +736 bytes on the Badger and +800 on the Tufty,
    // measured against the commit before it. Most of that is the loan
    // machinery rather than this wrapper — the two were not isolated — and on
    // a 2 MB flash it buys one loop instead of two that would drift.
    run_async(
        display,
        board,
        palette,
        pins,
        root,
        async |display: &mut D| present(display),
    )
    .await
}

/// The same loop, for a panel whose flush suspends.
///
/// `present` is handed the display **itself**, not a borrow of the backend:
/// the display leaves for the duration of the flush and the loan puts it back
/// afterwards. That is the whole difference, and it is why an `await` is
/// possible here at all — see [`Backend::loan_display`]. Anything that paints
/// while the flush is in flight is discarded.
pub async fn run_async<D, S>(
    display: D,
    board: Board,
    palette: Palette<D::Color>,
    pins: ButtonPins,
    root: S,
    mut present: impl AsyncFnMut(&mut D),
) -> !
where
    D: DrawTarget + Send + 'static,
    // The backend is installed as a `Host`, which is `Sync`. With the
    // `critical-section` feature that is worked out by the compiler rather
    // than claimed by an `unsafe impl`, and the compiler then wants this: the
    // palette is shared, and `PixelColor` does not require `Sync` even though
    // every implementor is a plain `Copy` value. An honest bound, and one no
    // real colour type fails.
    D::Color: Sync,
    S: Screen + 'static,
{
    // The board supplies the chrome and the type; the **panel** supplies the
    // size, because `Backend::new` measures whatever it is handed. Those two
    // agreeing is what makes "develop in a window, then flash it" true, and
    // nothing enforces it — a driver configured a quarter turn out reports a
    // size the simulator never laid out against. So it is said at boot, where
    // a mismatch is one line rather than an afternoon.
    let panel = display.bounding_box().size;
    rprintln!(
        "xpui: {} {}x{}, panel {}x{}",
        board.name,
        board.width,
        board.height,
        panel.width,
        panel.height
    );

    // Built from the board handed to this loop rather than by the caller, so
    // the description the chrome is painted from and the one the keys are read
    // from cannot be two different boards.
    let mut buttons = Buttons::new(board, pins);

    let backend = Backend::leak_for_board(display, board, palette);
    // Safety: one panel, one executor task, and nothing has rendered yet.
    unsafe { xpui::host::install(backend) };

    let mut app = App::new(root);

    // Painted once before the loop, or the panel holds whatever survived reset
    // until the first button press happens to make something dirty. On e-ink
    // that is not a blank screen — it is the previous firmware's last frame.
    app.render();
    backend.clear_dirty();
    flush(backend, &mut present).await;
    let (used, free) = crate::runtime::heap_used();
    rprintln!(
        "xpui: first frame up, heap {} of {} used",
        used,
        used + free
    );

    while app.is_running() {
        // Wrapping at 49 days is the framework's contract for a clock; what
        // reads it measures short intervals, not absolute time.
        backend.begin_frame(Instant::now().as_millis() as u32);
        // A root screen has nothing underneath it, so Back is withheld there
        // rather than delivered and obeyed — see `Buttons::poll`.
        buttons.poll(backend, app.depth() > 1);

        app.tick();
        if app.render_if_dirty() {
            backend.clear_dirty();
            flush(backend, &mut present).await;
        }

        Timer::after(FRAME_INTERVAL).await;
    }

    // The root screen finished. A device has nowhere to return to, so stop and
    // leave the last frame where it can still be read.
    rprintln!("xpui: root screen finished — parking, the board is now deaf");
    park()
}

/// Takes the display out, flushes it, and lets the loan put it back.
///
/// Nothing of the backend is borrowed across the `await`, which is the point:
/// a DMA transfer or a BUSY-pin wait can be seconds long, and a borrow held
/// that far is a second task finding the state already taken.
async fn flush<D: DrawTarget>(backend: &Backend<D>, present: &mut impl AsyncFnMut(&mut D)) {
    // `None` only if something else is already presenting, which this loop
    // does not do — it awaits each flush before starting the next.
    //
    // **A firmware copying this into a two-task design does not get that for
    // free.** There the `None` arm is a dropped frame with no signal, and it
    // should log or retry rather than skip silently.
    if let Some(mut display) = backend.loan_display() {
        present(&mut display).await;
    }
}
