// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Component for the BMP280 Temperature and Pressure Sensor.
//!
//! Based off the SHT3x code.
//!
//! I2C Interface
//!
//! Usage
//! -----
//!
//! With the default i2c address
//! ```rust
//! let bmp280 = crate::components::bmp280::Bmp280Component::new(sensors_i2c_bus, mux_alarm).finalize(
//!         crate::components::bmp280_component_static!(nrf52::rtc::Rtc<'static>),
//!     );
//! bmp280.begin_reset();
//! ```
//!
//! With a specified i2c address
//! ```rust
//! let bmp280 = crate::components::bmp280::Bmp280Component::new(sensors_i2c_bus, mux_alarm).finalize(
//!         crate::components::bmp280_component_static!(nrf52::rtc::Rtc<'static>, crate::capsules_extra::bmp280::BASE_ADDR),
//!     );
//! bmp280.begin_reset();
//! ```

use crate::capsules_core::virtualizers::virtual_alarm::{MuxAlarm, VirtualMuxAlarm};
use crate::capsules_core::virtualizers::virtual_i2c::{I2CDevice, MuxI2C};
use crate::capsules_extra::bmp280::Bmp280;
use core::mem::MaybeUninit;
use crate::kernel::component::Component;
use crate::kernel::hil::i2c;
use crate::kernel::hil::time::Alarm;

#[macro_export]
macro_rules! bmp280_component_static {
    ($A:ty, $I:ty $(,)?) => {{
        let i2c_device =
            crate::kernel::static_buf!(crate::capsules_core::virtualizers::virtual_i2c::I2CDevice<'static, $I>);
        let alarm = crate::kernel::static_buf!(
            crate::capsules_core::virtualizers::virtual_alarm::VirtualMuxAlarm<'static, $A>
        );
        let buffer = crate::kernel::static_buf!([u8; crate::capsules_extra::bmp280::BUFFER_SIZE]);
        let bmp280 = crate::kernel::static_buf!(
            crate::capsules_extra::bmp280::Bmp280<
                'static,
                VirtualMuxAlarm<'static, $A>,
                crate::capsules_core::virtualizers::virtual_i2c::I2CDevice<'static, $I>,
            >
        );

        (i2c_device, alarm, buffer, bmp280)
    }};
}

pub type Bmp280ComponentType<A, I> = crate::capsules_extra::bmp280::Bmp280<'static, A, I>;

pub struct Bmp280Component<A: 'static + Alarm<'static>, I: 'static + i2c::I2CMaster<'static>> {
    i2c_mux: &'static MuxI2C<'static, I>,
    i2c_address: u8,
    alarm_mux: &'static MuxAlarm<'static, A>,
}

impl<A: 'static + Alarm<'static>, I: 'static + i2c::I2CMaster<'static>> Bmp280Component<A, I> {
    pub fn new(
        i2c_mux: &'static MuxI2C<'static, I>,
        i2c_address: u8,
        alarm_mux: &'static MuxAlarm<'static, A>,
    ) -> Bmp280Component<A, I> {
        Bmp280Component {
            i2c_mux,
            i2c_address,
            alarm_mux,
        }
    }
}

impl<A: 'static + Alarm<'static>, I: 'static + i2c::I2CMaster<'static>> Component
    for Bmp280Component<A, I>
{
    type StaticInput = (
        &'static mut MaybeUninit<I2CDevice<'static, I>>,
        &'static mut MaybeUninit<VirtualMuxAlarm<'static, A>>,
        &'static mut MaybeUninit<[u8; crate::capsules_extra::bmp280::BUFFER_SIZE]>,
        &'static mut MaybeUninit<
            Bmp280<'static, VirtualMuxAlarm<'static, A>, I2CDevice<'static, I>>,
        >,
    );
    type Output = &'static Bmp280<'static, VirtualMuxAlarm<'static, A>, I2CDevice<'static, I>>;

    fn finalize(self, s: Self::StaticInput) -> Self::Output {
        let bmp280_i2c = s.0.write(I2CDevice::new(self.i2c_mux, self.i2c_address));
        let bmp280_alarm = s.1.write(VirtualMuxAlarm::new(self.alarm_mux));
        bmp280_alarm.setup();

        let buffer = s.2.write([0; crate::capsules_extra::bmp280::BUFFER_SIZE]);

        let bmp280 = s.3.write(Bmp280::new(bmp280_i2c, buffer, bmp280_alarm));
        bmp280_i2c.set_client(bmp280);
        bmp280_alarm.set_alarm_client(bmp280);

        bmp280
    }
}
