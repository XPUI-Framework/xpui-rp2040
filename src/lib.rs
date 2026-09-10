//! The [`xpui`] gallery, as firmware for two Pimoroni RP2040 boards.
//!
//! `badger2040` drives 296x128 of monochrome e-ink through a UC8151 over SPI.
//! `tufty2040` drives a 320x240 colour IPS panel through an ST7789v over an
//! 8-bit parallel bus. Both flash the *same* screens — `gallery::Menu` and
//! everything it opens, the crate the desktop simulator runs. What differs is
//! a [`Board`](xpui_boards_core::Board), a [`Palette`](xpui_eg::Palette), and which
//! pin is wired to what.
//!
//! This crate supplies the three things a bare-metal host has to bring itself:
//! a heap, somewhere for a panic to go, and the loop in [`run`] that turns
//! GPIO edges into [`xpui::Button`] presses and repaints only when the
//! framework asks it to.

#![no_std]
#![deny(missing_docs)]

extern crate alloc;

mod buttons;
mod frame;
mod runtime;

pub use buttons::ButtonPins;
pub use frame::{run, run_async};
pub use runtime::{init_heap, init_log, park};
