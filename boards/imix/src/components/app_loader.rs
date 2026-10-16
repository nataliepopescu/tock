// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2024.

//! Component for Dynamic App Loading Drivers.
//!
//! This provides one component, AppLoaderComponent, which provides
//! a system call interface to the dynamic app loading capsule.
//!
//! Example instantiation:
//!
//! ```rust
//! # use crate::kernel::static_init;
//!
//! type NonVolatilePages = crate::components::dynamic_binary_storage::NVPages<nrf52840::nvmc::Nvmc>;
//! type DynamicBinaryStorage<'a> = crate::kernel::dynamic_binary_storage::SequentialDynamicBinaryStorage<
//! 'static,
//! nrf52840::chip::NRF52<'a, Nrf52840DefaultPeripherals<'a>>,
//! crate::kernel::process::ProcessStandardDebugFull,
//! NonVolatilePages,
//! >;
//!
//! let dynamic_app_loader = crate::components::app_loader::AppLoaderComponent::new(
//!     board_kernel,
//!     crate::capsules_extra::app_loader::DRIVER_NUM,
//!     dynamic_binary_storage,
//!     dynamic_binary_storage,
//!     dynamic_binary_storage,
//!     create_capability!(capabilities::MemoryAllocationCapability),
//!     ).finalize(crate::components::app_loader_component_static!(
//!     DynamicBinaryStorage<'static>,
//!     DynamicBinaryStorage<'static>,
//!     DynamicBinaryStorage<'static>,
//!     ));
//! ```

use crate::capsules_extra::app_loader::AppLoader;
use core::mem::MaybeUninit;
use crate::kernel::capabilities::MemoryAllocationCapability;
use crate::kernel::component::Component;
use crate::kernel::dynamic_binary_storage;

// Setup static space for the objects.
#[macro_export]
macro_rules! app_loader_component_static {
    ($S:ty, $L:ty, $T:ty $(,)?) => {{
        let al = crate::kernel::static_buf!(crate::capsules_extra::app_loader::AppLoader<$S, $L, $T>);
        let buffer = crate::kernel::static_buf!([u8; crate::capsules_extra::app_loader::BUF_LEN]);

        (al, buffer)
    }};
}

pub struct AppLoaderComponent<
    S: dynamic_binary_storage::DynamicBinaryStore + 'static,
    L: dynamic_binary_storage::DynamicProcessLoad + 'static,
    T: dynamic_binary_storage::DynamicProcessUnload + 'static,
    CAP: MemoryAllocationCapability + 'static,
> {
    board_kernel: &'static crate::kernel::Kernel,
    driver_num: usize,
    storage_driver: &'static S,
    load_driver: &'static L,
    unload_driver: &'static T,
    mem_cap: CAP,
}

impl<
    S: dynamic_binary_storage::DynamicBinaryStore + 'static,
    L: dynamic_binary_storage::DynamicProcessLoad + 'static,
    T: dynamic_binary_storage::DynamicProcessUnload + 'static,
    CAP: MemoryAllocationCapability + 'static,
> AppLoaderComponent<S, L, T, CAP>
{
    pub fn new(
        board_kernel: &'static crate::kernel::Kernel,
        driver_num: usize,
        storage_driver: &'static S,
        load_driver: &'static L,
        unload_driver: &'static T,
        mem_cap: CAP,
    ) -> Self {
        Self {
            board_kernel,
            driver_num,
            storage_driver,
            load_driver,
            unload_driver,
            mem_cap,
        }
    }
}

impl<
    S: dynamic_binary_storage::DynamicBinaryStore + 'static,
    L: dynamic_binary_storage::DynamicProcessLoad + 'static,
    T: dynamic_binary_storage::DynamicProcessUnload + 'static,
    CAP: MemoryAllocationCapability + 'static,
> Component for AppLoaderComponent<S, L, T, CAP>
{
    type StaticInput = (
        &'static mut MaybeUninit<AppLoader<S, L, T>>,
        &'static mut MaybeUninit<[u8; crate::capsules_extra::app_loader::BUF_LEN]>,
    );
    type Output = &'static AppLoader<S, L, T>;

    fn finalize(self, static_buffer: Self::StaticInput) -> Self::Output {
        let buffer = static_buffer
            .1
            .write([0; crate::capsules_extra::app_loader::BUF_LEN]);

        let dynamic_app_loader = static_buffer.0.write(AppLoader::new(
            self.board_kernel
                .create_grant(self.driver_num, &self.mem_cap),
            self.storage_driver,
            self.load_driver,
            self.unload_driver,
            buffer,
        ));
        dynamic_binary_storage::DynamicBinaryStore::set_storage_client(
            self.storage_driver,
            dynamic_app_loader,
        );
        dynamic_binary_storage::DynamicProcessLoad::set_load_client(
            self.load_driver,
            dynamic_app_loader,
        );
        dynamic_binary_storage::DynamicProcessUnload::set_unload_client(
            self.unload_driver,
            dynamic_app_loader,
        );
        dynamic_app_loader
    }
}
