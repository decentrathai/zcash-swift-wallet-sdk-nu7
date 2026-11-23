//! ZIP 316 Revision 0 encodings of Unified Addresses and Unified Viewing Keys.
//!
//! `zcash_keys 0.17` encodes Unified Addresses, UFVKs and UIVKs as ZIP 316 Revision 2
//! (`zu…`/`tu…`, `uvf…`, `uvi…`) unconditionally, and its shielded-only `zu` form drops the
//! transparent receiver. SDK 3.0.0 handed Kotlin the Revision 0 forms (`u1…`, `uview1…`,
//! `uivk1…` and their testnet equivalents), and downstream apps parse those prefixes, so this
//! build keeps returning Revision 0 strings until the upstream SDK makes the switch.
//!
//! Every conversion keeps all data items (receivers or viewing keys) and changes only the
//! revision. A container that carries metadata (such as an expiry height) or that ZIP 316 does
//! not allow in Revision 0 (for example a transparent-only address) has no Revision 0 form and is
//! returned in its Revision 2 encoding unchanged.
//!
//! Copied from the Android NU7 fork (zcash-android-wallet-sdk a99a745e,
//! `backend-lib/src/main/rust/r0.rs`) without its `tests` module: those tests live in
//! `crate::nu7_tests`, except `mixed_database_reads_back_one_string_per_address`, which needs the
//! `zcash_client_sqlite` `test-dependencies` feature. Android's `clippy.toml` (which disallows the
//! upstream Revision 2 encoders outside this module) is not part of this crate.

#![allow(clippy::disallowed_methods)]

use zcash_address::{
    ToAddress, ZcashAddress,
    unified::{self, Container, Encoding, Revision, Uitem},
};
use zcash_client_backend::{
    address::UnifiedAddress,
    keys::{UnifiedFullViewingKey, UnifiedIncomingViewingKey},
};
use zcash_protocol::consensus::{NetworkType, Parameters};

/// Re-encodes a Unified container string as Revision 0 when it has a Revision 0 form, and
/// returns it unchanged otherwise (including when it is already Revision 0 or does not parse as
/// `C`).
fn to_r0<C: Encoding + Container>(encoded: String) -> String {
    let Ok((net, revision, container)) = C::decode(&encoded) else {
        return encoded;
    };
    if revision == Revision::R0 {
        return encoded;
    }

    let mut data_items = Vec::with_capacity(container.items_as_parsed().len());
    for item in container.items_as_parsed() {
        match item {
            Uitem::Data(data) => data_items.push(Uitem::Data(data.clone())),
            Uitem::Metadata(_) => {
                tracing::warn!(
                    "Unified container carries metadata; keeping its Revision 2 encoding"
                );
                return encoded;
            }
        }
    }

    match C::try_from_items(Revision::R0, data_items) {
        Ok(r0) => r0.encode(&net),
        Err(e) => {
            tracing::warn!("Unified container has no Revision 0 form ({e}); keeping Revision 2");
            encoded
        }
    }
}

/// The string form of `ua` that SDK 3.0.0 returned: Revision 0, with every receiver
/// (including a transparent one, which the Revision 2 `zu` form would strip).
pub(crate) fn encode_ua<P: Parameters>(ua: &UnifiedAddress, params: &P) -> String {
    // zcash_keys 0.17.0 (stable) encodes at Revision 0 whenever Revision 0 can represent the
    // address, with every receiver; `to_r0` is then a no-op and stays as the guard.
    to_r0::<unified::Address>(ua.to_zcash_address(params.network_type()).encode())
}

/// The string form of `ufvk` that SDK 3.0.0 returned (Revision 0 `uview…`).
pub(crate) fn encode_ufvk<P: Parameters>(ufvk: &UnifiedFullViewingKey, params: &P) -> String {
    // zcash_keys 0.17.0: Revision 0 when representable, else Revision 2; an error means neither
    // revision can encode the key, which no key this SDK derives or imports can be.
    to_r0::<unified::Ufvk>(ufvk.encode(params).expect("a UFVK has a ZIP 316 encoding"))
}

/// The string form of `uivk` that SDK 3.0.0 returned (Revision 0 `uivk…`).
pub(crate) fn encode_uivk<P: Parameters>(uivk: &UnifiedIncomingViewingKey, params: &P) -> String {
    to_r0::<unified::Uivk>(uivk.encode(params).expect("a UIVK has a ZIP 316 encoding"))
}

/// The Revision 0 form of a Unified Address, UFVK or UIVK string; any other string (a
/// transparent, Sapling or TEX address, one that is already Revision 0, or one with no Revision 0
/// form) is returned unchanged.
///
/// This is the conversion for address strings read back from the wallet database:
/// `zcash_client_sqlite` stores the addresses it hands out in their receiver-preserving Revision 2
/// form (`tu…` with a transparent receiver, `zu…` without), while rows written by SDK 3.0.0 are
/// Revision 0. Stored rows are never rewritten; only the string handed to the app is converted.
pub(crate) fn to_revision_0(encoded: String) -> String {
    to_r0::<unified::Uivk>(to_r0::<unified::Ufvk>(to_r0::<unified::Address>(encoded)))
}

/// The Revision 0 Unified Address holding only the given Orchard receiver, as SDK 3.0.0
/// built it for the account's own internal migration destination.
pub(crate) fn orchard_only_address(net: NetworkType, receiver: &orchard::Address) -> ZcashAddress {
    let ua = unified::Address::try_from_items(
        Revision::R0,
        vec![Uitem::Data(unified::Receiver::Orchard(
            receiver.to_raw_address_bytes(),
        ))],
    )
    .expect("an Orchard receiver alone is a valid Revision 0 Unified Address");
    ZcashAddress::from_unified(net, ua)
}
