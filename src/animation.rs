//! Stateless procedural posture animation.

use crate::character::{DefaultPostures, Posture};
use std::f32::consts::TAU;
use std::time::Duration;

/// A built-in seamless procedural animation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationPreset {
    Idle,
    Walk,
    Run,
    Wave,
    Fly,
}

impl AnimationPreset {
    /// All built-in presets in the order used by interactive selectors.
    pub const ALL: [Self; 5] = [Self::Idle, Self::Walk, Self::Run, Self::Wave, Self::Fly];

    /// The natural duration of one seamless animation cycle.
    pub const fn cycle_duration(self) -> Duration {
        match self {
            Self::Idle => Duration::from_secs(4),
            Self::Walk => Duration::from_secs(1),
            Self::Run => Duration::from_millis(600),
            Self::Wave => Duration::from_millis(1_200),
            Self::Fly => Duration::from_millis(1_500),
        }
    }

    /// Sample a complete posture. Time automatically wraps at the cycle duration.
    pub fn sample(self, time: Duration) -> Posture {
        let cycle_secs = self.cycle_duration().as_secs_f64();
        let phase = ((time.as_secs_f64() % cycle_secs) / cycle_secs) as f32 * TAU;
        let wave = phase.sin();

        match self {
            Self::Idle => Posture {
                cape_pitch: DefaultPostures::STAND.cape_pitch + (phase - 0.5).sin() * 2.0,
                elytra_pitch: DefaultPostures::STAND.elytra_pitch + wave * 1.0,
                elytra_roll: DefaultPostures::STAND.elytra_roll + wave * 1.5,
                head_yaw: wave * 2.5,
                head_pitch: (phase * 2.0).sin() * 1.25,
                left_arm_pitch: -wave * 1.5,
                right_arm_pitch: wave * 1.5,
                ..DefaultPostures::STAND
            },
            Self::Walk => Posture {
                cape_pitch: 14.0 + (phase - 0.65).sin() * 4.0,
                head_yaw: wave * 2.0,
                head_pitch: (phase * 2.0).sin() * 1.5,
                left_arm_roll: 4.0,
                left_arm_pitch: -wave * 35.0,
                right_arm_roll: -4.0,
                right_arm_pitch: wave * 35.0,
                left_leg_pitch: wave * 35.0,
                right_leg_pitch: -wave * 35.0,
                ..DefaultPostures::STAND
            },
            Self::Run => Posture {
                cape_pitch: 48.0 + (phase - 0.8).sin() * 8.0,
                body_pitch: -8.0,
                head_yaw: wave * 2.5,
                head_pitch: -4.0 + (phase * 2.0).sin() * 2.0,
                left_arm_roll: 10.0,
                left_arm_pitch: -wave * 70.0,
                right_arm_roll: -10.0,
                right_arm_pitch: wave * 70.0,
                left_leg_pitch: wave * 65.0,
                right_leg_pitch: -wave * 65.0,
                ..DefaultPostures::STAND
            },
            Self::Wave => Posture {
                cape_pitch: DefaultPostures::STAND.cape_pitch + (phase - 0.4).sin() * 1.5,
                head_yaw: -8.0 + wave * 3.0,
                head_pitch: (phase * 2.0).sin(),
                left_arm_roll: -28.0 + wave * 22.0,
                left_arm_pitch: 155.0 + (phase * 2.0).sin() * 5.0,
                ..DefaultPostures::STAND
            },
            Self::Fly => Posture {
                cape_pitch: 45.0 + (phase - 0.7).sin() * 5.0,
                body_pitch: -90.0,
                elytra_pitch: 20.0 + wave * 3.0,
                elytra_yaw: wave * 2.0,
                elytra_roll: 58.0 + (phase - 0.25).sin() * 7.0,
                head_yaw: wave * 2.0,
                head_pitch: 42.0 + (phase * 2.0).sin() * 2.0,
                left_arm_roll: 4.0 + wave * 2.0,
                left_arm_pitch: -8.0 + wave * 3.0,
                right_arm_roll: -4.0 - wave * 2.0,
                right_arm_pitch: -8.0 - wave * 3.0,
                left_leg_pitch: -5.0 + wave * 2.0,
                right_leg_pitch: -5.0 - wave * 2.0,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AnimationPreset;
    use crate::character::Posture;
    use std::time::Duration;

    fn angles(posture: Posture) -> [f32; 13] {
        [
            posture.cape_pitch,
            posture.body_pitch,
            posture.elytra_pitch,
            posture.elytra_yaw,
            posture.elytra_roll,
            posture.head_yaw,
            posture.head_pitch,
            posture.left_arm_roll,
            posture.left_arm_pitch,
            posture.right_arm_roll,
            posture.right_arm_pitch,
            posture.left_leg_pitch,
            posture.right_leg_pitch,
        ]
    }

    #[test]
    fn sampling_a_full_cycle_returns_to_the_start() {
        for preset in AnimationPreset::ALL {
            let start = preset.sample(std::time::Duration::ZERO);
            let end = preset.sample(preset.cycle_duration());

            assert_eq!(start, end, "{preset:?} should loop continuously");
        }
    }

    #[test]
    fn key_cycle_samples_have_only_finite_angles() {
        for preset in AnimationPreset::ALL {
            let cycle = preset.cycle_duration();
            for time in [Duration::ZERO, cycle / 4, cycle / 2, cycle * 3 / 4, cycle] {
                assert!(
                    angles(preset.sample(time)).into_iter().all(f32::is_finite),
                    "{preset:?} has a non-finite angle at {time:?}"
                );
            }
        }
    }

    #[test]
    fn locomotion_swings_opposite_limbs_out_of_phase() {
        for preset in [AnimationPreset::Walk, AnimationPreset::Run] {
            let posture = preset.sample(preset.cycle_duration() / 4);

            assert!(posture.left_arm_pitch < 0.0);
            assert!(posture.right_arm_pitch > 0.0);
            assert!(posture.left_leg_pitch > 0.0);
            assert!(posture.right_leg_pitch < 0.0);
            assert_eq!(posture.left_arm_pitch, -posture.right_arm_pitch);
            assert_eq!(posture.left_leg_pitch, -posture.right_leg_pitch);
        }
    }

    #[test]
    fn presets_expose_their_natural_cycle_durations() {
        assert_eq!(
            AnimationPreset::Idle.cycle_duration(),
            Duration::from_secs(4)
        );
        assert_eq!(
            AnimationPreset::Walk.cycle_duration(),
            Duration::from_secs(1)
        );
        assert_eq!(
            AnimationPreset::Run.cycle_duration(),
            Duration::from_millis(600)
        );
        assert_eq!(
            AnimationPreset::Wave.cycle_duration(),
            Duration::from_millis(1_200)
        );
        assert_eq!(
            AnimationPreset::Fly.cycle_duration(),
            Duration::from_millis(1_500)
        );
    }

    #[test]
    fn wave_and_fly_keep_their_defining_baselines_through_the_cycle() {
        for fraction in 0..4 {
            let wave =
                AnimationPreset::Wave.sample(AnimationPreset::Wave.cycle_duration() * fraction / 4);
            assert!(wave.left_arm_pitch > 140.0);
            assert_eq!(wave.right_arm_pitch, 0.0);

            let fly =
                AnimationPreset::Fly.sample(AnimationPreset::Fly.cycle_duration() * fraction / 4);
            assert_eq!(fly.body_pitch, -90.0);
            assert!(fly.elytra_roll > 45.0);
        }
    }
}
