use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use kira::{
    manager::{error::PlaySoundError, AudioManager},
    sound::{
        static_sound::{StaticSoundData, StaticSoundHandle, StaticSoundSettings},
        FromFileError,
    },
    track::TrackHandle,
    tween::Tween,
};

use crate::{processable::Processable, utils::lerp};

/// A struct that simulates a turntable from a digital file.
pub struct Turntable {
    sound_data: Option<StaticSoundData>,
    sound: Option<StaticSoundHandle>,
    audio_manager: Arc<Mutex<AudioManager>>,
    output_destination: Arc<Mutex<TrackHandle>>,
    /// the virtual speed of the vinyl
    pitch_true: f64,
    /// the virtual speed of the platter
    pitch_target: f64,
    is_playing: bool,
    is_scratching: bool,
    /// the current force on the vinyl (to be consumed into pitch variation)
    force: f64,
    currently_loaded: Option<String>,
}

#[derive(Debug)]
pub enum LoadError {
    FromFile(FromFileError),
    Play(PlaySoundError<()>),
    IsPlaying,
}

impl From<FromFileError> for LoadError {
    fn from(error: FromFileError) -> Self {
        LoadError::FromFile(error)
    }
}

impl From<PlaySoundError<()>> for LoadError {
    fn from(error: PlaySoundError<()>) -> Self {
        LoadError::Play(error)
    }
}

#[derive(Debug)]
pub enum SeekError {
    EmptyDuration,
    EmptySound,
}

impl Turntable {
    /// Creates a new instance of a turntable
    pub fn new(
        audio_manager: Arc<Mutex<AudioManager>>,
        output_destination: Arc<Mutex<TrackHandle>>,
    ) -> Self {
        Self {
            sound_data: None,
            sound: None,
            audio_manager: audio_manager,
            output_destination: output_destination,
            pitch_true: 0.0,
            pitch_target: 1.0,
            is_playing: false,
            is_scratching: false,
            force: 0.0,
            currently_loaded: None,
        }
    }

    /// Load an audio file into the turntable
    pub fn load(&mut self, path: &Path) -> Result<(), LoadError> {
        if self.is_playing {
            return Err(LoadError::IsPlaying);
        }

        self.sound_data = match StaticSoundData::from_file(path) {
            Ok(sound_data) => Some(sound_data),
            Err(e) => return Err(LoadError::FromFile(e)),
        };

        if let Some(sound) = &mut self.sound {
            sound.stop(Tween::default());
        }

        let settings = StaticSoundSettings::new()
            .output_destination(&*self.output_destination.lock().unwrap());

        if let Some(sound_data) = &mut self.sound_data {
            self.sound = match self
                .audio_manager
                .lock()
                .unwrap()
                .play(sound_data.with_settings(settings))
            {
                Ok(sound) => Some(sound),
                Err(e) => return Err(LoadError::Play(e)),
            };
        }

        self.currently_loaded = Some(path.to_string_lossy().to_string());

        Ok(())
    }

    pub fn currently_loaded(&self) -> Option<String> {
        self.currently_loaded.clone()
    }

    pub fn pitch(&self) -> f64 {
        self.pitch_target
    }

    pub fn position(&self) -> Option<f64> {
        match &self.sound {
            Some(sound) => Some(sound.position()),
            None => None,
        }
    }

    pub fn duration(&self) -> Option<f64> {
        match &self.sound_data {
            Some(sound_data) => Some(sound_data.duration().as_secs_f64()),
            None => None,
        }
    }

    pub fn toggle_start_stop(&mut self) {
        self.is_playing = !self.is_playing;
    }

    /// Set the pitch of the turntable.
    /// The value is clamped in the range [0.92, 1.08], i.e. +-8%
    pub fn set_pitch(&mut self, pitch: f64) {
        self.pitch_target = pitch.clamp(0.92, 1.08)
    }

    pub fn start_scratching(&mut self) {
        self.is_scratching = true;
    }

    pub fn end_scratching(&mut self) {
        self.is_scratching = false;
    }

    pub fn apply_force(&mut self, force: f64) {
        self.force += force;
    }

    pub fn seek(&mut self, percent: f64) -> Result<(), SeekError> {
        let duration = self.duration().ok_or(SeekError::EmptyDuration)?;
        let sound = self.sound.as_mut().ok_or(SeekError::EmptySound)?;

        sound.seek_to(percent * duration);

        Ok(())
    }
}

impl Processable for Turntable {
    fn process(&mut self, delta: f64) {
        let force = self.force * 0.02 / delta;

        let pitch_per_state = match (self.is_playing, self.is_scratching) {
            (false, false) => 0.0 + 0.01 * force,
            (true, false) => self.pitch_target + 0.01 * force,
            (_, true) => 0.1 * force,
        };

        self.pitch_true = lerp(self.pitch_true, pitch_per_state, 0.8 * 0.02 / delta);

        if let Some(sound) = &mut self.sound {
            sound.set_playback_rate(self.pitch_true, Tween::default());
        }

        self.force = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use kira::{
        manager::{AudioManager, AudioManagerSettings, DefaultBackend},
        track::TrackBuilder,
    };

    use super::*;

    #[test]
    fn test_load() {
        let audio_manager = Arc::new(Mutex::new(
            AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap(),
        ));

        let track = Arc::new(Mutex::new(
            audio_manager
                .lock()
                .unwrap()
                .add_sub_track(TrackBuilder::new())
                .unwrap(),
        ));

        let mut turntable = Turntable::new(audio_manager, track);

        let result = turntable.load(Path::new("assets/test_file01.mp3"));

        assert!(result.is_ok());
    }

    #[test]
    fn test_duration() {
        let audio_manager = Arc::new(Mutex::new(
            AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap(),
        ));

        let track = Arc::new(Mutex::new(
            audio_manager
                .lock()
                .unwrap()
                .add_sub_track(TrackBuilder::new())
                .unwrap(),
        ));

        let mut turntable = Turntable::new(audio_manager, track);

        let _ = turntable.load(Path::new("assets/test_file01.mp3"));

        assert_eq!(turntable.duration(), Some(85.681632653));
    }

    #[test]
    fn test_position() {
        let audio_manager = Arc::new(Mutex::new(
            AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap(),
        ));

        let track = Arc::new(Mutex::new(
            audio_manager
                .lock()
                .unwrap()
                .add_sub_track(TrackBuilder::new())
                .unwrap(),
        ));

        let mut turntable = Turntable::new(audio_manager, track);

        let _ = turntable.load(Path::new("assets/test_file01.mp3"));

        assert_eq!(turntable.position(), Some(0.0));
    }

    #[test]
    fn test_start_scratching() {
        let audio_manager = Arc::new(Mutex::new(
            AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap(),
        ));

        let track = Arc::new(Mutex::new(
            audio_manager
                .lock()
                .unwrap()
                .add_sub_track(TrackBuilder::new())
                .unwrap(),
        ));

        let mut turntable = Turntable::new(audio_manager, track);
        turntable.start_scratching();

        assert_eq!(turntable.is_scratching, true);
    }

    #[test]
    fn test_toggle_start_stop() {
        let audio_manager = Arc::new(Mutex::new(
            AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap(),
        ));

        let track = Arc::new(Mutex::new(
            audio_manager
                .lock()
                .unwrap()
                .add_sub_track(TrackBuilder::new())
                .unwrap(),
        ));

        let mut turntable = Turntable::new(audio_manager, track);
        turntable.toggle_start_stop();

        assert_eq!(turntable.is_playing, true);

        turntable.toggle_start_stop();

        assert_eq!(turntable.is_playing, false);
    }

    #[test]
    fn test_end_scratching() {
        let audio_manager = Arc::new(Mutex::new(
            AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap(),
        ));

        let track = Arc::new(Mutex::new(
            audio_manager
                .lock()
                .unwrap()
                .add_sub_track(TrackBuilder::new())
                .unwrap(),
        ));

        let mut turntable = Turntable::new(audio_manager, track);
        turntable.end_scratching();

        assert_eq!(turntable.is_scratching, false);
    }

    #[test]
    fn test_apply_force() {
        let audio_manager = Arc::new(Mutex::new(
            AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap(),
        ));

        let track = Arc::new(Mutex::new(
            audio_manager
                .lock()
                .unwrap()
                .add_sub_track(TrackBuilder::new())
                .unwrap(),
        ));

        let mut turntable = Turntable::new(audio_manager, track);
        turntable.apply_force(42.0);
        turntable.apply_force(-69.0);

        assert_eq!(turntable.force, 42.0 - 69.0);
    }
}
