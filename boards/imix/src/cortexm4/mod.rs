// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Shared implementations for ARM Cortex-M4 MCUs.


use core::fmt::Write;

pub mod mpu {
    use crate::kernel::utilities::StaticRef;

    pub type MPU = crate::cortexm::mpu::MPU<8, 32>;

    const MPU_BASE_ADDRESS: StaticRef<crate::cortexm::mpu::MpuRegisters> =
        unsafe { StaticRef::new(0xE000ED90 as *const crate::cortexm::mpu::MpuRegisters) };

    pub unsafe fn new() -> MPU {
        unsafe { MPU::new(MPU_BASE_ADDRESS) }
    }
}

pub use crate::cortexm::CortexMVariant;
pub use crate::cortexm::dwt;
pub use crate::cortexm::initialize_ram_jump_to_main;
pub use crate::cortexm::nvic;
pub use crate::cortexm::scb;
pub use crate::cortexm::support;
pub use crate::cortexm::systick;
pub use crate::cortexm::thread_id;
pub use crate::cortexm::unhandled_interrupt;

// Enum with no variants to ensure that this type is not instantiable. It is
// only used to pass architecture-specific constants and functions via the
// `CortexMVariant` trait.
pub enum CortexM4 {}

impl crate::cortexm::CortexMVariant for CortexM4 {
    const GENERIC_ISR: unsafe extern "C" fn() = crate::cortexv7m::generic_isr_arm_v7m;
    const SYSTICK_HANDLER: unsafe extern "C" fn() = crate::cortexv7m::systick_handler_arm_v7m;
    const SVC_HANDLER: unsafe extern "C" fn() = crate::cortexv7m::svc_handler_arm_v7m;
    const HARD_FAULT_HANDLER: unsafe extern "C" fn() = crate::cortexv7m::hard_fault_handler_arm_v7m;

    #[cfg(all(target_arch = "arm", target_os = "none"))]
    unsafe fn switch_to_user(
        user_stack: *const usize,
        process_regs: &mut [usize; 8],
    ) -> *const usize {
        unsafe { crate::cortexv7m::switch_to_user_arm_v7m(user_stack, process_regs) }
    }

    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    unsafe fn switch_to_user(
        _user_stack: *const usize,
        _process_regs: &mut [usize; 8],
    ) -> *const usize {
        unimplemented!()
    }

    #[inline]
    unsafe fn print_cortexm_state(writer: &mut dyn Write) {
        crate::cortexm::print_cortexm_state(writer)
    }
}

pub mod syscall {
    pub type SysCall = crate::cortexm::syscall::SysCall<crate::cortexm4::CortexM4>;
}
