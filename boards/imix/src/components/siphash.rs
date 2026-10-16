// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2023.

//! Components for SipHash hasher.

use core::mem::MaybeUninit;
use crate::kernel::component::Component;
use crate::kernel::deferred_call::DeferredCallClient;

// Setup static space for the objects.
#[macro_export]
macro_rules! siphasher24_component_static {
    ($(,)?) => {{ crate::kernel::static_buf!(crate::capsules_extra::sip_hash::SipHasher24) }};
}

pub type Siphasher24ComponentType = crate::capsules_extra::sip_hash::SipHasher24<'static>;

pub struct Siphasher24Component {}

impl Siphasher24Component {
    pub fn new() -> Siphasher24Component {
        Siphasher24Component {}
    }
}

impl Component for Siphasher24Component {
    type StaticInput = &'static mut MaybeUninit<crate::capsules_extra::sip_hash::SipHasher24<'static>>;
    type Output = &'static crate::capsules_extra::sip_hash::SipHasher24<'static>;

    fn finalize(self, s: Self::StaticInput) -> Self::Output {
        let sip_hash = s.write(crate::capsules_extra::sip_hash::SipHasher24::new());
        sip_hash.register();
        sip_hash
    }
}
