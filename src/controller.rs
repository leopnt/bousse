use std::path::Path;

use crate::app::AppData;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TurntableFocus {
    One,
    Two,
}

#[derive(Debug)]
pub enum BoothEvent<'a> {
    FocusChanged(TurntableFocus),
    TrackLoad(&'a Path),
    CueMixChanged(f64),
    ForceApplied(f64),
    ToggleDebug,
    ScratchBegin,
    ScratchEnd,
    ToggleStartStopOne,
    ToggleStartStopTwo,
    ToggleCueOne,
    ToggleCueTwo,
    VolumeOneChanged(f64),
    VolumeTwoChanged(f64),
    PitchOneChanged(f64),
    PitchTwoChanged(f64),
    SeekOne(f32),
}

pub struct Controller {}

impl Controller {
    pub fn new() -> Self {
        Self {}
    }

    pub fn handle_event(&self, app_data: &mut AppData, event: BoothEvent) {
        match (&event, &mut app_data.turntable_focus) {
            (BoothEvent::FocusChanged(focus), _) => app_data.turntable_focus = *focus,
            (BoothEvent::ToggleDebug, _) => app_data.show_debug_panel = !app_data.show_debug_panel,
            (BoothEvent::CueMixChanged(mix), _) => app_data.mixer.set_cue_mix_value(*mix),
            (BoothEvent::TrackLoad(path), TurntableFocus::One) => {
                app_data.turntable_one.load(path).unwrap()
            }
            (BoothEvent::TrackLoad(path), TurntableFocus::Two) => {
                app_data.turntable_two.load(path).unwrap()
            }
            (BoothEvent::ToggleStartStopOne, _) => app_data.turntable_one.toggle_start_stop(),
            (BoothEvent::ToggleStartStopTwo, _) => app_data.turntable_two.toggle_start_stop(),
            (BoothEvent::ToggleCueOne, _) => {
                let cue = app_data.mixer.is_cue_one_enabled();
                app_data.mixer.set_cue_one(!cue);
            }
            (BoothEvent::ToggleCueTwo, _) => {
                let cue = app_data.mixer.is_cue_two_enabled();
                app_data.mixer.set_cue_two(!cue);
            }
            (BoothEvent::VolumeOneChanged(volume), _) => {
                app_data.mixer.set_ch_one_volume(*volume);
            }
            (BoothEvent::VolumeTwoChanged(volume), _) => {
                app_data.mixer.set_ch_two_volume(*volume);
            }
            (BoothEvent::PitchOneChanged(pitch), _) => {
                app_data.turntable_one.set_pitch(*pitch);
            }
            (BoothEvent::PitchTwoChanged(pitch), _) => {
                app_data.turntable_two.set_pitch(*pitch);
            }
            (BoothEvent::ScratchBegin, TurntableFocus::One) => {
                app_data.turntable_one.start_scratching();
            }
            (BoothEvent::ScratchEnd, TurntableFocus::One) => {
                app_data.turntable_one.end_scratching();
            }
            (BoothEvent::ScratchBegin, TurntableFocus::Two) => {
                app_data.turntable_two.start_scratching();
            }
            (BoothEvent::ScratchEnd, TurntableFocus::Two) => {
                app_data.turntable_two.end_scratching();
            }
            (BoothEvent::ForceApplied(force), TurntableFocus::One) => {
                app_data.turntable_one.apply_force(*force);
            }
            (BoothEvent::ForceApplied(force), TurntableFocus::Two) => {
                app_data.turntable_two.apply_force(*force);
            }
            (BoothEvent::SeekOne(percent), _) => {
                app_data.turntable_one.seek(*percent);
            }
        }
    }
}
