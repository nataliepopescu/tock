// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2025.

//! Component for buttons using keyboard presses.
//!
//! Capsule: capsules/extra/src/button_keyboard.rs
//!
//! Implements the button system call with keyboard inputs.

use core::mem::MaybeUninit;
use crate::kernel::capabilities::MemoryAllocationCapability;
use crate::kernel::component::Component;
use crate::kernel::hil;

#[macro_export]
macro_rules! keyboard_button_component_static {
    () => {{ crate::kernel::static_buf!(crate::capsules_extra::button_keyboard::ButtonKeyboard<'static>) }};
}

pub type KeyboardButtonComponentType = crate::capsules_extra::button_keyboard::ButtonKeyboard<'static>;

pub struct KeyboardButtonComponent<
    K: 'static + hil::keyboard::Keyboard<'static>,
    CAP: MemoryAllocationCapability + 'static,
> {
    board_kernel: &'static crate::kernel::Kernel,
    driver_num: usize,
    keyboard: &'static K,
    key_codes: &'static [u16],
    mem_cap: CAP,
}

impl<K: 'static + hil::keyboard::Keyboard<'static>, CAP: MemoryAllocationCapability + 'static>
    KeyboardButtonComponent<K, CAP>
{
    pub fn new(
        board_kernel: &'static crate::kernel::Kernel,
        driver_num: usize,
        keyboard: &'static K,
        key_codes: &'static [u16],
        mem_cap: CAP,
    ) -> Self {
        Self {
            board_kernel,
            driver_num,
            keyboard,
            key_codes,
            mem_cap,
        }
    }
}

impl<K: 'static + hil::keyboard::Keyboard<'static>, CAP: MemoryAllocationCapability + 'static>
    Component for KeyboardButtonComponent<K, CAP>
{
    type StaticInput =
        &'static mut MaybeUninit<crate::capsules_extra::button_keyboard::ButtonKeyboard<'static>>;
    type Output = &'static crate::capsules_extra::button_keyboard::ButtonKeyboard<'static>;

    fn finalize(self, s: Self::StaticInput) -> Self::Output {
        let button_keyboard = s.write(crate::capsules_extra::button_keyboard::ButtonKeyboard::new(
            self.key_codes,
            self.board_kernel
                .create_grant(self.driver_num, &self.mem_cap),
        ));
        self.keyboard.set_client(button_keyboard);

        button_keyboard
    }
}
