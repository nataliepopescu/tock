// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2025.

//! Component for SignatureVerifyInMemoryKeys.

use core::mem::MaybeUninit;
use crate::kernel::component::Component;
use crate::kernel::deferred_call::DeferredCallClient;

#[macro_export]
macro_rules! signature_verify_in_memory_keys_component_static {
    ($S:ty, $NUM_KEYS:expr_2021, $KL:expr_2021, $HL:expr_2021, $SL:expr_2021 $(,)?) => {{
        let verifier = crate::kernel::static_buf!(
            crate::capsules_extra::signature_verify_in_memory_keys::SignatureVerifyInMemoryKeys<
                'static,
                $S,
                $NUM_KEYS,
                $KL,
                $HL,
                $SL,
            >
        );

        verifier
    }};
}

pub type SignatureVerifyInMemoryKeysComponentType<
    S,
    const NUM_KEYS: usize,
    const KL: usize,
    const HL: usize,
    const SL: usize,
> = crate::capsules_extra::signature_verify_in_memory_keys::SignatureVerifyInMemoryKeys<
    'static,
    S,
    NUM_KEYS,
    KL,
    HL,
    SL,
>;

pub struct SignatureVerifyInMemoryKeysComponent<
    S: crate::kernel::hil::public_key_crypto::signature::SignatureVerify<'static, HL, SL>
        + crate::kernel::hil::public_key_crypto::keys::SetKeyBySlice<'static, KL>
        + 'static,
    const NUM_KEYS: usize,
    const KL: usize,
    const HL: usize,
    const SL: usize,
> {
    verifier: &'static S,
    keys: &'static mut [&'static mut [u8; KL]],
}

impl<
    S: crate::kernel::hil::public_key_crypto::signature::SignatureVerify<'static, HL, SL>
        + crate::kernel::hil::public_key_crypto::keys::SetKeyBySlice<'static, KL>
        + 'static,
    const NUM_KEYS: usize,
    const KL: usize,
    const HL: usize,
    const SL: usize,
> SignatureVerifyInMemoryKeysComponent<S, NUM_KEYS, KL, HL, SL>
{
    pub fn new(verifier: &'static S, keys: &'static mut [&'static mut [u8; KL]]) -> Self {
        Self { verifier, keys }
    }
}

impl<
    S: crate::kernel::hil::public_key_crypto::signature::SignatureVerify<'static, HL, SL>
        + crate::kernel::hil::public_key_crypto::keys::SetKeyBySlice<'static, KL>
        + 'static,
    const NUM_KEYS: usize,
    const KL: usize,
    const HL: usize,
    const SL: usize,
> Component for SignatureVerifyInMemoryKeysComponent<S, NUM_KEYS, KL, HL, SL>
{
    type StaticInput = &'static mut MaybeUninit<
        crate::capsules_extra::signature_verify_in_memory_keys::SignatureVerifyInMemoryKeys<
            'static,
            S,
            NUM_KEYS,
            KL,
            HL,
            SL,
        >,
    >;

    type Output =
        &'static crate::capsules_extra::signature_verify_in_memory_keys::SignatureVerifyInMemoryKeys<
            'static,
            S,
            NUM_KEYS,
            KL,
            HL,
            SL,
        >;

    fn finalize(self, s: Self::StaticInput) -> Self::Output {
        let verifier_multiple_keys = s.write(
            crate::capsules_extra::signature_verify_in_memory_keys::SignatureVerifyInMemoryKeys::new(
                self.verifier,
            ),
        );
        self.verifier.set_client(verifier_multiple_keys);

        for (i, k) in self.keys.iter_mut().enumerate() {
            let _ = verifier_multiple_keys.init_key(i, k);
        }
        verifier_multiple_keys.register();
        verifier_multiple_keys
    }
}
