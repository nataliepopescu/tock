// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2024.

//! Component for creating a process checking machine.

use core::mem::MaybeUninit;
use crate::kernel::component::Component;

#[macro_export]
macro_rules! process_checker_machine_component_static {
    () => {{ crate::kernel::static_buf!(crate::kernel::process::ProcessCheckerMachine) }};
}

pub type ProcessCheckerMachineComponentType = crate::kernel::process::ProcessCheckerMachine;

pub struct ProcessCheckerMachineComponent {
    policy: &'static dyn crate::kernel::process_checker::AppCredentialsPolicy<'static>,
}

impl ProcessCheckerMachineComponent {
    pub fn new(policy: &'static dyn crate::kernel::process_checker::AppCredentialsPolicy) -> Self {
        Self { policy }
    }
}

impl Component for ProcessCheckerMachineComponent {
    type StaticInput = &'static mut MaybeUninit<crate::kernel::process::ProcessCheckerMachine>;

    type Output = &'static crate::kernel::process::ProcessCheckerMachine;

    fn finalize(self, s: Self::StaticInput) -> Self::Output {
        let checker = s.write(crate::kernel::process::ProcessCheckerMachine::new(self.policy));

        self.policy.set_client(checker);
        checker
    }
}
