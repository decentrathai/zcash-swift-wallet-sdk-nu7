//! The operating system's random number generator, as a `rand_core 0.10` RNG.
//!
//! `rand 0.10` replaced `rand::rngs::OsRng` with the fallible `SysRng`, and the librustzcash crates
//! of the NU7 cohort take their RNG as an argument (`rand_core::{Rng, CryptoRng}`) instead of
//! drawing from the operating system themselves. This unit struct keeps the old `OsRng` name and
//! behaviour for the JNI layer: it reads from the operating system on every call, and panics if
//! the operating system cannot supply randomness, as `rand 0.8`'s `OsRng` did through `RngCore`.

use std::convert::Infallible;

use rand::rand_core::{TryCryptoRng, TryRng};
use rand::rngs::SysRng;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct OsRng;

impl TryRng for OsRng {
    type Error = Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Infallible> {
        Ok(SysRng.try_next_u32().expect("operating system RNG failed"))
    }

    fn try_next_u64(&mut self) -> Result<u64, Infallible> {
        Ok(SysRng.try_next_u64().expect("operating system RNG failed"))
    }

    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Infallible> {
        SysRng
            .try_fill_bytes(dst)
            .expect("operating system RNG failed");
        Ok(())
    }
}

impl TryCryptoRng for OsRng {}
