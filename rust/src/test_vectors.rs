//! Ground truth from SDK 3.0.0 for tests of strings and keys this build must keep stable.
//!
//! The public Alice and Ben test wallets of the SDK's own Kotlin fixture are derived (account 0,
//! mainnet and testnet) with the crate versions SDK v3.0.0 locked (zcash_keys 0.16.1, orchard
//! 0.15.4, sapling-crypto 0.7.0, zip32 0.2.1, zcash_address 0.13.0), exactly as its JNI functions
//! do, and only the first 8 bytes (hex) of the SHA-256 of each result are kept: addresses, keys
//! and seeds stay out of the source and the test output. Generated 2 Oct 2026.
//!
//! - `ua_all`: `deriveUnifiedAddressFromSeed` (`AllAvailableKeys`, first diversifier).
//! - `ua_sapling_orchard`: the first Sapling+Orchard address (receiver flags 12).
//! - `ufvk`, `uivk`: the account's viewing keys as strings.
//! - `mk_account_sk`, `mk_account_c`: `deriveAccountMetadataKeyFromSeed` (key and chain code).
//! - `mk_inherent`, `mk_external`: `derivePrivateUseMetadataKey` with subject [`SUBJECT`],
//!   without a UFVK and with the account's UFVK (one key per FVK item, in order).

use sha2::{Digest, Sha256};

/// The SDK's own Kotlin fixture. Only its seed phrases are used; its addresses predate 3.0.0.
const WALLET_FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/rust/src/test_fixtures/WalletFixture.kt"
));

/// The private-use subject the `mk_*` digests were derived with.
pub(crate) const SUBJECT: &[u8] = b"zchat-nu7-groundtruth";

/// (wallet, network, kind, digest); `mk_external` holds one digest per key, comma-separated.
pub(crate) const SDK_3_0_0: &[(&str, &str, &str, &str)] = &[
    ("Alice", "main", "ua_all", "a69309b30813cc69"),
    ("Alice", "main", "ua_sapling_orchard", "67c24291b684bce2"),
    ("Alice", "main", "ufvk", "a278ced699e9a18c"),
    ("Alice", "main", "uivk", "9ba68fb612e4267b"),
    ("Alice", "main", "mk_account_sk", "c27b70df74fdd73e"),
    ("Alice", "main", "mk_account_c", "3c6646557d6b2b28"),
    ("Alice", "main", "mk_inherent", "71e6b36dbddda784"),
    (
        "Alice",
        "main",
        "mk_external",
        "f8e45e1f37b8f080,3021169a4b3f324b,232957ec7da43dd5",
    ),
    ("Alice", "test", "ua_all", "e61354330d86b980"),
    ("Alice", "test", "ua_sapling_orchard", "9714b82325bb5b52"),
    ("Alice", "test", "ufvk", "6b886d99b3466751"),
    ("Alice", "test", "uivk", "5ded72a45e1a271a"),
    ("Alice", "test", "mk_account_sk", "cfa54aa377c69333"),
    ("Alice", "test", "mk_account_c", "e7797a1eaa98b68a"),
    ("Alice", "test", "mk_inherent", "c9c743f52fccacc5"),
    (
        "Alice",
        "test",
        "mk_external",
        "553588fb327edbfd,785b71ba5558f1d3,70fa92f8c6ba7e6b",
    ),
    ("Ben", "main", "ua_all", "86da21012dc949f2"),
    ("Ben", "main", "ua_sapling_orchard", "9e146d289ad5e629"),
    ("Ben", "main", "ufvk", "e82ff1b9d4f9cd55"),
    ("Ben", "main", "uivk", "bb4974f86c3ac73d"),
    ("Ben", "main", "mk_account_sk", "a3476fc5cd6c7ec9"),
    ("Ben", "main", "mk_account_c", "ad41b2c1321ae759"),
    ("Ben", "main", "mk_inherent", "c4871e536afa6a8c"),
    (
        "Ben",
        "main",
        "mk_external",
        "51dc180eb90bb0b0,7e07375daf53ed1d,143c3fc579482cf1",
    ),
    ("Ben", "test", "ua_all", "6c5c5e76d4e1a21e"),
    ("Ben", "test", "ua_sapling_orchard", "b9abefedb6da26a3"),
    ("Ben", "test", "ufvk", "ff6b21b778ad9f91"),
    ("Ben", "test", "uivk", "69b4eb7ad92cf3af"),
    ("Ben", "test", "mk_account_sk", "fab942b90ce0e528"),
    ("Ben", "test", "mk_account_c", "222d5759cefac0e6"),
    ("Ben", "test", "mk_inherent", "3632f7fe60776171"),
    (
        "Ben",
        "test",
        "mk_external",
        "50137e51a69d7370,b6ed30488b4419f8,04684bb259f9d8fc",
    ),
];

pub(crate) fn expected(wallet: &str, network: &str, kind: &str) -> &'static str {
    SDK_3_0_0
        .iter()
        .find(|(w, n, k, _)| *w == wallet && *n == network && *k == kind)
        .map(|(_, _, _, d)| *d)
        .expect("ground truth present")
}

/// First 8 bytes (hex) of SHA-256.
pub(crate) fn digest(bytes: &[u8]) -> String {
    hex::encode(&Sha256::digest(bytes)[..8])
}

/// The BIP 39 seed (empty passphrase) of a fixture wallet.
pub(crate) fn seed(wallet: &str) -> [u8; 64] {
    let start = WALLET_FIXTURE
        .find(&format!("data object {wallet}"))
        .expect("fixture wallet present");
    let section = &WALLET_FIXTURE[start..];
    let from = section
        .find("seedPhrase: String")
        .expect("seed phrase getter");
    let to = section
        .find("getBirthday")
        .expect("birthday getter after the seed");
    let phrase: String = section[from..to].split('"').skip(1).step_by(2).collect();
    bip0039::Mnemonic::<bip0039::English>::from_phrase(phrase)
        .expect("valid mnemonic")
        .to_seed("")
}
