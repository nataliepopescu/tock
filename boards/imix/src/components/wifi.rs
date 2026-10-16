// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OxidOS Automotive 2025.

use crate::capsules_extra::wifi;
use core::mem::MaybeUninit;
use crate::kernel::capabilities::MemoryAllocationCapability;
use crate::kernel::component::Component;

#[macro_export]
macro_rules! wifi_component_static {
    ($D:ty $(,)?) => {{ crate::kernel::static_buf!(crate::capsules_extra::wifi::WifiDriver<'static, $D>) }};
}

pub struct WifiComponent<
    D: 'static + wifi::Device<'static>,
    CAP: MemoryAllocationCapability + 'static,
> {
    board_kernel: &'static crate::kernel::Kernel,
    driver_num: usize,
    device: &'static D,
    mem_cap: CAP,
}

impl<D: 'static + wifi::Device<'static>, CAP: MemoryAllocationCapability + 'static>
    WifiComponent<D, CAP>
{
    pub fn new(
        board_kernel: &'static crate::kernel::Kernel,
        driver_num: usize,
        device: &'static D,
        mem_cap: CAP,
    ) -> Self {
        Self {
            board_kernel,
            driver_num,
            device,
            mem_cap,
        }
    }
}

impl<D: 'static + wifi::Device<'static>, CAP: MemoryAllocationCapability + 'static> Component
    for WifiComponent<D, CAP>
{
    type StaticInput = &'static mut MaybeUninit<wifi::WifiDriver<'static, D>>;
    type Output = &'static wifi::WifiDriver<'static, D>;
    fn finalize(self, static_memory: Self::StaticInput) -> Self::Output {
        let wifi = static_memory.write(wifi::WifiDriver::new(
            self.device,
            self.board_kernel
                .create_grant(self.driver_num, &self.mem_cap),
        ));
        self.device.set_client(wifi);

        wifi
    }
}
