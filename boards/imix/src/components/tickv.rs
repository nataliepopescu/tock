// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Component for TicKV KV System Driver.
//!
//! This provides one component, TicKVComponent, which provides
//! a system call inteface to non-volatile storage.
//!
//! Usage
//! -----
//! ```rust
//!    let flash_ctrl_read_buf = static_init!(
//!        [u8; lowrisc::flash_ctrl::PAGE_SIZE],
//!        [0; lowrisc::flash_ctrl::PAGE_SIZE]
//!    );
//!    let page_buffer = static_init!(
//!        lowrisc::flash_ctrl::LowRiscPage,
//!        lowrisc::flash_ctrl::LowRiscPage::default()
//!    );
//!
//!    let mux_flash = crate::components::tickv::FlashMuxComponent::new(&peripherals.flash_ctrl).finalize(
//!        crate::components::flash_user_component_static!(lowrisc::flash_ctrl::FlashCtrl),
//!    );
//!
//!    // SipHash
//!    let sip_hash = static_init!(
//!        crate::capsules_extra::sip_hash::SipHasher24,
//!        crate::capsules_extra::sip_hash::SipHasher24::new()
//!    );
//!    sip_hash.register();
//!
//!    let tickv = crate::components::tickv::TicKVComponent::new(
//!        sip_hash,
//!        &mux_flash,
//!        0x20040000 / lowrisc::flash_ctrl::PAGE_SIZE,
//!        0x40000,
//!        flash_ctrl_read_buf,
//!        page_buffer,
//!    )
//!    .finalize(crate::components::tickv_component_static!(
//!        lowrisc::flash_ctrl::FlashCtrl,
//!        crate::capsules_extra::sip_hash::SipHasher24
//!    ));
//!    hil::flash::HasClient::set_client(&peripherals.flash_ctrl, mux_flash);
//! ```

use crate::capsules_core::virtualizers::virtual_flash::FlashUser;
use crate::capsules_core::virtualizers::virtual_flash::MuxFlash;
use crate::capsules_extra::tickv::TicKVSystem;
use core::mem::MaybeUninit;
use crate::kernel::component::Component;
use crate::kernel::hil;
use crate::kernel::hil::flash::HasClient;
use crate::kernel::hil::hasher::Hasher;

// Setup static space for the objects.
#[macro_export]
macro_rules! tickv_component_static {
    ($F:ty, $H:ty, $PAGE_SIZE:expr $(,)?) => {{
        let flash =
            crate::kernel::static_buf!(crate::capsules_core::virtualizers::virtual_flash::FlashUser<'static, $F>);
        let tickv = crate::kernel::static_buf!(
            crate::capsules_extra::tickv::TicKVSystem<
                'static,
                crate::capsules_core::virtualizers::virtual_flash::FlashUser<'static, $F>,
                $H,
                $PAGE_SIZE,
            >
        );

        (flash, tickv)
    }};
}

#[macro_export]
macro_rules! tickv_dedicated_flash_component_static {
    ($F:ty, $H:ty, $PAGE_SIZE:expr $(,)?) => {{
        let tickfs_read_buffer = crate::kernel::static_buf!([u8; $PAGE_SIZE]);
        let tickv =
            crate::kernel::static_buf!(crate::capsules_extra::tickv::TicKVSystem<'static, $F, $H, $PAGE_SIZE>);

        (tickv, tickfs_read_buffer)
    }};
}

pub struct TicKVComponent<
    F: 'static + hil::flash::Flash + hil::flash::HasClient<'static, MuxFlash<'static, F>>,
    H: 'static + Hasher<'static, 8>,
    const PAGE_SIZE: usize,
> {
    mux_flash: &'static MuxFlash<'static, F>,
    hasher: &'static H,
    region_offset: usize,
    flash_size: usize,
    tickfs_read_buf: &'static mut [u8; PAGE_SIZE],
    flash_read_buffer: &'static mut F::Page,
}

impl<
    F: 'static + hil::flash::Flash + hil::flash::HasClient<'static, MuxFlash<'static, F>>,
    H: Hasher<'static, 8>,
    const PAGE_SIZE: usize,
> TicKVComponent<F, H, PAGE_SIZE>
{
    pub fn new(
        hasher: &'static H,
        mux_flash: &'static MuxFlash<'static, F>,
        region_offset: usize,
        flash_size: usize,
        tickfs_read_buf: &'static mut [u8; PAGE_SIZE],
        flash_read_buffer: &'static mut F::Page,
    ) -> Self {
        Self {
            mux_flash,
            hasher,
            region_offset,
            flash_size,
            tickfs_read_buf,
            flash_read_buffer,
        }
    }
}

impl<
    F: 'static + hil::flash::Flash + hil::flash::HasClient<'static, MuxFlash<'static, F>>,
    H: 'static + Hasher<'static, 8>,
    const PAGE_SIZE: usize,
> Component for TicKVComponent<F, H, PAGE_SIZE>
{
    type StaticInput = (
        &'static mut MaybeUninit<FlashUser<'static, F>>,
        &'static mut MaybeUninit<TicKVSystem<'static, FlashUser<'static, F>, H, PAGE_SIZE>>,
    );
    type Output = &'static TicKVSystem<'static, FlashUser<'static, F>, H, PAGE_SIZE>;

    fn finalize(self, static_buffer: Self::StaticInput) -> Self::Output {
        let virtual_flash = static_buffer.0.write(FlashUser::new(self.mux_flash));

        let driver = static_buffer.1.write(TicKVSystem::new(
            virtual_flash,
            self.hasher,
            self.tickfs_read_buf,
            self.flash_read_buffer,
            self.region_offset,
            self.flash_size,
        ));
        virtual_flash.set_client(driver);
        driver.initialise();
        driver
    }
}

pub type TicKVDedicatedFlashComponentType<F, H, const PAGE: usize> =
    crate::capsules_extra::tickv::TicKVSystem<'static, F, H, PAGE>;

pub struct TicKVDedicatedFlashComponent<
    F: 'static
        + hil::flash::Flash
        + hil::flash::HasClient<'static, TicKVSystem<'static, F, H, PAGE_SIZE>>,
    H: 'static + Hasher<'static, 8>,
    const PAGE_SIZE: usize,
> {
    flash: &'static F,
    hasher: &'static H,
    region_offset: usize,
    flash_size: usize,
    flash_read_buffer: &'static mut F::Page,
}

impl<
    F: 'static
        + hil::flash::Flash
        + hil::flash::HasClient<'static, TicKVSystem<'static, F, H, PAGE_SIZE>>,
    H: Hasher<'static, 8>,
    const PAGE_SIZE: usize,
> TicKVDedicatedFlashComponent<F, H, PAGE_SIZE>
{
    pub fn new(
        hasher: &'static H,
        flash: &'static F,
        region_offset: usize,
        flash_size: usize,
        flash_read_buffer: &'static mut F::Page,
    ) -> Self {
        Self {
            flash,
            hasher,
            region_offset,
            flash_size,
            flash_read_buffer,
        }
    }
}

impl<
    F: 'static
        + hil::flash::Flash
        + hil::flash::HasClient<'static, TicKVSystem<'static, F, H, PAGE_SIZE>>,
    H: 'static + Hasher<'static, 8>,
    const PAGE_SIZE: usize,
> Component for TicKVDedicatedFlashComponent<F, H, PAGE_SIZE>
{
    type StaticInput = (
        &'static mut MaybeUninit<TicKVSystem<'static, F, H, PAGE_SIZE>>,
        &'static mut MaybeUninit<[u8; PAGE_SIZE]>,
    );
    type Output = &'static TicKVSystem<'static, F, H, PAGE_SIZE>;

    fn finalize(self, static_buffer: Self::StaticInput) -> Self::Output {
        let tickfs_read_buf = static_buffer.1.write([0; PAGE_SIZE]);

        let tickv = static_buffer.0.write(TicKVSystem::new(
            self.flash,
            self.hasher,
            tickfs_read_buf,
            self.flash_read_buffer,
            self.region_offset,
            self.flash_size,
        ));
        self.flash.set_client(tickv);
        self.hasher.set_client(tickv);
        tickv.initialise();
        tickv
    }
}
