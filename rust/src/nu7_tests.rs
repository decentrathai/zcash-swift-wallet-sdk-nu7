//! RED tests for the NU7 port of libzcashlc (ZChat fork, milestone M2a).
//!
//! Ported from the Android fork (zcash-android-wallet-sdk at a99a745e, tag v3.0.0-nu7-zchat.3):
//! `backend-lib/src/main/rust/lib.rs` `tests::branch_ids_around_nu7` and
//! `tests::metadata_keys_match_sdk_3_0_0`, and `backend-lib/src/main/rust/r0.rs` `tests`. The
//! ground-truth digests come from Android's exporter in [`crate::test_vectors`] (copied verbatim,
//! only the fixture path differs); none are typed in here.
//!
//! What these tests require of the port (they compile and pass without edits once it exists):
//! - the FFI's consensus parameters (`crate::parse_network`, `zcashlc_branch_id_for_height`) know
//!   `NetworkUpgrade::Nu7` / `BranchId::Nu7` = 0x77190ad9, mainnet height 3,543,405 and testnet
//!   height 4,465,026 (zcash_protocol 0.11.0);
//! - `crate::r0` with `pub(crate)` `encode_ua`, `encode_ufvk`, `encode_uivk`, `to_revision_0` and
//!   `orchard_only_address`, as in Android's `r0.rs`;
//! - `crate::derivation::{account_metadata_key, private_use_metadata_keys}` with the signatures of
//!   Android's lib.rs helpers of the same names.
//!
//! Not ported here: `r0::tests::mixed_database_reads_back_one_string_per_address` needs the
//! `zcash_client_sqlite` `test-dependencies` feature; it comes with Android's `r0.rs` itself.
//! `r0::to_r0` is private in Android's `r0.rs`, so the two cases that call it use the public
//! `to_revision_0`, which applies `to_r0::<unified::Address>` first.

// The Revision 2 strings are inputs here: they state the premise and what the database stores.
#![allow(clippy::disallowed_methods)]

use zcash_address::unified::{self, Container, Encoding, Revision, Uitem};
use zcash_client_backend::{
    address::UnifiedAddress,
    keys::{
        ReceiverRequirement::{Omit, Require},
        UnifiedAddressRequest, UnifiedFullViewingKey, UnifiedSpendingKey,
    },
};
use zcash_protocol::consensus::{
    BlockHeight, BranchId, Network, NetworkConstants, NetworkType, NetworkUpgrade, Parameters,
};

use crate::r0::{encode_ua, encode_ufvk, encode_uivk, orchard_only_address, to_revision_0};
use crate::test_vectors::{SUBJECT, digest, expected, seed};

const NU6_3: u32 = 0x37a5_165b;
const NU7: u32 = 0x7719_0ad9;
const NU7_MAINNET: u32 = 3_543_405;
const NU7_TESTNET: u32 = 4_465_026;

// ---------------------------------------------------------------------------------------------
// (1) NU7 consensus parameters as the FFI resolves them.
// ---------------------------------------------------------------------------------------------

/// NU7's consensus branch id is 0x77190ad9 in both directions.
#[test]
fn nu7_branch_id_is_0x77190ad9() {
    assert_eq!(u32::from(BranchId::Nu7), NU7);
    assert_eq!(BranchId::try_from(NU7).ok(), Some(BranchId::Nu7));
    assert_eq!(NetworkUpgrade::Nu7.branch_id(), BranchId::Nu7);
}

/// What `parse_network` hands every FFI call activates NU7 at 3,543,405 (mainnet, id 1) and
/// 4,465,026 (testnet, id 0).
#[test]
fn ffi_networks_activate_nu7_at_the_zip_259_heights() {
    let main = crate::parse_network(crate::NETWORK_ID_MAINNET).unwrap();
    let test = crate::parse_network(crate::NETWORK_ID_TESTNET).unwrap();
    assert_eq!(
        main.activation_height(NetworkUpgrade::Nu7),
        Some(BlockHeight::from(NU7_MAINNET))
    );
    assert_eq!(
        test.activation_height(NetworkUpgrade::Nu7),
        Some(BlockHeight::from(NU7_TESTNET))
    );
    assert_eq!(
        Network::MainNetwork.activation_height(NetworkUpgrade::Nu7),
        Some(BlockHeight::from(NU7_MAINNET))
    );
    assert_eq!(
        Network::TestNetwork.activation_height(NetworkUpgrade::Nu7),
        Some(BlockHeight::from(NU7_TESTNET))
    );
}

/// Android's `branch_ids_around_nu7`, through `parse_network` and through the exported
/// `zcashlc_branch_id_for_height` the Swift layer calls.
#[test]
fn branch_ids_around_nu7() {
    let branch = |network_id: u32, height: u32| {
        u32::from(BranchId::for_height(
            &crate::parse_network(network_id).unwrap(),
            BlockHeight::from(height),
        ))
    };
    let ffi = |network_id: u32, height: u32| {
        crate::zcashlc_branch_id_for_height(height as i32, network_id) as u32
    };
    for f in [&branch as &dyn Fn(u32, u32) -> u32, &ffi] {
        assert_eq!(f(0, NU7_TESTNET - 1), NU6_3);
        assert_eq!(f(0, NU7_TESTNET), NU7);
        assert_eq!(f(0, 5_000_000), NU7);
        assert_eq!(f(1, NU7_MAINNET - 1), NU6_3);
        assert_eq!(f(1, NU7_MAINNET), NU7);
        assert_eq!(f(1, 10_000_000), NU7);
    }
}

// ---------------------------------------------------------------------------------------------
// (2) ZIP 316 Revision 0 (`u1…`, `uview1…`, `uivk1…`) strings, as SDK 3.0.0 returned them.
// ---------------------------------------------------------------------------------------------

fn ufvk_for(name: &str, network: &Network) -> UnifiedFullViewingKey {
    UnifiedSpendingKey::from_seed(network, &seed(name), zip32::AccountId::ZERO)
        .expect("valid seed")
        .to_unified_full_viewing_key()
}

fn first_address(ufvk: &UnifiedFullViewingKey, request: UnifiedAddressRequest) -> UnifiedAddress {
    ufvk.find_address(zip32::DiversifierIndex::new(), request)
        .expect("an address is derivable")
        .0
}

fn network(tag: &str) -> Network {
    match tag {
        "main" => Network::MainNetwork,
        _ => Network::TestNetwork,
    }
}

#[test]
fn strings_match_sdk_3_0_0() {
    let sapling_orchard = UnifiedAddressRequest::custom(Require, Require, Omit).unwrap();
    let cases = ["Alice", "Ben"].into_iter().flat_map(|name| {
        ["main", "test"].into_iter().flat_map(move |tag| {
            ["ua_all", "ua_sapling_orchard", "ufvk", "uivk"].map(|kind| (name, tag, kind))
        })
    });
    for (name, tag, kind) in cases {
        let network = network(tag);
        let ufvk = ufvk_for(name, &network);
        let ours = match kind {
            "ua_all" => encode_ua(
                &first_address(&ufvk, UnifiedAddressRequest::AllAvailableKeys),
                &network,
            ),
            "ua_sapling_orchard" => encode_ua(&first_address(&ufvk, sapling_orchard), &network),
            "ufvk" => encode_ufvk(&ufvk, &network),
            "uivk" => encode_uivk(&ufvk.to_unified_incoming_viewing_key(), &network),
            _ => unreachable!(),
        };
        assert_eq!(
            digest(ours.as_bytes()),
            expected(name, tag, kind),
            "{name} {tag} {kind} differs from SDK 3.0.0"
        );
    }
}

#[test]
fn upstream_encoding_differs_from_sdk_3_0_0() {
    // Guards the premise of `r0`: without it, the strings would change.
    let network = Network::MainNetwork;
    let ufvk = ufvk_for("Ben", &network);
    let ua = first_address(&ufvk, UnifiedAddressRequest::AllAvailableKeys);
    // Stable zcash_keys 0.17.0 encodes at Revision 0 when it can: its default strings are
    // SDK 3.0.0's again.
    assert_eq!(
        digest(ua.to_zcash_address(network.network_type()).encode().as_bytes()),
        expected("Ben", "main", "ua_all")
    );
    assert_eq!(
        digest(ufvk.encode(&network).expect("UFVK encodes").as_bytes()),
        expected("Ben", "main", "ufvk")
    );
}

#[test]
fn viewing_keys_round_trip() {
    for (network, ufvk_hrp, uivk_hrp) in [
        (Network::MainNetwork, "uview1", "uivk1"),
        (Network::TestNetwork, "uviewtest1", "uivktest1"),
    ] {
        let ufvk = ufvk_for("Alice", &network);
        let ufvk_r0 = encode_ufvk(&ufvk, &network);
        assert!(ufvk_r0.starts_with(ufvk_hrp), "UFVK is not Revision 0");
        let decoded = UnifiedFullViewingKey::decode(&network, &ufvk_r0).expect("R0 UFVK decodes");
        assert!(
            decoded.encode(&network).expect("UFVK encodes")
                == ufvk.encode(&network).expect("UFVK encodes"),
            "UFVK content changed"
        );

        let uivk = ufvk.to_unified_incoming_viewing_key();
        let uivk_r0 = encode_uivk(&uivk, &network);
        assert!(uivk_r0.starts_with(uivk_hrp), "UIVK is not Revision 0");
        let (net, revision, r0) = unified::Uivk::decode(&uivk_r0).expect("R0 UIVK decodes");
        let (_, _, r2) = unified::Uivk::decode(
            &uivk
                .encode_revision(&network, Revision::R2)
                .expect("R2 encodes"),
        )
        .expect("R2 UIVK decodes");
        assert_eq!((net, revision), (network.network_type(), Revision::R0));
        assert!(r0.items() == r2.items(), "UIVK items changed");
    }
}

#[test]
fn revision_0_input_is_unchanged_and_garbage_passes_through() {
    let network = Network::TestNetwork;
    let ua = first_address(
        &ufvk_for("Ben", &network),
        UnifiedAddressRequest::AllAvailableKeys,
    );
    let r0 = encode_ua(&ua, &network);
    // Android calls the private `to_r0::<unified::Address>`; `to_revision_0` applies it first.
    assert!(to_revision_0(r0.clone()) == r0);
    assert_eq!(
        to_revision_0("not an address".to_owned()),
        "not an address"
    );
}

#[test]
fn metadata_keeps_revision_2() {
    let network = Network::MainNetwork;
    let ua = first_address(
        &ufvk_for("Ben", &network),
        UnifiedAddressRequest::AllAvailableKeys,
    );
    let expiring = UnifiedAddress::from_receivers(
        ua.orchard().copied(),
        ua.sapling().copied(),
        ua.transparent().copied(),
        Some(BlockHeight::from_u32(5_000_000)),
        None,
    )
    .expect("valid receivers");
    let (_, revision, parsed) =
        unified::Address::decode(&encode_ua(&expiring, &network)).expect("decodes");
    assert_eq!(revision, Revision::R2);
    assert!(
        parsed
            .items_as_parsed()
            .iter()
            .any(|i| matches!(i, Uitem::Metadata(_))),
        "expiry metadata must survive"
    );
}

/// What the wallet database stores for these keys and addresses (Revision 2) reads back as the
/// string SDK 3.0.0 returned.
#[test]
fn stored_revision_2_strings_convert_to_sdk_3_0_0_strings() {
    let sapling_orchard = UnifiedAddressRequest::custom(Require, Require, Omit).unwrap();
    for name in ["Alice", "Ben"] {
        for tag in ["main", "test"] {
            let network = network(tag);
            let ufvk = ufvk_for(name, &network);
            let all = first_address(&ufvk, UnifiedAddressRequest::AllAvailableKeys);
            let shielded = first_address(&ufvk, sapling_orchard);
            let uivk = ufvk.to_unified_incoming_viewing_key();
            for (kind, stored) in [
                // `tu…`: the stored form of an address with a transparent receiver.
                (
                    "ua_all",
                    all.to_zcash_address_revision(network.network_type(), Revision::R2)
                        .expect("R2 encodes")
                        .encode(),
                ),
                // `zu…`: the stored form of a shielded-only address.
                (
                    "ua_sapling_orchard",
                    shielded
                        .to_zcash_address_revision(network.network_type(), Revision::R2)
                        .expect("R2 encodes")
                        .encode(),
                ),
                (
                    "ufvk",
                    ufvk.encode_revision(&network, Revision::R2)
                        .expect("R2 encodes"),
                ),
                (
                    "uivk",
                    uivk.encode_revision(&network, Revision::R2)
                        .expect("R2 encodes"),
                ),
            ] {
                let (_, revision, _) = match kind {
                    "ufvk" => unified::Ufvk::decode(&stored).map(|(n, r, _)| (n, r, ())),
                    "uivk" => unified::Uivk::decode(&stored).map(|(n, r, _)| (n, r, ())),
                    _ => unified::Address::decode(&stored).map(|(n, r, _)| (n, r, ())),
                }
                .expect("stored string decodes");
                assert_eq!(revision, Revision::R2, "{name} {tag} {kind}: premise");
                let read_back = to_revision_0(stored);
                assert_eq!(
                    digest(read_back.as_bytes()),
                    expected(name, tag, kind),
                    "{name} {tag} {kind} differs from SDK 3.0.0"
                );
                // Converting twice changes nothing: Revision 0 rows read back as they are.
                assert!(to_revision_0(read_back.clone()) == read_back);
            }
        }
    }
}

#[test]
fn other_strings_are_returned_unchanged() {
    let network = Network::MainNetwork;
    let ufvk = ufvk_for("Alice", &network);
    let ua = first_address(&ufvk, UnifiedAddressRequest::AllAvailableKeys);
    let transparent = zcash_client_backend::encoding::AddressCodec::encode(
        ua.transparent().expect("transparent receiver"),
        &network,
    );
    let sapling =
        zcash_client_backend::address::Address::from(*ua.sapling().expect("Sapling receiver"))
            .encode(&network);
    for other in [
        transparent.as_str(),
        sapling.as_str(),
        "",
        "not an address",
        "zu1",
        "tu1qqqq",
    ] {
        assert!(
            to_revision_0(other.to_owned()) == other,
            "a non-unified string changed"
        );
    }
}

#[test]
fn orchard_only_address_is_revision_0() {
    for network in [Network::MainNetwork, Network::TestNetwork] {
        let receiver = ufvk_for("Alice", &network)
            .orchard()
            .expect("Orchard FVK")
            .address_at(0u32, orchard::keys::Scope::Internal);
        let encoded = orchard_only_address(network.network_type(), &receiver).encode();
        let prefix = match network.network_type() {
            NetworkType::Main => "u1",
            _ => "utest1",
        };
        assert!(encoded.starts_with(prefix), "not a Revision 0 address");
        let (_, revision, ua) = unified::Address::decode(&encoded).expect("decodes");
        assert_eq!(revision, Revision::R0);
        assert!(
            ua.items() == vec![unified::Receiver::Orchard(receiver.to_raw_address_bytes())],
            "receiver changed"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// (3) ZIP 325 metadata keys stay byte-identical to SDK 3.0.0.
// ---------------------------------------------------------------------------------------------

/// The account's UFVK reaches `zcashlc_derive_private_use_metadata_key` as Revision 0 (what this
/// build returns) or, from data written by another SDK, Revision 2; both must give the same keys.
#[test]
fn metadata_keys_match_sdk_3_0_0() {
    use crate::derivation::{account_metadata_key, private_use_metadata_keys};
    for (tag, network) in [
        ("main", Network::MainNetwork),
        ("test", Network::TestNetwork),
    ] {
        for name in ["Alice", "Ben"] {
            let seed = seed(name);
            let account = zip32::AccountId::ZERO;
            let ak = account_metadata_key(&seed, network.coin_type(), account).unwrap();
            assert_eq!(digest(ak.data()), expected(name, tag, "mk_account_sk"));
            assert_eq!(
                digest(ak.chain_code().as_bytes()),
                expected(name, tag, "mk_account_c")
            );

            let inherent =
                private_use_metadata_keys(&ak, None, network.network_type(), SUBJECT).unwrap();
            assert_eq!(inherent.len(), 1);
            assert_eq!(
                digest(inherent[0].data()),
                expected(name, tag, "mk_inherent")
            );

            let ufvk = UnifiedSpendingKey::from_seed(&network, &seed, account)
                .unwrap()
                .to_unified_full_viewing_key();
            for ufvk_string in [
                encode_ufvk(&ufvk, &network),
                ufvk.encode_revision(&network, Revision::R2)
                    .expect("R2 encodes"),
            ] {
                let keys = private_use_metadata_keys(
                    &ak,
                    Some(&ufvk_string),
                    network.network_type(),
                    SUBJECT,
                )
                .unwrap();
                let digests: Vec<String> = keys.iter().map(|k| digest(k.data())).collect();
                assert_eq!(digests.join(","), expected(name, tag, "mk_external"));
            }
        }
    }
}
