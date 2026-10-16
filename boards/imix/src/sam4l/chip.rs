// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Interrupt mapping and DMA channel setup.

use crate::sam4l::pm;

use core::fmt::Write;
use crate::cortexm4::{CortexM4, CortexMVariant};
use crate::kernel::platform::chip::{Chip, InterruptService};

pub struct Sam4l<I: InterruptService + 'static> {
    mpu: crate::cortexm4::mpu::MPU,
    userspace_kernel_boundary: crate::cortexm4::syscall::SysCall,
    pub pm: &'static crate::sam4l::pm::PowerManager,
    interrupt_service: &'static I,
}

impl<I: InterruptService + 'static> Sam4l<I> {
    pub unsafe fn new(pm: &'static crate::sam4l::pm::PowerManager, interrupt_service: &'static I) -> Self {
        Self {
            mpu: crate::cortexm4::mpu::new(),
            userspace_kernel_boundary: crate::cortexm4::syscall::SysCall::new(),
            pm,
            interrupt_service,
        }
    }
}

/// This struct, when initialized, instantiates all peripheral drivers for the apollo3.
///
/// If a board wishes to use only a subset of these peripherals, this
/// should not be used or imported, and a modified version should be
/// constructed manually in main.rs.
pub struct Sam4lDefaultPeripherals {
    pub acifc: crate::sam4l::acifc::Acifc<'static>,
    pub adc: crate::sam4l::adc::Adc<'static>,
    pub aes: crate::sam4l::aes::Aes<'static>,
    pub ast: crate::sam4l::ast::Ast<'static>,
    pub crccu: crate::sam4l::crccu::Crccu<'static>,
    pub dac: crate::sam4l::dac::Dac,
    pub dma_channels: [crate::sam4l::dma::DMAChannel; 16],
    pub eic: crate::sam4l::eic::Eic<'static>,
    pub flash_controller: crate::sam4l::flashcalw::FLASHCALW,
    pub gloc: crate::sam4l::gloc::Gloc,
    pub pa: crate::sam4l::gpio::Port<'static>,
    pub pb: crate::sam4l::gpio::Port<'static>,
    pub pc: crate::sam4l::gpio::Port<'static>,
    pub i2c0: crate::sam4l::i2c::I2CHw<'static>,
    pub i2c1: crate::sam4l::i2c::I2CHw<'static>,
    pub i2c2: crate::sam4l::i2c::I2CHw<'static>,
    pub i2c3: crate::sam4l::i2c::I2CHw<'static>,
    pub spi: crate::sam4l::spi::SpiHw<'static>,
    pub trng: crate::sam4l::trng::Trng<'static>,
    pub usart0: crate::sam4l::usart::USART<'static>,
    pub usart1: crate::sam4l::usart::USART<'static>,
    pub usart2: crate::sam4l::usart::USART<'static>,
    pub usart3: crate::sam4l::usart::USART<'static>,
    pub usbc: crate::sam4l::usbc::Usbc<'static>,
}

impl Sam4lDefaultPeripherals {
    pub fn new(pm: &'static crate::sam4l::pm::PowerManager) -> Self {
        use crate::sam4l::dma::{DMAChannel, DMAChannelNum};
        Self {
            acifc: crate::sam4l::acifc::Acifc::new(),
            adc: crate::sam4l::adc::Adc::new(crate::sam4l::dma::DMAPeripheral::ADCIFE_RX, pm),
            aes: crate::sam4l::aes::Aes::new(),
            ast: crate::sam4l::ast::Ast::new(),
            crccu: crate::sam4l::crccu::Crccu::new(crate::sam4l::crccu::BASE_ADDRESS),
            dac: crate::sam4l::dac::Dac::new(),
            dma_channels: [
                DMAChannel::new(DMAChannelNum::DMAChannel00),
                DMAChannel::new(DMAChannelNum::DMAChannel01),
                DMAChannel::new(DMAChannelNum::DMAChannel02),
                DMAChannel::new(DMAChannelNum::DMAChannel03),
                DMAChannel::new(DMAChannelNum::DMAChannel04),
                DMAChannel::new(DMAChannelNum::DMAChannel05),
                DMAChannel::new(DMAChannelNum::DMAChannel06),
                DMAChannel::new(DMAChannelNum::DMAChannel07),
                DMAChannel::new(DMAChannelNum::DMAChannel08),
                DMAChannel::new(DMAChannelNum::DMAChannel09),
                DMAChannel::new(DMAChannelNum::DMAChannel10),
                DMAChannel::new(DMAChannelNum::DMAChannel11),
                DMAChannel::new(DMAChannelNum::DMAChannel12),
                DMAChannel::new(DMAChannelNum::DMAChannel13),
                DMAChannel::new(DMAChannelNum::DMAChannel14),
                DMAChannel::new(DMAChannelNum::DMAChannel15),
            ],
            eic: crate::sam4l::eic::Eic::new(),
            flash_controller: crate::sam4l::flashcalw::FLASHCALW::new(
                crate::sam4l::pm::HSBClock::FLASHCALW,
                crate::sam4l::pm::HSBClock::FLASHCALWP,
                crate::sam4l::pm::PBBClock::FLASHCALW,
            ),
            gloc: crate::sam4l::gloc::Gloc::new(),
            pa: crate::sam4l::gpio::Port::new_port_a(),
            pb: crate::sam4l::gpio::Port::new_port_b(),
            pc: crate::sam4l::gpio::Port::new_port_c(),
            i2c0: crate::sam4l::i2c::I2CHw::new_i2c0(pm),
            i2c1: crate::sam4l::i2c::I2CHw::new_i2c1(pm),
            i2c2: crate::sam4l::i2c::I2CHw::new_i2c2(pm),
            i2c3: crate::sam4l::i2c::I2CHw::new_i2c3(pm),
            spi: crate::sam4l::spi::SpiHw::new(pm),
            trng: crate::sam4l::trng::Trng::new(),
            usart0: crate::sam4l::usart::USART::new_usart0(pm),
            usart1: crate::sam4l::usart::USART::new_usart1(pm),
            usart2: crate::sam4l::usart::USART::new_usart2(pm),
            usart3: crate::sam4l::usart::USART::new_usart3(pm),
            usbc: crate::sam4l::usbc::Usbc::new(pm),
        }
    }

    pub fn setup_circular_deps(&'static self) {
        use crate::sam4l::dma;
        self.usart0
            .set_dma(&self.dma_channels[0], &self.dma_channels[1]);
        self.dma_channels[0].initialize(&self.usart0, dma::DMAWidth::Width8Bit);
        self.dma_channels[1].initialize(&self.usart0, dma::DMAWidth::Width8Bit);

        self.usart1
            .set_dma(&self.dma_channels[2], &self.dma_channels[3]);
        self.dma_channels[2].initialize(&self.usart1, dma::DMAWidth::Width8Bit);
        self.dma_channels[3].initialize(&self.usart1, dma::DMAWidth::Width8Bit);

        self.usart2
            .set_dma(&self.dma_channels[4], &self.dma_channels[5]);
        self.dma_channels[4].initialize(&self.usart2, dma::DMAWidth::Width8Bit);
        self.dma_channels[5].initialize(&self.usart2, dma::DMAWidth::Width8Bit);

        self.usart3
            .set_dma(&self.dma_channels[6], &self.dma_channels[7]);
        self.dma_channels[6].initialize(&self.usart3, dma::DMAWidth::Width8Bit);
        self.dma_channels[7].initialize(&self.usart3, dma::DMAWidth::Width8Bit);

        self.spi
            .set_dma(&self.dma_channels[8], &self.dma_channels[9]);
        self.dma_channels[8].initialize(&self.spi, dma::DMAWidth::Width8Bit);
        self.dma_channels[9].initialize(&self.spi, dma::DMAWidth::Width8Bit);

        self.i2c0.set_dma(&self.dma_channels[10]);
        self.dma_channels[10].initialize(&self.i2c0, dma::DMAWidth::Width8Bit);

        self.i2c1.set_dma(&self.dma_channels[11]);
        self.dma_channels[11].initialize(&self.i2c1, dma::DMAWidth::Width8Bit);

        self.i2c2.set_dma(&self.dma_channels[12]);
        self.dma_channels[12].initialize(&self.i2c2, dma::DMAWidth::Width8Bit);

        self.adc.set_dma(&self.dma_channels[13]);
        self.dma_channels[13].initialize(&self.adc, dma::DMAWidth::Width16Bit);

        // REGISTER ALL PERIPHERALS WITH DEFERRED CALLS
        crate::kernel::deferred_call::DeferredCallClient::register(&self.crccu);
        crate::kernel::deferred_call::DeferredCallClient::register(&self.flash_controller);
        crate::kernel::deferred_call::DeferredCallClient::register(&self.usart0);
        crate::kernel::deferred_call::DeferredCallClient::register(&self.usart1);
        crate::kernel::deferred_call::DeferredCallClient::register(&self.usart2);
        crate::kernel::deferred_call::DeferredCallClient::register(&self.usart3);
    }
}
impl InterruptService for Sam4lDefaultPeripherals {
    fn service_interrupt(&self, interrupt: u32) -> bool {
        use crate::sam4l::nvic;
        match interrupt {
            nvic::ASTALARM => self.ast.handle_interrupt(),

            nvic::USART0 => self.usart0.handle_interrupt(),
            nvic::USART1 => self.usart1.handle_interrupt(),
            nvic::USART2 => self.usart2.handle_interrupt(),
            nvic::USART3 => self.usart3.handle_interrupt(),

            nvic::PDCA0 => self.dma_channels[0].handle_interrupt(),
            nvic::PDCA1 => self.dma_channels[1].handle_interrupt(),
            nvic::PDCA2 => self.dma_channels[2].handle_interrupt(),
            nvic::PDCA3 => self.dma_channels[3].handle_interrupt(),
            nvic::PDCA4 => self.dma_channels[4].handle_interrupt(),
            nvic::PDCA5 => self.dma_channels[5].handle_interrupt(),
            nvic::PDCA6 => self.dma_channels[6].handle_interrupt(),
            nvic::PDCA7 => self.dma_channels[7].handle_interrupt(),
            nvic::PDCA8 => self.dma_channels[8].handle_interrupt(),
            nvic::PDCA9 => self.dma_channels[9].handle_interrupt(),
            nvic::PDCA10 => self.dma_channels[10].handle_interrupt(),
            nvic::PDCA11 => self.dma_channels[11].handle_interrupt(),
            nvic::PDCA12 => self.dma_channels[12].handle_interrupt(),
            nvic::PDCA13 => self.dma_channels[13].handle_interrupt(),
            nvic::PDCA14 => self.dma_channels[14].handle_interrupt(),
            nvic::PDCA15 => self.dma_channels[15].handle_interrupt(),

            nvic::CRCCU => self.crccu.handle_interrupt(),
            nvic::USBC => self.usbc.handle_interrupt(),

            nvic::GPIO0 => self.pa.handle_interrupt(),
            nvic::GPIO1 => self.pa.handle_interrupt(),
            nvic::GPIO2 => self.pa.handle_interrupt(),
            nvic::GPIO3 => self.pa.handle_interrupt(),
            nvic::GPIO4 => self.pb.handle_interrupt(),
            nvic::GPIO5 => self.pb.handle_interrupt(),
            nvic::GPIO6 => self.pb.handle_interrupt(),
            nvic::GPIO7 => self.pb.handle_interrupt(),
            nvic::GPIO8 => self.pc.handle_interrupt(),
            nvic::GPIO9 => self.pc.handle_interrupt(),
            nvic::GPIO10 => self.pc.handle_interrupt(),
            nvic::GPIO11 => self.pc.handle_interrupt(),

            nvic::SPI => self.spi.handle_interrupt(),

            nvic::TWIM0 => self.i2c0.handle_interrupt(),
            nvic::TWIM1 => self.i2c1.handle_interrupt(),
            nvic::TWIM2 => self.i2c2.handle_interrupt(),
            nvic::TWIM3 => self.i2c3.handle_interrupt(),
            nvic::TWIS0 => self.i2c0.handle_slave_interrupt(),
            nvic::TWIS1 => self.i2c1.handle_slave_interrupt(),

            nvic::HFLASHC => self.flash_controller.handle_interrupt(),
            nvic::ADCIFE => self.adc.handle_interrupt(),
            nvic::DACC => self.dac.handle_interrupt(),
            nvic::ACIFC => self.acifc.handle_interrupt(),

            nvic::TRNG => self.trng.handle_interrupt(),
            nvic::AESA => self.aes.handle_interrupt(),

            nvic::EIC1 => self.eic.handle_interrupt(&crate::sam4l::eic::Line::Ext1),
            nvic::EIC2 => self.eic.handle_interrupt(&crate::sam4l::eic::Line::Ext2),
            nvic::EIC3 => self.eic.handle_interrupt(&crate::sam4l::eic::Line::Ext3),
            nvic::EIC4 => self.eic.handle_interrupt(&crate::sam4l::eic::Line::Ext4),
            nvic::EIC5 => self.eic.handle_interrupt(&crate::sam4l::eic::Line::Ext5),
            nvic::EIC6 => self.eic.handle_interrupt(&crate::sam4l::eic::Line::Ext6),
            nvic::EIC7 => self.eic.handle_interrupt(&crate::sam4l::eic::Line::Ext7),
            nvic::EIC8 => self.eic.handle_interrupt(&crate::sam4l::eic::Line::Ext8),
            _ => return false,
        }
        true
    }
}

impl<I: InterruptService + 'static> Chip for Sam4l<I> {
    type MPU = crate::cortexm4::mpu::MPU;
    type UserspaceKernelBoundary = crate::cortexm4::syscall::SysCall;
    type ThreadIdProvider = crate::cortexm4::thread_id::CortexMThreadIdProvider;

    fn init() {
        crate::cortexm4::nvic::disable_all();
        crate::cortexm4::nvic::clear_all_pending();
        crate::cortexm4::nvic::enable_all();
    }

    fn service_pending_interrupts(&self) {
        while let Some(interrupt) = crate::cortexm4::nvic::next_pending() {
            match self.interrupt_service.service_interrupt(interrupt) {
                true => {}
                false => panic!("unhandled interrupt"),
            }
            let n = crate::cortexm4::nvic::Nvic::new(interrupt);
            n.clear_pending();
            n.enable();
        }
    }

    fn has_pending_interrupts(&self) -> bool {
        crate::cortexm4::nvic::has_pending()
    }

    fn mpu(&self) -> &crate::cortexm4::mpu::MPU {
        &self.mpu
    }

    fn userspace_kernel_boundary(&self) -> &crate::cortexm4::syscall::SysCall {
        &self.userspace_kernel_boundary
    }

    fn sleep(&self) {
        if pm::deep_sleep_ready() {
            unsafe {
                crate::cortexm4::scb::set_sleepdeep();
            }
        } else {
            unsafe {
                crate::cortexm4::scb::unset_sleepdeep();
            }
        }

        unsafe {
            crate::cortexm4::support::wfi();
        }
    }

    fn with_interrupts_disabled<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        crate::cortexm4::support::with_interrupts_disabled(f)
    }

    unsafe fn print_state(_this: Option<&Self>, writer: &mut dyn Write) {
        CortexM4::print_cortexm_state(writer);
    }
}
