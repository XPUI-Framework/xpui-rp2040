# Reference

Every public item in `xpui-rp2040`: the frame loop a binary hands its panel to,
the pins it hands over with it, and the three calls that make a bare-metal
board a place a screen can run. [The tutorial](tutorial.md) puts a first screen
on a Badger with them; [hardware.md](hardware.md) is what each key, pin and
kilobyte is for.

Everything here compiles for `thumbv6m-none-eabi` and nothing else, so an
example that calls one of these is fenced `text` and says why. The one example
a laptop can check is a doctest in `docs-test/`.

## Topics

### Running a screen

| | |
|---|---|
| [`run`](#run) | Installs `display` as the host and runs `root` on it. |
| [`run_async`](#run_async) | The same loop, for a panel whose flush suspends. |

### Wiring

| | |
|---|---|
| [`ButtonPins`](#buttonpins) | The five switches, one field per switch. |

### The runtime

| | |
|---|---|
| [`init_log`](#init_log) | Opens the channel `probe-rs run` reads, so the board can be heard. |
| [`init_heap`](#init_heap) | Hands the allocator its memory. |
| [`park`](#park) | Stops, keeping whatever the panel last showed. |

## `run`

Installs `display` as the host and runs `root` on it.

```text
pub async fn run<D, S>(
    display: D,
    board: Board,
    palette: Palette<D::Color>,
    pins: ButtonPins,
    root: S,
    present: impl FnMut(&mut D),
) -> !
where
    D: DrawTarget + Send + 'static,
    D::Color: Sync,
    S: Screen + 'static,
```

| Parameter | Meaning |
|---|---|
| `display` | The panel driver, any `DrawTarget`. It is wired into a backend with `gallery::wire` and leaked, so it lives as long as the board runs. Its size is what the screens lay out against, whatever `board` says. |
| `board` | What the case is: `pimoroni::BADGER_2040` or `pimoroni::TUFTY_2040`. The chrome is sized from it, and each of `pins` is resolved against its keys. |
| `palette` | Which colour is ink. `Palette::INK_IS_OFF` on the Badger, whose UC8151 draws `Off` as black; `Palette::new(Rgb565::BLACK, Rgb565::WHITE)` on the Tufty. The wrong one inverts every screen and reports nothing. |
| `pins` | The five switches, as [`ButtonPins`](#buttonpins). |
| `root` | The first screen, kept for the life of the loop: Back reaches it like any key, and never finishes it. |
| `present` | Pushes the framebuffer to the glass. Called after the first paint and after every later one, and at no other time. |

Every 10 ms the loop starts a frame on the board's clock, samples the switches,
ticks the app, and paints only if the framework asked. On the way up it says
what it found over RTT: the board's size against the panel's, what each key
resolved to, and the heap after the first frame. The lines are in
[the tutorial](tutorial.md#what-has-been-proven-and-where).

> [!WARNING]
> On the Badger `present` is close to a second of the panel refreshing, and the
> switches are not read while it runs. That is why it is called only after a
> paint: presenting every frame leaves the panel permanently mid-update.

It is [`run_async`](#run_async) with the blocking `present` wrapped as an async
one that never suspends, so there is one loop to keep correct rather than two.
It never returns; if the root ever stopped being kept, the loop would end in
[`park`](#park).

**Example — the Badger's entry point**

```text
#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    init_log();
    init_heap();
    let p = embassy_rp::init(Default::default());
    // ... the 3V3 enable, `pins`, and a `Uc8151` on SPI0 ...

    run(
        display,
        pimoroni::BADGER_2040,
        Palette::INK_IS_OFF,
        pins,
        Menu::new(),
        |display: &mut _| {
            let _ = display.update(); // blocks until the refresh is done
        },
    )
    .await
}
```

Fenced `text`: an embassy entry point that owns the RP2040's peripherals, which
compiles for the board and nowhere else.
[`src/bin/badger2040.rs`](../src/bin/badger2040.rs) is all of it. The Tufty's
panel draws straight through its parallel bus, so
[`src/bin/tufty2040.rs`](../src/bin/tufty2040.rs) passes `|_display: &mut _| {}`.

**See also:** [`run_async`](#run_async), [`ButtonPins`](#buttonpins), [`park`](#park)

## `run_async`

The same loop, for a panel whose flush suspends.

```text
pub async fn run_async<D, S>(
    display: D,
    board: Board,
    palette: Palette<D::Color>,
    pins: ButtonPins,
    root: S,
    present: impl AsyncFnMut(&mut D),
) -> !
where
    D: DrawTarget + Send + 'static,
    D::Color: Sync,
    S: Screen + 'static,
```

Its parameters are [`run`](#run)'s, except `present`, which is awaited. It is
handed the display **itself**: the display is loaned out of the backend for the
length of the flush and put back afterwards, so nothing of the backend is
borrowed across the `await`. A DMA transfer or a BUSY-pin wait can take
seconds, and the loop sleeps through it rather than spinning.

> [!NOTE]
> Anything that paints while the flush is in flight is discarded. This loop
> awaits each flush before starting the next frame, so from here that never
> happens; a firmware copying the loop into a two-task design meets it as a
> dropped frame with no signal.

Neither board needs it today: `mipidsi` writes straight through, and the
published `uc8151` spins on the BUSY pin. It is here for a DMA-backed panel,
and for a driver with an async flush.

**Example — a driver that awaits its flush**

```text
run_async(
    display,
    board,
    palette,
    pins,
    Menu::new(),
    async |display: &mut _| {
        display.flush().await; // the loop sleeps until the panel is done
    },
)
.await
```

Fenced `text`: it needs embassy and a driver with an async flush, and no board
this repository flashes has one.

**See also:** [`run`](#run)

## `ButtonPins`

The five switches, one field per switch.

```text
pub struct ButtonPins
```

| Field | |
|---|---|
| `ButtonPins::a` | The switch printed `a`, leftmost along the bottom edge. |
| `ButtonPins::b` | The switch printed `b`, between `a` and `c` along the bottom edge. |
| `ButtonPins::c` | The switch printed `c`, rightmost along the bottom edge. |
| `ButtonPins::up` | The upper of the two switches on the right edge. |
| `ButtonPins::down` | The lower of the two switches on the right edge. |

Each field is an `embassy_rp::gpio::Input<'static>`. The fields are named
rather than positional because all five are one type, and a swap would compile.

**The firmware does not decide what a key means.** [`run`](#run) looks each
field's bezel label up in the board it was handed, and the board says what
that key sends:

| Field | Bezel label | Badger | Tufty | Sends, on both badges |
|---|---|---|---|---|
| `a` | `a` | GP12 | GP7 | `Back` |
| `b` | `b` | GP13 | GP8 | `Confirm` |
| `c` | `c` | GP14 | GP9 | nothing |
| `up` | `Up` | GP15 | GP22 | `Up` |
| `down` | `Dn` | GP11 | GP6 | `Down` |

A label the board does not carry resolves to nothing, and that switch is
sampled and silent. The boot log says what each resolved to:
`xpui: key Dn sends Some(Down)`, or `None` for a typo. A pin behind the wrong
field is the one fault no line can show; only pressing all five finds it.

> [!WARNING]
> **Every pin must be `Pull::Down`.** A press reads as `is_high`, so a pin
> pulled up compiles, links, and inverts all five keys: the board reports a
> press for every switch that is *not* held.

Each switch is debounced over four samples of the 10 ms loop and delivered as
an edge, never as a level, so a thumb resting on Down does not walk a list to
its end.

**Example — the Badger's pins**

```text
let pins = ButtonPins {
    a: Input::new(p.PIN_12, Pull::Down),
    b: Input::new(p.PIN_13, Pull::Down),
    c: Input::new(p.PIN_14, Pull::Down),
    up: Input::new(p.PIN_15, Pull::Down),
    down: Input::new(p.PIN_11, Pull::Down),
};
```

Fenced `text`: `p` is the RP2040's peripherals, which exist only on the board.

**Example — what the labels resolve to**

```rust
use xpui::Button;
use xpui_boards_pimoroni as pimoroni;

let bezel = pimoroni::BADGER_2040.bezel.expect("the Badger has keys");
let sends = |label: &str| {
    bezel
        .button_labelled(label)
        .and_then(|key| key.action.button())
};

assert_eq!(sends("a"), Some(Button::Back));
assert_eq!(sends("c"), None); // a key with nothing on it
assert_eq!(sends("Dn"), Some(Button::Down));
assert_eq!(sends("Down"), None); // spelled out, the switch is dead
```

**See also:** [`run`](#run), [the keys](hardware.md#the-keys), [the pins](hardware.md#pins)

## `init_log`

Opens the channel `probe-rs run` reads, so the board can be heard.

```text
pub fn init_log()
```

Call it first, before [`init_heap`](#init_heap), so a panic there still has
somewhere to go. With no probe attached, a line costs a copy into a ring buffer
that never fills.

What arrives over it is the boot log [`run`](#run) prints, and a panic's
message: the crate supplies the `#[panic_handler]`, which prints `PANIC:` and
the message, then [`park`](#park)s.

> [!NOTE]
> `probe-rs` finds the channel by looking its symbol up in the ELF. The release
> profile's `strip = "debuginfo"` keeps that symbol; `strip = true` deletes it,
> and the board then seems to print nothing.
> [The release profile](hardware.md#the-release-profile) says more.

## `init_heap`

Hands the allocator its memory.

```text
pub fn init_heap()
```

The heap is 64 KiB of the RP2040's 256 KiB of RAM, managed by a first-fit
allocator the crate installs as the `#[global_allocator]`. It holds the leaked
backend, and with it the Badger's 4,736-byte framebuffer, the stack of live
screens, and the view tree `body()` rebuilds on every frame carrying input.

Call it once, before anything allocates: `App::new` allocates on its first line.
A second call trips the allocator's own assertion rather than corrupting
anything. Raise `HEAP_SIZE` in [`src/runtime.rs`](../src/runtime.rs) before a
screen buffers an image; [the memory figures](hardware.md#memory) are measured.

> [!NOTE]
> Linking this crate brings a `#[global_allocator]` and a `#[panic_handler]`
> with it. A binary that declares its own of either does not build.

**Example — the first two lines of every binary**

```text
#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    init_log();  // first: a panic inside `init_heap` still has somewhere to go
    init_heap(); // second: `App::new` allocates on its first line
    // ...
}
```

Fenced `text`: an embassy entry point, which compiles for the board only.

## `park`

Stops, keeping whatever the panel last showed.

```text
pub fn park() -> !
```

It waits for an interrupt, forever. E-ink holds its image with the power off,
and there is no console on either board, so leaving the last frame up is the
most useful thing a firmware with nowhere to go can do.

The panic handler ends here, after saying why. So would [`run`](#run), if its
root screen ever finished: while the root is kept, it cannot.

**See also:** [`init_log`](#init_log), [`run`](#run)
