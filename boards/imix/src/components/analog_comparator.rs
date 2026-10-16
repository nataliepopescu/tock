// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Component for initializing an Analog Comparator.
//!
//! This provides one Component, AcComponent, which implements a userspace
//! syscall interface to a passed analog comparator driver.
//!
//! Usage
//! -----
//! ```rust
//! let analog_comparator = crate::components::analog_comparator::AnalogComparatorComponent::new(
//!     &crate::sam4l::acifc::ACIFC,
//!     crate::components::analog_comparator_component_helper!(
//!         <crate::sam4l::acifc::Acifc as crate::kernel::hil::analog_comparator::AnalogComparator>::Channel,
//!         &crate::sam4l::acifc::CHANNEL_AC0,
//!         &crate::sam4l::acifc::CHANNEL_AC1,
//!         &crate::sam4l::acifc::CHANNEL_AC2,
//!         &crate::sam4l::acifc::CHANNEL_AC3
//!     ),
//! )
//! .finalize(crate::components::analog_comparator_component_static!(crate::sam4l::acifc::Acifc));
//! ```

use crate::capsules_extra::analog_comparator::AnalogComparator;
use core::mem::MaybeUninit;
use crate::kernel::capabilities::MemoryAllocationCapability;
use crate::kernel::component::Component;

#[macro_export]
macro_rules! analog_comparator_component_helper {
    ($Channel:ty, $($P:expr_2021),+ $(,)?) => {{
        use crate::kernel::count_expressions;
        use crate::kernel::static_init;
        const NUM_CHANNELS: usize = count_expressions!($($P),+);

        static_init!(
            [&'static $Channel; NUM_CHANNELS],
            [
                $(static_init!($Channel, $P),)*
            ]
        )
    }};
}

#[macro_export]
macro_rules! analog_comparator_component_static {
    ($AC:ty $(,)?) => {{ crate::kernel::static_buf!(crate::capsules_extra::analog_comparator::AnalogComparator<'static, $AC>) }};
}

pub type AnalogComparatorComponentType<AC> = AnalogComparator<'static, AC>;

pub struct AnalogComparatorComponent<
    AC: 'static + crate::kernel::hil::analog_comparator::AnalogComparator<'static>,
    CAP: MemoryAllocationCapability + 'static,
> {
    comp: &'static AC,
    ac_channels: &'static [&'static AC::Channel],
    board_kernel: &'static crate::kernel::Kernel,
    driver_num: usize,
    mem_cap: CAP,
}

impl<
    AC: 'static + crate::kernel::hil::analog_comparator::AnalogComparator<'static>,
    CAP: MemoryAllocationCapability + 'static,
> AnalogComparatorComponent<AC, CAP>
{
    pub fn new(
        comp: &'static AC,
        ac_channels: &'static [&'static AC::Channel],
        board_kernel: &'static crate::kernel::Kernel,
        driver_num: usize,
        mem_cap: CAP,
    ) -> Self {
        Self {
            comp,
            ac_channels,
            board_kernel,
            driver_num,
            mem_cap,
        }
    }
}

impl<
    AC: 'static + crate::kernel::hil::analog_comparator::AnalogComparator<'static>,
    CAP: MemoryAllocationCapability + 'static,
> Component for AnalogComparatorComponent<AC, CAP>
{
    type StaticInput = &'static mut MaybeUninit<AnalogComparator<'static, AC>>;
    type Output = &'static AnalogComparator<'static, AC>;

    fn finalize(self, static_buffer: Self::StaticInput) -> Self::Output {
        let grant_ac = self
            .board_kernel
            .create_grant(self.driver_num, &self.mem_cap);

        let analog_comparator =
            static_buffer.write(AnalogComparator::new(self.comp, self.ac_channels, grant_ac));
        self.comp.set_client(analog_comparator);

        analog_comparator
    }
}
