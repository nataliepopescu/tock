// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2023.


pub mod test;

#[macro_use]
pub mod stream;

pub mod adc;
pub mod alarm;
pub mod button;
pub mod console;
pub mod console_ordered;
pub mod driver;
pub mod gpio;
pub mod i2c_master;
pub mod i2c_master_slave_combo;
pub mod i2c_master_slave_driver;
pub mod led;
pub mod low_level_debug;
pub mod process_console;
pub mod rng;
pub mod spi_controller;
pub mod spi_peripheral;
pub mod virtualizers;

// Re-export of this module's #[macro_export] macros (which live at the crate root).
#[allow(unused_imports)]
pub use crate::{dec_consume,dec_try,enc_consume,enc_try,stream_cond,stream_done,stream_err,stream_from_option,stream_len_cond};
