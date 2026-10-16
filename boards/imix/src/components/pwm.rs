// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Components for using PWM.

use crate::capsules_core::virtualizers::virtual_pwm::{MuxPwm, PwmPinUser};
use crate::capsules_extra::pwm::Pwm;
use core::mem::MaybeUninit;
use crate::kernel::capabilities::MemoryAllocationCapability;
use crate::kernel::component::Component;
use crate::kernel::hil::pwm;

#[macro_export]
macro_rules! pwm_mux_component_static {
    ($A:ty $(,)?) => {{ crate::kernel::static_buf!(crate::capsules_core::virtualizers::virtual_pwm::MuxPwm<'static, $A>) }};
}

#[macro_export]
macro_rules! pwm_pin_user_component_static {
    ($A:ty $(,)?) => {{ crate::kernel::static_buf!(crate::capsules_core::virtualizers::virtual_pwm::PwmPinUser<'static, $A>) }};
}

#[macro_export]
macro_rules! pwm_driver_component_helper {
    ($($P:expr),+ $(,)?) => {{
        use crate::kernel::count_expressions;
        use crate::kernel::static_init;
        const NUM_DRIVERS: usize = count_expressions!($($P),+);

        let drivers = static_init!(
            [&'static dyn crate::kernel::hil::pwm::PwmPin; NUM_DRIVERS],
            [
                $($P,)*
            ]
        );
        let pwm = crate::kernel::static_buf!(crate::capsules_extra::pwm::Pwm<'static, NUM_DRIVERS>);
        (pwm, drivers)
    }};
}

pub struct PwmMuxComponent<P: 'static + pwm::Pwm> {
    pwm: &'static P,
}

impl<P: 'static + pwm::Pwm> PwmMuxComponent<P> {
    pub fn new(pwm: &'static P) -> Self {
        PwmMuxComponent { pwm }
    }
}

impl<P: 'static + pwm::Pwm> Component for PwmMuxComponent<P> {
    type StaticInput = &'static mut MaybeUninit<MuxPwm<'static, P>>;
    type Output = &'static MuxPwm<'static, P>;

    fn finalize(self, static_buffer: Self::StaticInput) -> Self::Output {
        let pwm_mux = static_buffer.write(MuxPwm::new(self.pwm));

        pwm_mux
    }
}

pub struct PwmPinUserComponent<P: 'static + pwm::Pwm> {
    pwm_mux: &'static MuxPwm<'static, P>,
    channel: P::Pin,
}

impl<P: 'static + pwm::Pwm> PwmPinUserComponent<P> {
    pub fn new(mux: &'static MuxPwm<'static, P>, channel: P::Pin) -> Self {
        PwmPinUserComponent {
            pwm_mux: mux,
            channel,
        }
    }
}

impl<P: 'static + pwm::Pwm> Component for PwmPinUserComponent<P> {
    type StaticInput = &'static mut MaybeUninit<PwmPinUser<'static, P>>;
    type Output = &'static PwmPinUser<'static, P>;

    fn finalize(self, static_buffer: Self::StaticInput) -> Self::Output {
        let pwm_pin = static_buffer.write(PwmPinUser::new(self.pwm_mux, self.channel));

        pwm_pin.add_to_mux();

        pwm_pin
    }
}

pub type PwmDriverComponentType<const NUM_PINS: usize> =
    crate::capsules_extra::pwm::Pwm<'static, NUM_PINS>;

pub struct PwmDriverComponent<const NUM_PINS: usize, CAP: MemoryAllocationCapability + 'static> {
    board_kernel: &'static crate::kernel::Kernel,
    driver_num: usize,
    mem_cap: CAP,
}

impl<const NUM_PINS: usize, CAP: MemoryAllocationCapability + 'static>
    PwmDriverComponent<NUM_PINS, CAP>
{
    pub fn new(
        board_kernel: &'static crate::kernel::Kernel,
        driver_num: usize,
        mem_cap: CAP,
    ) -> PwmDriverComponent<NUM_PINS, CAP> {
        PwmDriverComponent {
            board_kernel,
            driver_num,
            mem_cap,
        }
    }
}

impl<const NUM_PINS: usize, CAP: MemoryAllocationCapability + 'static> Component
    for PwmDriverComponent<NUM_PINS, CAP>
{
    type StaticInput = (
        &'static mut MaybeUninit<Pwm<'static, NUM_PINS>>,
        &'static [&'static dyn crate::kernel::hil::pwm::PwmPin; NUM_PINS],
    );
    type Output = &'static crate::capsules_extra::pwm::Pwm<'static, NUM_PINS>;

    fn finalize(self, static_buffer: Self::StaticInput) -> Self::Output {
        let grant_adc = self
            .board_kernel
            .create_grant(self.driver_num, &self.mem_cap);

        let pwm = static_buffer
            .0
            .write(crate::capsules_extra::pwm::Pwm::new(static_buffer.1, grant_adc));

        pwm
    }
}
