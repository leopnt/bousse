use std::env;

use kira::{
    manager::{AudioManager, AudioManagerSettings, DefaultBackend},
    sound::{
        streaming::{StreamingSoundData, StreamingSoundHandle, StreamingSoundSettings},
        FromFileError,
    },
    track::{TrackBuilder, TrackHandle, TrackRoutes},
    tween::Tween,
};

pub struct Mixer {
    audio_manager: AudioManager,
    master_track: TrackHandle,
    cue_track: TrackHandle,
    ch_one_track: TrackHandle,
    sound_one: StreamingSoundHandle<FromFileError>,
    cue_mix_value: f64,
    cue_one_enabled: bool,
    ch_one_volume: f64,
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

        let settings = StreamingSoundSettings::new().output_destination(&track_one);

        let sound_path = env::var("SOUND_PATH").expect("SOUND_PATH environment variable not set");
        let sound = StreamingSoundData::from_file(sound_path).unwrap();

        let mut sound_one = manager.play(sound.with_settings(settings)).unwrap();

        Self {
            audio_manager: manager,
            sound_one: sound_one,
            master_track: master,
            cue_track: cue,
            ch_one_track: track_one,
            cue_mix_value: 0.5,
            cue_one_enabled: false,
            ch_one_volume: 0.0,
        }
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

    pub fn get_ch_one_volume(&self) -> f64 {
        self.ch_one_volume
    }

    pub fn set_ch_one_volume(&mut self, volume: f64) {
        self.ch_one_volume = volume;

        self.ch_one_track
            .set_route(&self.master_track, self.ch_one_volume, Tween::default())
            .unwrap();
    }

    /// Explode a given value between 0.0 and 1.0 into respective mixed values.
    /// The sum of the two output values is 1.0
    fn cue_crossfade(norm_value: f64) -> (f64, f64) {
        let norm_value = norm_value.clamp(0.0, 1.0);
        (1. - norm_value, norm_value)
    }
}