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

/// How an [`AnimationTimeline`] behaves when it reaches the preset's cycle end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopMode {
    Loop,
    Once,
}

/// The result of advancing an [`AnimationTimeline`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdvanceResult {
    Advanced,
    Looped,
    Finished,
    Noop,
}

/// Stateful playback of one procedural animation preset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimationTimeline {
    preset: AnimationPreset,
    position: Duration,
    speed: f32,
    playing: bool,
    loop_mode: LoopMode,
    finished: bool,
}

impl AnimationTimeline {
    const MAX_FRAME_GAP: Duration = Duration::from_millis(250);
    const TERMINAL_SAMPLE_OFFSET: Duration = Duration::from_millis(1);

    /// Create a paused, looping timeline at the start of a preset.
    pub fn new(preset: AnimationPreset) -> Self {
        Self {
            preset,
            position: Duration::ZERO,
            speed: 1.0,
            playing: false,
            loop_mode: LoopMode::Loop,
            finished: false,
        }
    }

    pub fn preset(&self) -> AnimationPreset {
        self.preset
    }

    pub fn position(&self) -> Duration {
        self.position
    }

    pub fn duration(&self) -> Duration {
        self.preset.cycle_duration()
    }

    pub fn speed(&self) -> f32 {
        self.speed
    }

    pub fn loop_mode(&self) -> LoopMode {
        self.loop_mode
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Return the current posture, including while paused.
    pub fn sample(&self) -> Posture {
        if self.finished && self.loop_mode == LoopMode::Once {
            self.preset
                .sample(self.duration().saturating_sub(Self::TERMINAL_SAMPLE_OFFSET))
        } else {
            self.preset.sample(self.position)
        }
    }

    pub fn play(&mut self) {
        if !self.finished {
            self.playing = true;
        }
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn set_preset(&mut self, preset: AnimationPreset) {
        self.preset = preset;
        self.position = Duration::ZERO;
        self.finished = false;
    }

    pub fn set_loop_mode(&mut self, loop_mode: LoopMode) {
        if self.loop_mode == LoopMode::Once && loop_mode == LoopMode::Loop && self.finished {
            self.position = Duration::ZERO;
            self.finished = false;
        }
        self.loop_mode = loop_mode;
    }

    /// Clamp playback speed to the supported range.
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = if speed.is_finite() {
            speed.clamp(0.25, 4.0)
        } else if speed.is_sign_negative() {
            0.25
        } else {
            4.0
        };
    }

    pub fn seek_to(&mut self, position: Duration) {
        let cycle_nanos = self.duration().as_nanos();
        if self.loop_mode == LoopMode::Loop {
            self.position = Duration::from_nanos((position.as_nanos() % cycle_nanos) as u64);
            self.finished = false;
        } else if position >= self.duration() {
            self.position = self.duration();
            self.finished = true;
            self.playing = false;
        } else {
            self.position = position;
            self.finished = false;
        }
    }

    pub fn seek_by(&mut self, delta: Duration) {
        let cycle_nanos = self.duration().as_nanos();
        let target = self.position.as_nanos().saturating_add(delta.as_nanos());
        if self.loop_mode == LoopMode::Loop {
            self.position = Duration::from_nanos((target % cycle_nanos) as u64);
            self.finished = false;
        } else {
            self.seek_to(Duration::from_nanos(target.min(cycle_nanos) as u64));
        }
    }

    pub fn seek_back(&mut self, delta: Duration) {
        let cycle_nanos = self.duration().as_nanos();
        let current = self.position.as_nanos().min(cycle_nanos);
        if self.loop_mode == LoopMode::Loop {
            let offset = delta.as_nanos() % cycle_nanos;
            self.position =
                Duration::from_nanos(((current + cycle_nanos - offset) % cycle_nanos) as u64);
            self.finished = false;
        } else {
            self.seek_to(Duration::from_nanos(
                current.saturating_sub(delta.as_nanos()) as u64,
            ));
        }
    }

    /// Advance by a platform-provided delta. Long gaps are treated as pauses.
    pub fn advance(&mut self, delta: Duration) -> AdvanceResult {
        if !self.playing || self.finished || delta.is_zero() || delta > Self::MAX_FRAME_GAP {
            return AdvanceResult::Noop;
        }
        let scaled = Duration::from_secs_f64(delta.as_secs_f64() * f64::from(self.speed));
        let cycle_nanos = self.duration().as_nanos();
        let target = self.position.as_nanos().saturating_add(scaled.as_nanos());

        if self.loop_mode == LoopMode::Once && target >= cycle_nanos {
            self.position = self.duration();
            self.playing = false;
            self.finished = true;
            AdvanceResult::Finished
        } else if self.loop_mode == LoopMode::Loop && target >= cycle_nanos {
            self.position = Duration::from_nanos((target % cycle_nanos) as u64);
            AdvanceResult::Looped
        } else {
            self.position = Duration::from_nanos(target as u64);
            AdvanceResult::Advanced
        }
    }
}

/// Deterministic frame times for one offline animation cycle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimationSchedule {
    preset: AnimationPreset,
    fps: u32,
    speed: f32,
    frame_count: u32,
}

impl AnimationSchedule {
    /// Return `None` when FPS or speed is outside the supported output range.
    pub fn new(preset: AnimationPreset, fps: u32, speed: f32) -> Option<Self> {
        if !(1..=60).contains(&fps) || !speed.is_finite() || !(0.25..=4.0).contains(&speed) {
            return None;
        }
        let cycle_seconds = preset.cycle_duration().as_secs_f64();
        let frame_count = (cycle_seconds / f64::from(speed) * f64::from(fps))
            .round()
            .max(1.0) as u32;
        Some(Self {
            preset,
            fps,
            speed,
            frame_count,
        })
    }

    pub fn preset(&self) -> AnimationPreset {
        self.preset
    }

    pub fn fps(&self) -> u32 {
        self.fps
    }

    pub fn speed(&self) -> f32 {
        self.speed
    }

    pub fn frame_count(&self) -> u32 {
        self.frame_count
    }

    pub fn duration(&self) -> Duration {
        Duration::from_secs_f64(self.preset.cycle_duration().as_secs_f64() / f64::from(self.speed))
    }

    pub fn sample_time(&self, frame_index: u32) -> Duration {
        debug_assert!(frame_index < self.frame_count);
        let nanos = self.preset.cycle_duration().as_nanos() * u128::from(frame_index)
            / u128::from(self.frame_count);
        Duration::from_nanos(nanos as u64)
    }
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
                right_arm_roll: 4.0,
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
                left_arm_roll: 0.0,
                left_arm_pitch: -wave * 70.0,
                right_arm_roll: 0.0,
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
                right_arm_roll: 4.0 + wave * 2.0,
                right_arm_pitch: -8.0 - wave * 3.0,
                left_leg_pitch: -5.0 + wave * 2.0,
                right_leg_pitch: -5.0 - wave * 2.0,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.001,
            "expected {expected}, got {actual}"
        );
    }

    fn samples(preset: AnimationPreset) -> [Posture; 3] {
        let cycle = preset.cycle_duration();
        [
            preset.sample(Duration::ZERO),
            preset.sample(cycle / 4),
            preset.sample(cycle / 2),
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
    fn presets_match_their_defining_key_samples() {
        let [idle_start, idle_quarter, idle_half] = samples(AnimationPreset::Idle);
        for (sample, expected) in [idle_start, idle_quarter, idle_half]
            .into_iter()
            .zip([0.0, 2.5, 0.0])
        {
            assert_close(sample.head_yaw, expected);
        }
        assert_close(idle_start.cape_pitch, 9.841_149);
        assert_close(idle_quarter.cape_pitch, 12.555_165);
        assert_close(idle_half.cape_pitch, 11.758_851);

        let [walk_start, walk_quarter, walk_half] = samples(AnimationPreset::Walk);
        assert_close(walk_start.left_leg_pitch, 0.0);
        assert_close(walk_quarter.left_leg_pitch, 35.0);
        assert_close(walk_half.left_leg_pitch, 0.0);
        assert_close(walk_quarter.left_arm_pitch, -35.0);
        assert_close(walk_quarter.cape_pitch, 17.184_336);

        let [run_start, run_quarter, run_half] = samples(AnimationPreset::Run);
        for sample in [run_start, run_quarter, run_half] {
            assert_close(sample.body_pitch, -8.0);
        }
        assert_close(run_quarter.left_arm_pitch, -70.0);
        assert_close(run_quarter.left_leg_pitch, 65.0);
        assert_close(run_half.cape_pitch, 53.738_85);

        let [wave_start, wave_quarter, wave_half] = samples(AnimationPreset::Wave);
        assert_close(wave_start.left_arm_roll, -28.0);
        assert_close(wave_quarter.left_arm_roll, -6.0);
        assert_close(wave_half.left_arm_roll, -28.0);
        assert_close(wave_quarter.left_arm_pitch, 155.0);
        assert_close(wave_quarter.head_yaw, -5.0);

        let [fly_start, fly_quarter, fly_half] = samples(AnimationPreset::Fly);
        for sample in [fly_start, fly_quarter, fly_half] {
            assert_close(sample.body_pitch, -90.0);
        }
        assert_close(fly_start.elytra_pitch, 20.0);
        assert_close(fly_quarter.elytra_pitch, 23.0);
        assert_close(fly_half.elytra_pitch, 20.0);
        assert_close(fly_quarter.elytra_roll, 64.782_39);
    }

    #[test]
    fn symmetric_presets_store_the_same_arm_roll_for_both_arms() {
        // The renderer applies the left arm's roll negated (mirrored), so a symmetric pose
        // stores the same roll value on both arms.
        for preset in [
            AnimationPreset::Walk,
            AnimationPreset::Run,
            AnimationPreset::Fly,
        ] {
            for fraction in 0..4 {
                let posture = preset.sample(preset.cycle_duration() * fraction / 4);
                assert_eq!(
                    posture.left_arm_roll, posture.right_arm_roll,
                    "{preset:?} should roll both arms the same way in model space"
                );
            }
        }
    }

    #[test]
    fn default_locomotion_postures_match_the_preset_roll() {
        for (preset, posture) in [
            (AnimationPreset::Walk, DefaultPostures::WALKING),
            (AnimationPreset::Run, DefaultPostures::RUNNING),
        ] {
            let sampled = preset.sample(Duration::ZERO);
            assert_eq!(sampled.left_arm_roll, posture.left_arm_roll, "{preset:?}");
            assert_eq!(sampled.right_arm_roll, posture.right_arm_roll, "{preset:?}");
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

    #[test]
    fn timeline_defaults_to_paused_looping_start() {
        let timeline = AnimationTimeline::new(AnimationPreset::Walk);
        assert_eq!(timeline.position(), Duration::ZERO);
        assert_eq!(timeline.speed(), 1.0);
        assert_eq!(timeline.loop_mode(), LoopMode::Loop);
        assert!(!timeline.is_playing());
        assert!(!timeline.is_finished());
        assert_eq!(
            timeline.sample(),
            AnimationPreset::Walk.sample(Duration::ZERO)
        );
    }

    #[test]
    fn timeline_advances_scales_speed_and_ignores_long_gaps() {
        let mut timeline = AnimationTimeline::new(AnimationPreset::Walk);
        timeline.play();
        assert_eq!(
            timeline.advance(Duration::from_millis(100)),
            AdvanceResult::Advanced
        );
        assert_eq!(timeline.position(), Duration::from_millis(100));

        timeline.set_speed(2.0);
        assert_eq!(
            timeline.advance(Duration::from_millis(50)),
            AdvanceResult::Advanced
        );
        assert_eq!(timeline.position(), Duration::from_millis(200));
        assert_eq!(
            timeline.advance(Duration::from_millis(251)),
            AdvanceResult::Noop
        );
        assert_eq!(timeline.position(), Duration::from_millis(200));
    }

    #[test]
    fn timeline_loop_and_once_have_distinct_cycle_end_states() {
        let mut looping = AnimationTimeline::new(AnimationPreset::Walk);
        looping.seek_to(Duration::from_millis(900));
        looping.play();
        assert_eq!(
            looping.advance(Duration::from_millis(100)),
            AdvanceResult::Looped
        );
        assert_eq!(looping.position(), Duration::ZERO);

        let mut once = AnimationTimeline::new(AnimationPreset::Walk);
        once.set_loop_mode(LoopMode::Once);
        once.seek_to(Duration::from_millis(900));
        once.play();
        assert_eq!(
            once.advance(Duration::from_millis(100)),
            AdvanceResult::Finished
        );
        assert!(once.is_finished());
        assert!(!once.is_playing());
        assert_eq!(once.position(), AnimationPreset::Walk.cycle_duration());
        assert_eq!(
            once.sample(),
            AnimationPreset::Walk
                .sample(AnimationPreset::Walk.cycle_duration() - Duration::from_millis(1))
        );
        assert_ne!(once.sample(), AnimationPreset::Walk.sample(Duration::ZERO));
    }

    #[test]
    fn timeline_seeks_in_both_directions_with_cycle_rules() {
        let mut timeline = AnimationTimeline::new(AnimationPreset::Walk);
        timeline.seek_back(Duration::from_millis(100));
        assert_eq!(timeline.position(), Duration::from_millis(900));
        timeline.seek_by(Duration::from_millis(200));
        assert_eq!(timeline.position(), Duration::from_millis(100));

        timeline.set_loop_mode(LoopMode::Once);
        timeline.seek_back(Duration::from_millis(200));
        assert_eq!(timeline.position(), Duration::ZERO);
    }

    #[test]
    fn animation_schedule_validates_and_samples_one_cycle() {
        assert!(AnimationSchedule::new(AnimationPreset::Walk, 0, 1.0).is_none());
        assert!(AnimationSchedule::new(AnimationPreset::Walk, 30, f32::NAN).is_none());

        let schedule = AnimationSchedule::new(AnimationPreset::Walk, 30, 2.0).unwrap();
        assert_eq!(schedule.frame_count(), 15);
        assert_eq!(schedule.duration(), Duration::from_millis(500));
        assert_eq!(schedule.sample_time(0), Duration::ZERO);
        assert!(schedule.sample_time(14) < AnimationPreset::Walk.cycle_duration());
    }
}
