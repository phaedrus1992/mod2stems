//! Property tests for the profile resolver and the Amiga verdict.
//!
//! The example tests in `tests/profile.rs` pin the named cases. These tests
//! cover the shapes that a fixed case cannot: any sample pattern, any length,
//! any profile text, and any combination of the request fields.

use mod2stems::AmigaModel;
use mod2stems::Profile;
use mod2stems::ProfileError;
use mod2stems::ProfileRequest;
use mod2stems::profile::AmigaVerdict;
use mod2stems::profile::require_deterministic;
use mod2stems::profile::verify_amiga_applies;
use proptest::prelude::any;
use proptest::prelude::prop_assert_eq;
use proptest::proptest;
use proptest::test_runner::Config;
use proptest::test_runner::TestCaseError;

/// The inclusive rate range that the renderer accepts.
const RATES: std::ops::RangeInclusive<i32> = 8_000..=192_000;

/// Returns one of the three profiles for a small index.
fn profile_at(index: u8) -> Profile {
    match index % 3 {
        0 => Profile::Compatible,
        1 => Profile::Amiga,
        _ => Profile::Clean,
    }
}

/// Returns one of the three models for a small index.
fn model_at(index: u8) -> AmigaModel {
    match index % 3 {
        0 => AmigaModel::A500,
        1 => AmigaModel::A1200,
        _ => AmigaModel::Unfiltered,
    }
}

/// Builds a request for a profile and an optional model, then sets a rate.
fn request_for(profile: Profile, model: Option<u8>, rate: i32) -> ProfileRequest {
    let request = match (profile, model) {
        (Profile::Amiga, Some(index)) => ProfileRequest::amiga(Some(model_at(index))),
        (Profile::Amiga, None) => ProfileRequest::amiga(None),
        (Profile::Clean, Some(index)) => ProfileRequest::clean().with_amiga_model(model_at(index)),
        (Profile::Clean, None) => ProfileRequest::clean(),
        (Profile::Compatible, Some(index)) => {
            ProfileRequest::default().with_amiga_model(model_at(index))
        }
        (Profile::Compatible, None) => ProfileRequest::default(),
    };
    request.with_rate(rate)
}

proptest! {
    #![proptest_config(Config::with_cases(64))]

    /// A buffer matches itself, and a one-bit change makes it differ.
    #[test]
    fn the_verdict_follows_the_sample_bits(
        bits in proptest::collection::vec(any::<u32>(), 0..64),
        index in any::<usize>(),
    ) {
        let plain: Vec<f32> = bits.iter().copied().map(f32::from_bits).collect();
        prop_assert_eq!(
            verify_amiga_applies(&plain, &plain, &plain),
            Ok(AmigaVerdict::DoesNotApply)
        );
        if plain.is_empty() {
            return Ok(());
        }
        let at = index % plain.len();
        let mut changed = plain.clone();
        changed[at] = f32::from_bits(plain[at].to_bits() ^ 1);
        prop_assert_eq!(
            verify_amiga_applies(&plain, &plain, &changed),
            Ok(AmigaVerdict::Applies)
        );
    }

    /// A length difference is reported with both lengths.
    #[test]
    fn a_length_difference_names_both_lengths(
        plain_len in 0_usize..64,
        emulated_len in 0_usize..64,
    ) {
        if plain_len == emulated_len {
            return Ok(());
        }
        let plain = vec![0.0_f32; plain_len];
        let emulated = vec![0.0_f32; emulated_len];
        prop_assert_eq!(
            verify_amiga_applies(&plain, &plain, &emulated),
            Err(ProfileError::AmigaComparisonLength {
                plain: plain_len,
                emulated: emulated_len,
            })
        );
    }

    /// Two plain renders that differ stop the comparison before a verdict.
    #[test]
    fn a_randomized_pair_gives_no_verdict(
        plain_len in 1_usize..64,
        index in any::<usize>(),
    ) {
        let plain = vec![0.25_f32; plain_len];
        let emulated = vec![0.5_f32; plain_len];
        let mut again = plain.clone();
        let at = index % plain.len();
        again[at] = f32::from_bits(plain[at].to_bits() ^ 1);
        prop_assert_eq!(
            verify_amiga_applies(&plain, &again, &emulated),
            Err(ProfileError::Nondeterministic { sample: at })
        );
    }

    /// A buffer matches itself in the determinism check.
    #[test]
    fn the_determinism_check_accepts_an_equal_pair(
        bits in proptest::collection::vec(any::<u32>(), 0..64),
    ) {
        let plain: Vec<f32> = bits.iter().copied().map(f32::from_bits).collect();
        prop_assert_eq!(require_deterministic(&plain, &plain), Ok(()));
    }

    /// A profile name parses exactly when it is one of the three names.
    #[test]
    fn a_profile_name_parses_exactly(text in ".*") {
        let parsed = Profile::parse(&text);
        let known = [Profile::Compatible, Profile::Amiga, Profile::Clean]
            .into_iter()
            .find(|profile| profile.as_str() == text);
        match (parsed, known) {
            (Ok(found), Some(wanted)) => prop_assert_eq!(found, wanted),
            (Err(ProfileError::UnknownName { name }), None) => prop_assert_eq!(name, text),
            (other, _) => return Err(TestCaseError::fail(format!("{text:?} parsed to {other:?}"))),
        }
    }

    /// The resolver follows the documented decision table.
    #[test]
    fn the_resolver_follows_the_decision_table(
        profile_index in any::<u8>(),
        model_index in any::<Option<u8>>(),
        rate in any::<i32>(),
    ) {
        let profile = profile_at(profile_index);
        let request = request_for(profile, model_index, rate);
        if !RATES.contains(&rate) {
            prop_assert_eq!(
                request.resolve(),
                Err(ProfileError::RateOutOfRange {
                    rate,
                    lowest: *RATES.start(),
                    highest: *RATES.end(),
                })
            );
            return Ok(());
        }
        match (profile, model_index) {
            (Profile::Amiga, None) => prop_assert_eq!(
                request.resolve(),
                Err(ProfileError::MissingAmigaModel)
            ),
            (Profile::Compatible | Profile::Clean, Some(index)) => {
                prop_assert_eq!(
                    request.resolve(),
                    Err(ProfileError::UnexpectedAmigaModel {
                        profile: profile.as_str(),
                        model: model_at(index).as_str(),
                    })
                );
            }
            _ => {
                let config = request
                    .resolve()
                    .map_err(|error| TestCaseError::fail(format!("{profile:?} failed: {error}")))?;
                prop_assert_eq!(config.profile(), profile);
                prop_assert_eq!(config.rate(), rate);
                prop_assert_eq!(config.amiga_enabled(), profile == Profile::Amiga);
            }
        }
    }
}
