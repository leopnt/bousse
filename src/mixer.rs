use std::{env, path::PathBuf};

use kira::{
    manager::{AudioManager, AudioManagerSettings, DefaultBackend},
    sound::static_sound::{StaticSoundData, StaticSoundHandle, StaticSoundSettings},
    track::{TrackBuilder, TrackHandle, TrackRoutes},
    tween::Tween,
};

use crate::utils::lerp;

#[derive(Clone, Copy)]
pub enum ChControl {
    Untouched,
    SoftTouching,
    Cueing,
    Seeking,
}

#[derive(Clone, Copy)]
pub enum ChState {
    Playing,
    Paused,
}

pub struct Mixer {
    audio_manager: AudioManager,

    master_track: TrackHandle,
    cue_track: TrackHandle,
    cue_mix_value: f64,

    ch_one_track: TrackHandle,
    sound_one_origin: StaticSoundData,
    sound_one: StaticSoundHandle,
    cue_one_enabled: bool,
    ch_one_volume: f64,
    pitch_one_target: f64,
    pitch_one: f64,
    ch_one_state: ChState,
    ch_one_control: ChControl,

    ch_two_track: TrackHandle,
    sound_two_origin: StaticSoundData,
    sound_two: StaticSoundHandle,
    cue_two_enabled: bool,
    ch_two_volume: f64,
    pitch_two_target: f64,
    pitch_two: f64,
    ch_two_state: ChState,
    ch_two_control: ChControl,
}

impl Mixer {
    pub fn new() -> Self {
        let mut manager =
            AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap();

        let master = manager.add_sub_track(TrackBuilder::new()).unwrap();
        let cue = manager.add_sub_track(TrackBuilder::new()).unwrap();

        let track_one = manager
            .add_sub_track(
                TrackBuilder::new().volume(1.).routes(
                    TrackRoutes::empty()
                        .with_route(&master, 0.0)
                        .with_route(&cue, 0.0),
                ),
            )
            .unwrap();

        let track_two = manager
            .add_sub_track(
                TrackBuilder::new().volume(1.).routes(
                    TrackRoutes::empty()
                        .with_route(&master, 0.0)
                        .with_route(&cue, 0.0),
                ),
            )
            .unwrap();

        let sound_path =
            env::var("SOUND_PATH_ONE").expect("SOUND_PATH_ONE environment variable not set");
        let sound_one_origin = StaticSoundData::from_file(sound_path).unwrap();
        let settings = StaticSoundSettings::new().output_destination(&track_one);
        let sound_one = manager
            .play(sound_one_origin.with_settings(settings))
            .unwrap();

        let sound_path =
            env::var("SOUND_PATH_TWO").expect("SOUND_PATH_TWO environment variable not set");
        let sound_two_origin = StaticSoundData::from_file(sound_path).unwrap();
        let settings = StaticSoundSettings::new().output_destination(&track_two);
        let sound_two = manager
            .play(sound_two_origin.with_settings(settings))
            .unwrap();

        Self {
            audio_manager: manager,
            master_track: master,
            cue_track: cue,
            cue_mix_value: 0.5,

            sound_one_origin: sound_one_origin,
            sound_one: sound_one,
            ch_one_track: track_one,
            cue_one_enabled: false,
            ch_one_volume: 0.0,
            pitch_one_target: 1.0,
            pitch_one: 1.0,
            ch_one_state: ChState::Playing,
            ch_one_control: ChControl::Untouched,

            sound_two_origin: sound_two_origin,
            sound_two: sound_two,
            ch_two_track: track_two,
            cue_two_enabled: false,
            ch_two_volume: 0.0,
            pitch_two_target: 1.0,
            pitch_two: 1.0,
            ch_two_state: ChState::Playing,
            ch_two_control: ChControl::Untouched,
        }
    }

    pub fn load_ch_one(&mut self, path: &PathBuf) {
        match StaticSoundData::from_file(path) {
            Ok(sound_one_origin) => {
                let settings = StaticSoundSettings::new().output_destination(&self.ch_one_track);
                self.sound_one.stop(Tween::default());
                self.sound_one = self
                    .audio_manager
                    .play(sound_one_origin.with_settings(settings))
                    .unwrap();
            }
            Err(e) => eprintln!("Failed to open {:?}: {:?}", path, e),
        }
    }

    pub fn load_ch_two(&mut self, path: &PathBuf) {
        match StaticSoundData::from_file(path) {
            Ok(sound_two_origin) => {
                let settings = StaticSoundSettings::new().output_destination(&self.ch_two_track);
                self.sound_two.stop(Tween::default());
                self.sound_two = self
                    .audio_manager
                    .play(sound_two_origin.with_settings(settings))
                    .unwrap();
            }
            Err(e) => eprintln!("Failed to open {:?}: {:?}", path, e),
        }
    }

    pub fn process(&mut self) {
        match (self.ch_one_state, self.ch_one_control) {
            (_, ChControl::Cueing) => {
                self.pitch_one = lerp(self.pitch_one, 0.0, 0.3);
            }
            (_, ChControl::Seeking) => {
                self.pitch_one = lerp(self.pitch_one, 0.0, 0.3);
            }
            (ChState::Playing, _) => {
                self.pitch_one = lerp(self.pitch_one, self.pitch_one_target, 0.3);
            }
            (ChState::Paused, _) => {
                self.pitch_one = lerp(self.pitch_one, 0.0, 0.3);
            }
        }

        self.sound_one
            .set_playback_rate(self.pitch_one, Tween::default());

        match (self.ch_two_state, self.ch_two_control) {
            (_, ChControl::Cueing) => {
                self.pitch_two = lerp(self.pitch_two, 0.0, 0.3);
            }
            (_, ChControl::Seeking) => {
                self.pitch_two = lerp(self.pitch_two, 0.0, 0.3);
            }
            (ChState::Playing, _) => {
                self.pitch_two = lerp(self.pitch_two, self.pitch_two_target, 0.3);
            }
            (ChState::Paused, _) => {
                self.pitch_two = lerp(self.pitch_two, 0.0, 0.3);
            }
        }

        self.sound_two
            .set_playback_rate(self.pitch_two, Tween::default());
    }

    pub fn get_cue_mix_value(&self) -> f64 {
        self.cue_mix_value
    }

    pub fn set_cue_mix_value(&mut self, value: f64) {
        self.cue_mix_value = value;

        let (cue_volume, master_volume) = Mixer::cue_crossfade(self.cue_mix_value);

        self.cue_track.set_volume(cue_volume, Tween::default());
        self.master_track
            .set_volume(master_volume, Tween::default());
    }

    pub fn is_cue_one_enabled(&self) -> bool {
        self.cue_one_enabled
    }

    pub fn set_cue_one(&mut self, enabled: bool) {
        self.cue_one_enabled = enabled;

        self.ch_one_track
            .set_route(
                &self.cue_track,
                if self.cue_one_enabled { 1.0 } else { 0.0 },
                Tween::default(),
            )
            .unwrap();
    }

    pub fn toggle_start_stop_one(&mut self) {
        match &mut self.ch_one_state {
            ChState::Paused => self.ch_one_state = ChState::Playing,
            ChState::Playing => self.ch_one_state = ChState::Paused,
        }
    }

    pub fn toggle_start_stop_two(&mut self) {
        match &mut self.ch_two_state {
            ChState::Paused => self.ch_two_state = ChState::Playing,
            ChState::Playing => self.ch_two_state = ChState::Paused,
        }
    }

    pub fn set_ch_one_control_state(&mut self, control_state: ChControl) {
        self.ch_one_control = control_state;
    }

    pub fn set_ch_two_control_state(&mut self, control_state: ChControl) {
        self.ch_two_control = control_state;
    }

    pub fn get_duration_one(&self) -> f64 {
        self.sound_one_origin.duration().as_secs_f64()
    }

    pub fn get_duration_two(&self) -> f64 {
        self.sound_two_origin.duration().as_secs_f64()
    }

    pub fn get_position_one(&self) -> f64 {
        self.sound_one.position()
    }

    pub fn get_position_two(&self) -> f64 {
        self.sound_two.position()
    }

    pub fn get_ch_one_volume(&self) -> f64 {
        self.ch_one_volume
    }

    pub fn get_pitch_one(&self) -> f64 {
        self.pitch_one
    }

    pub fn get_pitch_two(&self) -> f64 {
        self.pitch_two
    }

    pub fn get_pitch_one_target(&self) -> f64 {
        self.pitch_one_target
    }

    pub fn set_pitch_one_target(&mut self, pitch: f64) {
        self.pitch_one_target = pitch;
    }

    pub fn get_pitch_two_target(&self) -> f64 {
        self.pitch_two_target
    }

    pub fn set_pitch_two_target(&mut self, pitch: f64) {
        self.pitch_two_target = pitch;
    }

    pub fn set_ch_one_volume(&mut self, volume: f64) {
        self.ch_one_volume = volume;

        self.ch_one_track
            .set_route(&self.master_track, self.ch_one_volume, Tween::default())
            .unwrap();
    }

    pub fn is_cue_two_enabled(&self) -> bool {
        self.cue_two_enabled
    }

    pub fn set_cue_two(&mut self, enabled: bool) {
        self.cue_two_enabled = enabled;

        self.ch_two_track
            .set_route(
                &self.cue_track,
                if self.cue_two_enabled { 1.0 } else { 0.0 },
                Tween::default(),
            )
            .unwrap();
    }

    pub fn touch_one(&mut self, force: f64) {
        match self.ch_one_control {
            ChControl::SoftTouching => {
                self.pitch_one = self.pitch_one - force;
            }
            ChControl::Cueing => {
                // compensate for mouse acceleration profile by applying powf with a number in [0, 1]
                // this an empirical approximation / hack
                self.pitch_one = self.pitch_one - 10.0 * force.signum() * force.abs().powf(0.65);
            }
            ChControl::Seeking => {
                self.pitch_one = self.pitch_one - 400.0 * force;
            }
            ChControl::Untouched => (),
        }
    }

    pub fn touch_two(&mut self, force: f64) {
        match self.ch_two_control {
            ChControl::SoftTouching => {
                self.pitch_two = self.pitch_two - force;
            }
            ChControl::Cueing => {
                // compensate for mouse acceleration profile by applying powf with a number in [0, 1]
                // this an empirical approximation / hack
                self.pitch_two = self.pitch_two - 10.0 * force.signum() * force.abs().powf(0.65);
            }
            ChControl::Seeking => {
                self.pitch_two = self.pitch_two - 400.0 * force;
            }
            ChControl::Untouched => (),
        }
    }

    pub fn get_ch_two_volume(&self) -> f64 {
        self.ch_two_volume
    }

    pub fn set_ch_two_volume(&mut self, volume: f64) {
        self.ch_two_volume = volume;

        self.ch_two_track
            .set_route(&self.master_track, self.ch_two_volume, Tween::default())
            .unwrap();
    }

    /// Explode a given value between 0.0 and 1.0 into respective mixed values.
    /// The sum of the two output values is 1.0
    fn cue_crossfade(norm_value: f64) -> (f64, f64) {
        let norm_value = norm_value.clamp(0.0, 1.0);
        (1. - norm_value, norm_value)
    }
}
