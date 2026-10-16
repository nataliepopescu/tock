// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Component for IEEE 802.15.4 radio syscall interface.
//!
//! This provides one Component, `Ieee802154Component`, which implements a
//! userspace syscall interface to a full 802.15.4 stack with a always-on MAC
//! implementation, as well as multiplexed access to that MAC implementation.
//!
//! Usage
//! -----
//! ```rust
//! let aes_mux = crate::components::ieee802154::MuxAes128ccmComponent::new(
//!     &base_peripherals.ecb,
//! )
//!  .finalize(crate::components::mux_aes128ccm_component_static!(
//!     nrf52840::aes::AesECB
//! ));
//!
//! let (radio, mux_mac) = crate::components::ieee802154::Ieee802154Component::new(
//!     board_kernel,
//!     crate::capsules_extra::ieee802154::DRIVER_NUM,
//!     &nrf52::ieee802154_radio::RADIO,
//!     aes_mux,
//!     PAN_ID,
//!     SRC_MAC,
//!     deferred_caller,
//! )
//! .finalize(crate::components::ieee802154_component_static!(
//!     nrf52::ieee802154_radio::Radio,
//!     nrf52::aes::AesECB<'static>
//! ));
//! ```

use crate::capsules_core::virtualizers::virtual_aes_ccm::MuxAES128CCM;
use crate::capsules_extra::ieee802154::device::MacDevice;
use crate::capsules_extra::ieee802154::mac::{AwakeMac, Mac};
use core::mem::MaybeUninit;
use crate::kernel::capabilities::MemoryAllocationCapability;
use crate::kernel::component::Component;
use crate::kernel::hil::radio::{self, MAX_BUF_SIZE};
use crate::kernel::hil::symmetric_encryption::{self, AES, AES128, AESCBC, AESCCM, AESCtr, AESECB};

// This buffer is used as an intermediate buffer for AES CCM encryption. An
// upper bound on the required size is `3 * BLOCK_SIZE + radio::MAX_BUF_SIZE`.
pub const CRYPT_SIZE: usize = 3 * symmetric_encryption::AES_BLOCK_SIZE + radio::MAX_BUF_SIZE;

#[macro_export]
macro_rules! mux_aes128ccm_component_static {
    ($A:ty $(,)?) => {{ crate::kernel::static_buf!(crate::capsules_core::virtualizers::virtual_aes_ccm::MuxAES128CCM<'static, $A>) }};
}

pub type MuxAes128ccmComponentType<A> = MuxAES128CCM<'static, A>;

pub struct MuxAes128ccmComponent<A: 'static + AES<'static, AES128> + AESCtr + AESCBC + AESECB> {
    aes: &'static A,
}

impl<A: 'static + AES<'static, AES128> + AESCtr + AESCBC + AESECB> MuxAes128ccmComponent<A> {
    pub fn new(aes: &'static A) -> Self {
        Self { aes }
    }
}

impl<A: 'static + AES<'static, AES128> + AESCtr + AESCBC + AESECB> Component
    for MuxAes128ccmComponent<A>
{
    type StaticInput = &'static mut MaybeUninit<MuxAES128CCM<'static, A>>;
    type Output = &'static MuxAES128CCM<'static, A>;

    fn finalize(self, static_buffer: Self::StaticInput) -> Self::Output {
        let aes_mux = static_buffer.write(MuxAES128CCM::new(self.aes));
        crate::kernel::deferred_call::DeferredCallClient::register(aes_mux);
        self.aes.set_client(aes_mux);

        aes_mux
    }
}

// Setup static space for the objects.
#[macro_export]
macro_rules! ieee802154_component_static {
    ($R:ty, $A:ty $(,)?) => {{
        let virtual_aes = crate::kernel::static_buf!(
            crate::capsules_core::virtualizers::virtual_aes_ccm::VirtualAES128CCM<'static, $A>
        );
        let awake_mac = crate::kernel::static_buf!(crate::capsules_extra::ieee802154::mac::AwakeMac<'static, $R>);
        let framer = crate::kernel::static_buf!(
            crate::capsules_extra::ieee802154::framer::Framer<
                'static,
                crate::capsules_extra::ieee802154::mac::AwakeMac<'static, $R>,
                crate::capsules_core::virtualizers::virtual_aes_ccm::VirtualAES128CCM<'static, $A>,
            >
        );

        let mux_mac = crate::kernel::static_buf!(
            crate::capsules_extra::ieee802154::virtual_mac::MuxMac<
                'static,
                crate::capsules_extra::ieee802154::framer::Framer<
                    'static,
                    crate::capsules_extra::ieee802154::mac::AwakeMac<'static, $R>,
                    crate::capsules_core::virtualizers::virtual_aes_ccm::VirtualAES128CCM<'static, $A>,
                >,
            >
        );
        let mac_user = crate::kernel::static_buf!(
            crate::capsules_extra::ieee802154::virtual_mac::MacUser<
                'static,
                crate::capsules_extra::ieee802154::framer::Framer<
                    'static,
                    crate::capsules_extra::ieee802154::mac::AwakeMac<'static, $R>,
                    crate::capsules_core::virtualizers::virtual_aes_ccm::VirtualAES128CCM<'static, $A>,
                >,
            >
        );
        let radio_driver = crate::kernel::static_buf!(
            crate::capsules_extra::ieee802154::RadioDriver<
                'static,
                crate::capsules_extra::ieee802154::virtual_mac::MacUser<
                    'static,
                    crate::capsules_extra::ieee802154::framer::Framer<
                        'static,
                        crate::capsules_extra::ieee802154::mac::AwakeMac<'static, $R>,
                        crate::capsules_core::virtualizers::virtual_aes_ccm::VirtualAES128CCM<'static, $A>,
                    >,
                >,
            >
        );

        let radio_buf = crate::kernel::static_buf!([u8; crate::kernel::hil::radio::MAX_BUF_SIZE]);
        let radio_rx_buf = crate::kernel::static_buf!([u8; crate::kernel::hil::radio::MAX_BUF_SIZE]);
        let crypt_buf = crate::kernel::static_buf!([u8; crate::components::ieee802154::CRYPT_SIZE]);
        let radio_rx_crypt_buf = crate::kernel::static_buf!([u8; crate::kernel::hil::radio::MAX_BUF_SIZE]);

        (
            virtual_aes,
            awake_mac,
            framer,
            mux_mac,
            mac_user,
            radio_driver,
            radio_buf,
            radio_rx_buf,
            crypt_buf,
            radio_rx_crypt_buf,
        )
    }};
}

pub type Ieee802154ComponentType<R, A> = crate::capsules_extra::ieee802154::RadioDriver<
    'static,
    crate::capsules_extra::ieee802154::virtual_mac::MacUser<
        'static,
        crate::capsules_extra::ieee802154::framer::Framer<
            'static,
            crate::capsules_extra::ieee802154::mac::AwakeMac<'static, R>,
            crate::capsules_core::virtualizers::virtual_aes_ccm::VirtualAES128CCM<'static, A>,
        >,
    >,
>;

pub type Ieee802154ComponentMacDeviceType<R, A> = crate::capsules_extra::ieee802154::framer::Framer<
    'static,
    crate::capsules_extra::ieee802154::mac::AwakeMac<'static, R>,
    crate::capsules_core::virtualizers::virtual_aes_ccm::VirtualAES128CCM<'static, A>,
>;

pub struct Ieee802154Component<
    R: 'static + crate::kernel::hil::radio::Radio<'static>,
    A: 'static + AES<'static, AES128> + AESCtr + AESCBC + AESECB,
    CAP: MemoryAllocationCapability + 'static,
> {
    board_kernel: &'static crate::kernel::Kernel,
    driver_num: usize,
    radio: &'static R,
    aes_mux: &'static MuxAES128CCM<'static, A>,
    pan_id: crate::capsules_extra::net::ieee802154::PanID,
    short_addr: u16,
    long_addr: [u8; 8],
    mem_cap: CAP,
}

impl<
    R: 'static + crate::kernel::hil::radio::Radio<'static>,
    A: 'static + AES<'static, AES128> + AESCtr + AESCBC + AESECB,
    CAP: MemoryAllocationCapability + 'static,
> Ieee802154Component<R, A, CAP>
{
    pub fn new(
        board_kernel: &'static crate::kernel::Kernel,
        driver_num: usize,
        radio: &'static R,
        aes_mux: &'static MuxAES128CCM<'static, A>,
        pan_id: crate::capsules_extra::net::ieee802154::PanID,
        short_addr: u16,
        long_addr: [u8; 8],
        mem_cap: CAP,
    ) -> Self {
        Self {
            board_kernel,
            driver_num,
            radio,
            aes_mux,
            pan_id,
            short_addr,
            long_addr,
            mem_cap,
        }
    }
}

impl<
    R: 'static + crate::kernel::hil::radio::Radio<'static>,
    A: 'static + AES<'static, AES128> + AESCtr + AESCBC + AESECB,
    CAP: MemoryAllocationCapability + 'static,
> Component for Ieee802154Component<R, A, CAP>
{
    type StaticInput = (
        &'static mut MaybeUninit<
            crate::capsules_core::virtualizers::virtual_aes_ccm::VirtualAES128CCM<'static, A>,
        >,
        &'static mut MaybeUninit<crate::capsules_extra::ieee802154::mac::AwakeMac<'static, R>>,
        &'static mut MaybeUninit<
            crate::capsules_extra::ieee802154::framer::Framer<
                'static,
                AwakeMac<'static, R>,
                crate::capsules_core::virtualizers::virtual_aes_ccm::VirtualAES128CCM<'static, A>,
            >,
        >,
        &'static mut MaybeUninit<
            crate::capsules_extra::ieee802154::virtual_mac::MuxMac<
                'static,
                crate::capsules_extra::ieee802154::framer::Framer<
                    'static,
                    AwakeMac<'static, R>,
                    crate::capsules_core::virtualizers::virtual_aes_ccm::VirtualAES128CCM<'static, A>,
                >,
            >,
        >,
        &'static mut MaybeUninit<
            crate::capsules_extra::ieee802154::virtual_mac::MacUser<
                'static,
                crate::capsules_extra::ieee802154::framer::Framer<
                    'static,
                    AwakeMac<'static, R>,
                    crate::capsules_core::virtualizers::virtual_aes_ccm::VirtualAES128CCM<'static, A>,
                >,
            >,
        >,
        &'static mut MaybeUninit<
            crate::capsules_extra::ieee802154::RadioDriver<
                'static,
                crate::capsules_extra::ieee802154::virtual_mac::MacUser<
                    'static,
                    crate::capsules_extra::ieee802154::framer::Framer<
                        'static,
                        AwakeMac<'static, R>,
                        crate::capsules_core::virtualizers::virtual_aes_ccm::VirtualAES128CCM<'static, A>,
                    >,
                >,
            >,
        >,
        &'static mut MaybeUninit<[u8; radio::MAX_BUF_SIZE]>,
        &'static mut MaybeUninit<[u8; radio::MAX_BUF_SIZE]>,
        &'static mut MaybeUninit<[u8; CRYPT_SIZE]>,
        &'static mut MaybeUninit<[u8; radio::MAX_BUF_SIZE]>,
    );
    type Output = (
        &'static crate::capsules_extra::ieee802154::RadioDriver<
            'static,
            crate::capsules_extra::ieee802154::virtual_mac::MacUser<
                'static,
                crate::capsules_extra::ieee802154::framer::Framer<
                    'static,
                    AwakeMac<'static, R>,
                    crate::capsules_core::virtualizers::virtual_aes_ccm::VirtualAES128CCM<'static, A>,
                >,
            >,
        >,
        &'static crate::capsules_extra::ieee802154::virtual_mac::MuxMac<
            'static,
            crate::capsules_extra::ieee802154::framer::Framer<
                'static,
                AwakeMac<'static, R>,
                crate::capsules_core::virtualizers::virtual_aes_ccm::VirtualAES128CCM<'static, A>,
            >,
        >,
    );

    fn finalize(self, static_buffer: Self::StaticInput) -> Self::Output {
        let crypt_buf = static_buffer.8.write([0; CRYPT_SIZE]);
        let aes_ccm = static_buffer.0.write(
            crate::capsules_core::virtualizers::virtual_aes_ccm::VirtualAES128CCM::new(
                self.aes_mux,
                crypt_buf,
            ),
        );
        aes_ccm.setup();

        // Keeps the radio on permanently; pass-through layer.
        let radio_rx_buf = static_buffer.7.write([0; radio::MAX_BUF_SIZE]);
        let awake_mac = static_buffer.1.write(AwakeMac::new(self.radio));
        self.radio.set_transmit_client(awake_mac);
        self.radio.set_receive_client(awake_mac);
        self.radio.set_receive_buffer(radio_rx_buf);

        let radio_rx_crypt_buf = static_buffer.9.write([0; MAX_BUF_SIZE]);

        let mac_device = static_buffer
            .2
            .write(crate::capsules_extra::ieee802154::framer::Framer::new(
                awake_mac,
                aes_ccm,
                crate::kernel::utilities::leasable_buffer::SubSliceMut::new(radio_rx_crypt_buf),
            ));
        AESCCM::set_client(aes_ccm, mac_device);
        awake_mac.set_transmit_client(mac_device);
        awake_mac.set_receive_client(mac_device);
        awake_mac.set_config_client(mac_device);

        let mux_mac = static_buffer
            .3
            .write(crate::capsules_extra::ieee802154::virtual_mac::MuxMac::new(
                mac_device,
            ));
        mac_device.set_transmit_client(mux_mac);
        mac_device.set_receive_client(mux_mac);

        let userspace_mac =
            static_buffer
                .4
                .write(crate::capsules_extra::ieee802154::virtual_mac::MacUser::new(
                    mux_mac,
                ));
        mux_mac.add_user(userspace_mac);

        let radio_buffer = static_buffer.6.write([0; radio::MAX_BUF_SIZE]);
        let radio_driver = static_buffer
            .5
            .write(crate::capsules_extra::ieee802154::RadioDriver::new(
                userspace_mac,
                self.board_kernel
                    .create_grant(self.driver_num, &self.mem_cap),
                radio_buffer,
            ));
        crate::kernel::deferred_call::DeferredCallClient::register(radio_driver);

        mac_device.set_key_procedure(radio_driver);
        mac_device.set_device_procedure(radio_driver);
        userspace_mac.set_transmit_client(radio_driver);
        userspace_mac.set_receive_client(radio_driver);
        userspace_mac.set_pan(self.pan_id);
        userspace_mac.set_address(self.short_addr);
        userspace_mac.set_address_long(self.long_addr);

        (radio_driver, mux_mac)
    }
}

// IEEE 802.15.4 RAW DRIVER

// Setup static space for the objects.
#[macro_export]
macro_rules! ieee802154_raw_component_static {
    ($R:ty $(,)?) => {{
        let radio_driver =
            crate::kernel::static_buf!(crate::capsules_extra::ieee802154::phy_driver::RadioDriver<$R>);
        let tx_buffer = crate::kernel::static_buf!([u8; crate::kernel::hil::radio::MAX_BUF_SIZE]);
        let rx_buffer = crate::kernel::static_buf!([u8; crate::kernel::hil::radio::MAX_BUF_SIZE]);

        (radio_driver, tx_buffer, rx_buffer)
    }};
}

pub type Ieee802154RawComponentType<R> =
    crate::capsules_extra::ieee802154::phy_driver::RadioDriver<'static, R>;

pub struct Ieee802154RawComponent<
    R: 'static + crate::kernel::hil::radio::Radio<'static>,
    CAP: MemoryAllocationCapability + 'static,
> {
    board_kernel: &'static crate::kernel::Kernel,
    driver_num: usize,
    radio: &'static R,
    mem_cap: CAP,
}

impl<R: 'static + crate::kernel::hil::radio::Radio<'static>, CAP: MemoryAllocationCapability + 'static>
    Ieee802154RawComponent<R, CAP>
{
    pub fn new(
        board_kernel: &'static crate::kernel::Kernel,
        driver_num: usize,
        radio: &'static R,
        mem_cap: CAP,
    ) -> Self {
        Self {
            board_kernel,
            driver_num,
            radio,
            mem_cap,
        }
    }
}

impl<R: 'static + crate::kernel::hil::radio::Radio<'static>, CAP: MemoryAllocationCapability + 'static>
    Component for Ieee802154RawComponent<R, CAP>
{
    type StaticInput = (
        &'static mut MaybeUninit<crate::capsules_extra::ieee802154::phy_driver::RadioDriver<'static, R>>,
        &'static mut MaybeUninit<[u8; radio::MAX_BUF_SIZE]>,
        &'static mut MaybeUninit<[u8; radio::MAX_BUF_SIZE]>,
    );
    type Output = &'static crate::capsules_extra::ieee802154::phy_driver::RadioDriver<'static, R>;

    fn finalize(self, static_buffer: Self::StaticInput) -> Self::Output {
        let tx_buffer = static_buffer.1.write([0; MAX_BUF_SIZE]);
        let radio_rx_buf = static_buffer.2.write([0; radio::MAX_BUF_SIZE]);

        let radio_driver =
            static_buffer
                .0
                .write(crate::capsules_extra::ieee802154::phy_driver::RadioDriver::new(
                    self.radio,
                    self.board_kernel
                        .create_grant(self.driver_num, &self.mem_cap),
                    tx_buffer,
                ));

        self.radio.set_transmit_client(radio_driver);
        self.radio.set_receive_client(radio_driver);
        self.radio.set_receive_buffer(radio_rx_buf);

        radio_driver
    }
}
