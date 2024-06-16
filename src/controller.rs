use std::path::Path;

use crate::{app::AppData, file_navigator::FileNavigatorSelection, utils::to_cover_path};

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
    EqLowOneChanged(f64),
    EqHighOneChanged(f64),
    EqLowTwoChanged(f64),
    EqHighTwoChanged(f64),
    SeekOne(f64),
    SeekTwo(f64),
    FileNavigatorDown,
    FileNavigatorUp,
    FileNavigatorSelect,
    FileNavigatorBack,
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
                app_data.turntable_one.load(path).unwrap();

                app_data
                    .cover_one
                    .load_image_data(&to_cover_path(&path.to_string_lossy().to_string()));
            }
            (BoothEvent::TrackLoad(path), TurntableFocus::Two) => {
                app_data.turntable_two.load(path).unwrap();

                app_data
                    .cover_two
                    .load_image_data(&to_cover_path(&path.to_string_lossy().to_string()));
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
            (BoothEvent::EqLowOneChanged(gain), _) => {
                app_data.mixer.set_eq_low_one_gain(*gain);
            }
            (BoothEvent::EqHighOneChanged(gain), _) => {
                app_data.mixer.set_eq_high_one_gain(*gain);
            }
            (BoothEvent::EqLowTwoChanged(gain), _) => {
                app_data.mixer.set_eq_low_two_gain(*gain);
            }
            (BoothEvent::EqHighTwoChanged(gain), _) => {
                app_data.mixer.set_eq_high_two_gain(*gain);
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
                match app_data.turntable_one.seek(*percent) {
                    Ok(()) => (),
                    Err(e) => log::error!("Cannot seek track one: {:?}", e),
                };
            }
            (BoothEvent::SeekTwo(percent), _) => {
                match app_data.turntable_two.seek(*percent) {
                    Ok(()) => (),
                    Err(e) => log::error!("Cannot seek track two: {:?}", e),
                };
            }
            (BoothEvent::FileNavigatorUp, _) => {
                app_data.file_navigator.go_up();
            }
            (BoothEvent::FileNavigatorDown, _) => {
                app_data.file_navigator.go_down();
            }
            (BoothEvent::FileNavigatorBack, _) => match app_data.file_navigator.go_back() {
                Err(e) => log::error!("{}", e),
                _ => (),
            },
            (BoothEvent::FileNavigatorSelect, TurntableFocus::One) => {
                match app_data.file_navigator.select() {
                    FileNavigatorSelection::File(file_path) => {
                        self.handle_event(app_data, BoothEvent::TrackLoad(Path::new(&file_path)));
                    }
                    _ => (),
                }
            }
            (BoothEvent::FileNavigatorSelect, TurntableFocus::Two) => {
                match app_data.file_navigator.select() {
                    FileNavigatorSelection::File(file_path) => {
                        self.handle_event(app_data, BoothEvent::TrackLoad(Path::new(&file_path)));
                    }
                    _ => (),
                }
            }
        }
    }
}
