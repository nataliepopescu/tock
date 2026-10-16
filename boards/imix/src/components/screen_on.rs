// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2025.

//! Components for implementing outputs on the screen.
//!
//! Supported examples:
//! - LED: draw LEDs on the screen.

use crate::capsules_extra::screen::screen_on_led;
use core::mem::MaybeUninit;
use crate::kernel::component::Component;
use crate::kernel::hil;

#[macro_export]
macro_rules! screen_on_led_component_static {
    ($S:ty, $NUM_LEDS:expr, $SCREEN_WIDTH:expr, $SCREEN_HEIGHT:expr $(,)?) => {{
        let buffer = crate::kernel::static_buf!([u8; ($SCREEN_WIDTH * $SCREEN_HEIGHT) / 8]);
        let screen_on_led = crate::kernel::static_buf!(
            crate::capsules_extra::screen::screen_on_led::ScreenOnLed<
                'static,
                $S,
                $NUM_LEDS,
                $SCREEN_WIDTH,
                $SCREEN_HEIGHT,
            >
        );

        (buffer, screen_on_led)
    }};
}

pub type ScreenOnLedComponentType<
    S,
    const NUM_LEDS: usize,
    const SCREEN_WIDTH: usize,
    const SCREEN_HEIGHT: usize,
> = screen_on_led::ScreenOnLed<'static, S, NUM_LEDS, SCREEN_WIDTH, SCREEN_HEIGHT>;

pub struct ScreenOnLedComponent<
    S: hil::screen::Screen<'static> + 'static,
    const NUM_LEDS: usize,
    const SCREEN_WIDTH: usize,
    const SCREEN_HEIGHT: usize,
    const BUFFER_LENGTH: usize,
> {
    screen: &'static S,
}

impl<
    S: hil::screen::Screen<'static>,
    const NUM_LEDS: usize,
    const SCREEN_WIDTH: usize,
    const SCREEN_HEIGHT: usize,
    const BUFFER_LENGTH: usize,
> ScreenOnLedComponent<S, NUM_LEDS, SCREEN_WIDTH, SCREEN_HEIGHT, BUFFER_LENGTH>
{
    pub fn new(screen: &'static S) -> Self {
        Self { screen }
    }
}

impl<
    S: hil::screen::Screen<'static> + 'static,
    const NUM_LEDS: usize,
    const SCREEN_WIDTH: usize,
    const SCREEN_HEIGHT: usize,
    const BUFFER_LENGTH: usize,
> Component for ScreenOnLedComponent<S, NUM_LEDS, SCREEN_WIDTH, SCREEN_HEIGHT, BUFFER_LENGTH>
{
    type StaticInput = (
        &'static mut MaybeUninit<[u8; BUFFER_LENGTH]>,
        &'static mut MaybeUninit<
            screen_on_led::ScreenOnLed<'static, S, NUM_LEDS, SCREEN_WIDTH, SCREEN_HEIGHT>,
        >,
    );
    type Output =
        &'static screen_on_led::ScreenOnLed<'static, S, NUM_LEDS, SCREEN_WIDTH, SCREEN_HEIGHT>;

    fn finalize(self, static_input: Self::StaticInput) -> Self::Output {
        let buffer = static_input.0.write([0; BUFFER_LENGTH]);
        let screen_on_led = static_input
            .1
            .write(screen_on_led::ScreenOnLed::new(self.screen, buffer));

        crate::kernel::hil::screen::Screen::set_client(self.screen, screen_on_led);

        screen_on_led
    }
}
