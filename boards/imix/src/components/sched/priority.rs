// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Component for a priority scheduler.
//!
//! This provides one Component, PriorityComponent.
//!
//! Usage
//! -----
//! ```rust
//! let scheduler =
//!     crate::components::priority::PriorityComponent::new(board_kernel)
//!         .finalize(crate::components::priority_component_static!());
//! ```

use crate::capsules_system::scheduler::priority::PrioritySched;
use core::mem::MaybeUninit;
use crate::kernel::capabilities::ProcessManagementCapability;
use crate::kernel::component::Component;

#[macro_export]
macro_rules! priority_component_static {
    ($CAP:ty $(,)?) => {{ crate::kernel::static_buf!(crate::capsules_system::scheduler::priority::PrioritySched<$CAP>) }};
}

pub type PriorityComponentType<CAP> = crate::capsules_system::scheduler::priority::PrioritySched<CAP>;

pub struct PriorityComponent<CAP: ProcessManagementCapability> {
    board_kernel: &'static crate::kernel::Kernel,
    cap: CAP,
}

impl<CAP: ProcessManagementCapability> PriorityComponent<CAP> {
    pub fn new(board_kernel: &'static crate::kernel::Kernel, cap: CAP) -> Self {
        Self { board_kernel, cap }
    }
}

impl<CAP: ProcessManagementCapability + 'static> Component for PriorityComponent<CAP> {
    type StaticInput = &'static mut MaybeUninit<PrioritySched<CAP>>;
    type Output = &'static mut PrioritySched<CAP>;

    fn finalize(self, static_buffer: Self::StaticInput) -> Self::Output {
        static_buffer.write(PrioritySched::new(self.board_kernel, self.cap))
    }
}
