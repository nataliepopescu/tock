// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Component for any air quality sensor.
//!
//! Usage
//! -----
//! ```rust
//! let temp = AirQualityComponent::new(board_kernel, nrf52::temperature::TEMP)
//!     .finalize(air_quality_component_static!());
//! ```

use capsules_extra::air_quality::AirQualitySensor;
use core::mem::MaybeUninit;
use crate::kernel::capabilities::MemoryAllocationCapability;
use crate::kernel::component::Component;
use crate::kernel::hil;

#[macro_export]
macro_rules! air_quality_component_static {
    () => {{ crate::kernel::static_buf!(capsules_extra::air_quality::AirQualitySensor<'static>) }};
}

pub struct AirQualityComponent<
    T: 'static + hil::sensors::AirQualityDriver<'static>,
    CAP: MemoryAllocationCapability + 'static,
> {
    board_kernel: &'static crate::kernel::Kernel,
    driver_num: usize,
    temp_sensor: &'static T,
    mem_cap: CAP,
}

impl<
    T: 'static + hil::sensors::AirQualityDriver<'static>,
    CAP: MemoryAllocationCapability + 'static,
> AirQualityComponent<T, CAP>
{
    pub fn new(
        board_kernel: &'static crate::kernel::Kernel,
        driver_num: usize,
        temp_sensor: &'static T,
        mem_cap: CAP,
    ) -> AirQualityComponent<T, CAP> {
        AirQualityComponent {
            board_kernel,
            driver_num,
            temp_sensor,
            mem_cap,
        }
    }
}

impl<
    T: 'static + hil::sensors::AirQualityDriver<'static>,
    CAP: MemoryAllocationCapability + 'static,
> Component for AirQualityComponent<T, CAP>
{
    type StaticInput = &'static mut MaybeUninit<AirQualitySensor<'static>>;
    type Output = &'static AirQualitySensor<'static>;

    fn finalize(self, s: Self::StaticInput) -> Self::Output {
        let air_quality = s.write(AirQualitySensor::new(
            self.temp_sensor,
            self.board_kernel
                .create_grant(self.driver_num, &self.mem_cap),
        ));

        hil::sensors::AirQualityDriver::set_client(self.temp_sensor, air_quality);
        air_quality
    }
}
