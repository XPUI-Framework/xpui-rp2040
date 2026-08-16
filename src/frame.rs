//! The frame loop, written once for both boards.
//!
//! The same shape as the desktop simulator's — begin a frame, feed it input,
//! tick, and paint only if the framework asked — with embassy where SDL was.
//! Read `crates/backend/simulator/src/lib.rs` beside this: what differs is
//! where the input comes from and what a paint costs.

use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::draw_target::DrawTarget;
use xpui::App;
use xpui::screen::Screen;
use xpui_chrome::Board;
use xpui_eg::{Backend, Palette};

use crate::Buttons;
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
pub async fn run<D, S>(
    display: D,
    board: Board,
    palette: Palette<D::Color>,
    mut buttons: Buttons,
    root: S,
    mut present: impl FnMut(&mut D),
) -> !
where
    D: DrawTarget + Send + 'static,
    S: Screen + 'static,
{
    // Sized from the board, so a screen developed in the simulator window and
    // the same screen here lay out against identical numbers.
    let backend = Backend::leak_for_board(display, board, palette);
    // Safety: one panel, one executor task, and nothing has rendered yet.
    unsafe { xpui::host::install(backend) };

    let mut app = App::new(root);

    // Painted once before the loop, or the panel holds whatever survived reset
    // until the first button press happens to make something dirty. On e-ink
    // that is not a blank screen — it is the previous firmware's last frame.
    app.render();
    backend.clear_dirty();
    backend.with_display(&mut present);

    while app.is_running() {
        // Wrapping at 49 days is the framework's contract for a clock; what
        // reads it measures short intervals, not absolute time.
        backend.begin_frame(Instant::now().as_millis() as u32);
        buttons.poll(backend);

        app.tick();
        if app.render_if_dirty() {
            backend.clear_dirty();
            backend.with_display(&mut present);
        }

        Timer::after(FRAME_INTERVAL).await;
    }

    // The root screen finished. A device has nowhere to return to, so stop and
    // leave the last frame where it can still be read.
    park()
}
