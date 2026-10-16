// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Components for collections of LEDs.
//!
//! Usage
//! -----
//! ```rust
//! let led = crate::components::led::LedsComponent::new().finalize(crate::components::led_component_static!(
//!     crate::kernel::hil::led::LedLow<'static, crate::sam4l::gpio::GPIOPin>,
//!     LedLow::new(&crate::sam4l::gpio::PORT[LED_RED_PIN]),
//!     LedLow::new(&crate::sam4l::gpio::PORT[LED_GREEN_PIN]),
//!     LedLow::new(&crate::sam4l::gpio::PORT[LED_BLUE_PIN]),
//! ));
//! ```

use crate::capsules_core::led::LedDriver;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use crate::kernel::component::Component;
use crate::kernel::hil::led::Led;

#[macro_export]
macro_rules! led_component_static {
    ($Led:ty, $($L:expr),+ $(,)?) => {{
        use crate::kernel::count_expressions;
        use crate::kernel::static_init;
        const NUM_LEDS: usize = count_expressions!($($L),+);
        let arr = static_init!(
            [&'static $Led; NUM_LEDS],
            [
                $(
                    static_init!(
                        $Led,
                        $L
                    )
                ),+
            ]
        );

        let led = crate::kernel::static_buf!( crate::capsules_core::led::LedDriver<'static, $Led, NUM_LEDS>);
        (led, arr)
    }};
}

pub type LedsComponentType<L, const NUMLEDS: usize> = LedDriver<'static, L, NUMLEDS>;

pub struct LedsComponent<L: 'static + Led, const NUM_LEDS: usize> {
    _phantom: PhantomData<L>,
}

impl<L: 'static + Led, const NUM_LEDS: usize> LedsComponent<L, NUM_LEDS> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<L: 'static + Led, const NUM_LEDS: usize> Component for LedsComponent<L, NUM_LEDS> {
    type StaticInput = (
        &'static mut MaybeUninit<LedDriver<'static, L, NUM_LEDS>>,
        &'static mut [&'static L; NUM_LEDS],
    );
    type Output = &'static LedDriver<'static, L, NUM_LEDS>;

    fn finalize(self, static_buffer: Self::StaticInput) -> Self::Output {
        static_buffer.0.write(LedDriver::new(static_buffer.1))
    }
}
