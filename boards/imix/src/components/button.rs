// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Component for Buttons.
//!
//! Usage
//! -----
//!
//! The `button_component_helper!` macro takes 'static references to GPIO pins.
//! When GPIO instances are owned values, the `button_component_helper_owned!`
//! can be used, indicating that the passed values are owned values. This macro
//! will perform static allocation of the passed in GPIO pins internally.
//!
//! ```rust
//! let button = crate::components::button::ButtonComponent::new(
//!     board_kernel,
//!     crate::components::button_component_helper!(
//!         crate::sam4l::gpio::GPIOPin,
//!         (
//!             &crate::sam4l::gpio::PC[24],
//!             crate::kernel::hil::gpio::ActivationMode::ActiveLow,
//!             crate::kernel::hil::gpio::FloatingState::PullUp
//!         )
//!     ),
//! )
//! .finalize(button_component_static!(crate::sam4l::gpio::GPIOPin));
//! ```
//!
//! Typically, `ActivationMode::ActiveLow` will be associated with
//! `FloatingState::PullUp` whereas `ActivationMode::ActiveHigh` will be paired
//! with `FloatingState::PullDown`. `FloatingState::None` will be used when the
//! board provides external pull-up/pull-down resistors.

use crate::capsules_core::button::Button;
use core::mem::MaybeUninit;
use crate::kernel::capabilities::MemoryAllocationCapability;
use crate::kernel::component::Component;
use crate::kernel::hil::gpio;
use crate::kernel::hil::gpio::InterruptWithValue;

#[macro_export]
macro_rules! button_component_helper_owned {
    ($Pin:ty, $(($P:expr_2021, $M:expr_2021, $F:expr_2021)),+ $(,)?) => {
        $crate::components::button_component_helper!(
            $Pin,
            $((
                static_init!($Pin, $P),
                $M,
                $F
            ),)*
        )
    };
}

#[macro_export]
macro_rules! button_component_helper {
    ($Pin:ty, $(($P:expr_2021, $M:expr_2021, $F:expr_2021)),+ $(,)?) => {{
        use crate::kernel::static_init;
        use crate::kernel::count_expressions;
        use crate::kernel::hil::gpio::InterruptValueWrapper;
        const NUM_BUTTONS: usize = count_expressions!($($P),+);

        static_init!(
            [(&'static InterruptValueWrapper<'static, $Pin>, crate::kernel::hil::gpio::ActivationMode, crate::kernel::hil::gpio::FloatingState); NUM_BUTTONS],
            [
                $(
                    (static_init!(InterruptValueWrapper<$Pin>, InterruptValueWrapper::new($P))
                    .finalize(),
                    $M,
                    $F
                    ),
                )*
            ]
        )
    }};
}

#[macro_export]
macro_rules! button_component_static {
    ($Pin:ty $(,)?) => {{ crate::kernel::static_buf!(crate::capsules_core::button::Button<'static, $Pin>) }};
}

pub type ButtonComponentType<IP> = crate::capsules_core::button::Button<'static, IP>;

pub struct ButtonComponent<
    IP: 'static + gpio::InterruptPin<'static>,
    CAP: MemoryAllocationCapability + 'static,
> {
    board_kernel: &'static crate::kernel::Kernel,
    driver_num: usize,
    button_pins: &'static [(
        &'static gpio::InterruptValueWrapper<'static, IP>,
        gpio::ActivationMode,
        gpio::FloatingState,
    )],
    mem_cap: CAP,
}

impl<IP: 'static + gpio::InterruptPin<'static>, CAP: MemoryAllocationCapability + 'static>
    ButtonComponent<IP, CAP>
{
    pub fn new(
        board_kernel: &'static crate::kernel::Kernel,
        driver_num: usize,
        button_pins: &'static [(
            &'static gpio::InterruptValueWrapper<'static, IP>,
            gpio::ActivationMode,
            gpio::FloatingState,
        )],
        mem_cap: CAP,
    ) -> Self {
        Self {
            board_kernel,
            driver_num,
            button_pins,
            mem_cap,
        }
    }
}

impl<IP: 'static + gpio::InterruptPin<'static>, CAP: MemoryAllocationCapability + 'static> Component
    for ButtonComponent<IP, CAP>
{
    type StaticInput = &'static mut MaybeUninit<Button<'static, IP>>;
    type Output = &'static Button<'static, IP>;

    fn finalize(self, static_buffer: Self::StaticInput) -> Self::Output {
        let button = static_buffer.write(crate::capsules_core::button::Button::new(
            self.button_pins,
            self.board_kernel
                .create_grant(self.driver_num, &self.mem_cap),
        ));
        for (pin, _, _) in self.button_pins.iter() {
            pin.set_client(button);
        }

        button
    }
}
