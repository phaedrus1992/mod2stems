//! Property tests that the loader always returns.

mod support;

use openmpt_ext::Module;
use proptest::collection::vec;
use proptest::prelude::any;
use proptest::prelude::prop;
use proptest::prelude::prop_assert;
use proptest::proptest;
use proptest::test_runner::Config;

/// Changes up to 16 bytes of a valid file, so the bytes reach the MOD reader.
fn mutate_mod(bytes: &[u8], seed: u64, count: usize) -> Vec<u8> {
    let mut out = bytes.to_vec();
    if out.is_empty() {
        return out;
    }
    let mut state = seed;
    for _ in 0..count {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        // The top bits of a linear congruential generator spread better than
        // the low bits.
        let index = usize::try_from(state >> 40).unwrap_or(0) % out.len();
        let value = u8::try_from((state >> 24) & 0xFF).unwrap_or(0);
        if let Some(byte) = out.get_mut(index) {
            *byte = value;
        }
    }
    out
}

fn config() -> Config {
    Config {
        cases: 256,
        ..Config::default()
    }
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn from_bytes_never_crashes(bytes in vec(any::<u8>(), 0..8192)) {
        // A load either succeeds or fails. A crash, a hang, or an unwind
        // fails the test.
        let result = Module::from_bytes(&bytes, &[]);
        prop_assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn mutated_mod_never_crashes(seed in any::<u64>(), count in 0_usize..17) {
        let bytes = support::simple_mod();
        let mutated = mutate_mod(&bytes, seed, count);
        let result = Module::from_bytes(&mutated, &[]);
        prop_assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn an_empty_load_fails(empty in prop::collection::vec(any::<u8>(), 0)) {
        let result = Module::from_bytes(&empty, &[]);
        prop_assert!(result.is_err());
    }
}
