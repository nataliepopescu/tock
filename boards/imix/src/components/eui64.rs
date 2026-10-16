// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2024.

//! Component for EUI-64 (Extended Unique Identifier).
//!
//! Usage
//! -----
//! ```rust
//!let eui64 = crate::components::eui64::Eui64Component::new(u64::from_le_bytes(device_id))
//!     .finalize(crate::components::eui64_component_static!());
//! ```

use crate::capsules_extra::eui64::Eui64;
use core::mem::MaybeUninit;
use crate::kernel::component::Component;

#[macro_export]
macro_rules! eui64_component_static {
    () => {{ crate::kernel::static_buf!(crate::capsules_extra::eui64::Eui64) }};
}

pub type Eui64ComponentType = crate::capsules_extra::eui64::Eui64;

pub struct Eui64Component {
    eui64: u64,
}

impl Eui64Component {
    pub fn new(eui64: u64) -> Self {
        Self { eui64 }
    }
}

impl Component for Eui64Component {
    type StaticInput = &'static mut MaybeUninit<Eui64>;
    type Output = &'static Eui64;

    fn finalize(self, s: Self::StaticInput) -> Self::Output {
        s.write(Eui64::new(self.eui64))
    }
}
