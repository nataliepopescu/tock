// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Component for APDS9960 proximity sensor.

use crate::capsules_core::virtualizers::virtual_i2c::{I2CDevice, MuxI2C};
use crate::capsules_extra::apds9960::APDS9960;
use core::mem::MaybeUninit;
use crate::kernel::component::Component;
use crate::kernel::hil::gpio;
use crate::kernel::hil::i2c;

#[macro_export]
macro_rules! apds9960_component_static {
    ($I:ty $(,)?) => {{
        let i2c_device =
            crate::kernel::static_buf!(crate::capsules_core::virtualizers::virtual_i2c::I2CDevice<'static, $I>);
        let apds9960 = crate::kernel::static_buf!(
            crate::capsules_extra::apds9960::APDS9960<
                'static,
                crate::capsules_core::virtualizers::virtual_i2c::I2CDevice<$I>,
            >
        );
        let buffer = crate::kernel::static_buf!([u8; crate::capsules_extra::apds9960::BUF_LEN]);

        (i2c_device, apds9960, buffer)
    }};
}

pub struct Apds9960Component<I: 'static + i2c::I2CMaster<'static>> {
    i2c_mux: &'static MuxI2C<'static, I>,
    i2c_address: u8,
    interrupt_pin: &'static dyn gpio::InterruptPin<'static>,
}

impl<I: 'static + i2c::I2CMaster<'static>> Apds9960Component<I> {
    pub fn new(
        i2c_mux: &'static MuxI2C<'static, I>,
        i2c_address: u8,
        interrupt_pin: &'static dyn gpio::InterruptPin<'static>,
    ) -> Self {
        Apds9960Component {
            i2c_mux,
            i2c_address,
            interrupt_pin,
        }
    }
}

impl<I: 'static + i2c::I2CMaster<'static>> Component for Apds9960Component<I> {
    type StaticInput = (
        &'static mut MaybeUninit<I2CDevice<'static, I>>,
        &'static mut MaybeUninit<APDS9960<'static, I2CDevice<'static, I>>>,
        &'static mut MaybeUninit<[u8; crate::capsules_extra::apds9960::BUF_LEN]>,
    );
    type Output = &'static APDS9960<'static, I2CDevice<'static, I>>;

    fn finalize(self, s: Self::StaticInput) -> Self::Output {
        let apds9960_i2c = s.0.write(I2CDevice::new(self.i2c_mux, self.i2c_address));

        let buffer = s.2.write([0; crate::capsules_extra::apds9960::BUF_LEN]);

        let apds9960 =
            s.1.write(APDS9960::new(apds9960_i2c, self.interrupt_pin, buffer));
        apds9960_i2c.set_client(apds9960);
        self.interrupt_pin.set_client(apds9960);

        apds9960
    }
}
