//! The [`xpui`] gallery, as firmware for two Pimoroni RP2040 boards.
//!
//! `badger2040` drives 296x128 of monochrome e-ink through a UC8151 over SPI.
//! `tufty2040` drives a 320x240 colour IPS panel through an ST7789v over an
//! 8-bit parallel bus. Both flash the *same* screens — `gallery::Menu` and
//! everything it opens, the crate the desktop simulator runs. What differs is
//! a [`Board`](xpui_chrome::Board), a [`Palette`](xpui_eg::Palette), and which
//! pin is wired to what.
//!
//! This crate supplies the three things a bare-metal host has to bring itself:
//! a heap, somewhere for a panic to go, and the loop in [`run`] that turns
//! GPIO edges into [`xpui::Button`] presses and repaints only when the
//! framework asks it to.
//!
//! # Off the device
//!
//! Every item here is behind `cfg(device)`, which `build.rs` turns on only for
//! bare-metal ARM. The workspace's host gates build every member for the
//! machine they run on, and an RP2040 HAL does not compile there — so off the
//! device this crate is empty and its dependencies are never resolved.
//!
//! The consequence is that `cargo clippy --workspace` does not see any of this
//! code. Lint it by building it for the target it is for:
//!
//! ```bash
//! cargo clippy --release -p xpui-rp2040 --all-targets --target thumbv6m-none-eabi -- -D warnings
//! ```

#![cfg_attr(device, no_std)]

#[cfg(device)]
extern crate alloc;

#[cfg(device)]
mod buttons;
#[cfg(device)]
mod frame;
#[cfg(device)]
mod runtime;

#[cfg(device)]
pub use buttons::{ButtonPins, Buttons};
#[cfg(device)]
pub use frame::run;
#[cfg(device)]
pub use runtime::{init_heap, park};
