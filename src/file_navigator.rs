use core::fmt;
use std::{ffi::OsStr, fs, path::Path};

#[derive(Debug)]
pub enum FileNavigatorSelection {
    File(String),      // selected audio file with path
    Directory(String), // selected directory with path
    None,              // selected nothing
}

#[derive(Debug)]
pub enum FileNavigatorError {
    PastRootAttempt,
}

impl fmt::Display for FileNavigatorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            FileNavigatorError::PastRootAttempt => {
                write!(f, "Cannot go back at root dir")
            }
        }
    }
}

pub struct FileNavigator {
    cwd_stack: Vec<String>,
    entries: Vec<String>,
    cursor_stack: Vec<usize>,
}

impl FileNavigator {
    pub fn new(starting_folder: &String) -> Self {
        let mut file_navigator = Self {
            cwd_stack: vec![starting_folder.clone()],
            entries: Vec::new(),
            cursor_stack: Vec::new(),
        };

        file_navigator.update_entries();

        file_navigator
    }

    pub fn go_up(&mut self) {
        if let Some(cursor) = self.cursor() {
            if self.entries.len() > 0 {
                let new_cursor = *cursor as isize - 1;
                let new_cursor = new_cursor.rem_euclid(self.entries.len() as isize);

                self.set_cursor(new_cursor as usize);
            }
        }
    }

    pub fn go_down(&mut self) {
        if let Some(cursor) = self.cursor() {
            if self.entries.len() > 0 {
                let new_cursor = cursor + 1;
                let new_cursor = new_cursor.rem_euclid(self.entries.len());

                self.set_cursor(new_cursor);
            }
        }
    }

    fn is_supported_audio_filename(filename: &String) -> bool {
        match Path::new(filename)
            .extension()
            .and_then(OsStr::to_str)
            .map(|s| s.to_ascii_lowercase())
            .as_deref()
        {
            Some("wav") => true,
            Some("aif") => true,
            Some("aiff") => true,
            Some("flac") => true,
            Some("mp3") => true,
            _ => false,
        }
    }

    fn update_entries(&mut self) {
        self.entries.clear();
        if let Ok(paths) = fs::read_dir(self.cwd()) {
            for path in paths {
                if let Ok(entry) = path {
                    if let Ok(name) = entry.file_name().into_string() {
                        let full_path = vec![self.cwd(), name.clone()].join("/");

                        match fs::metadata(&full_path) {
                            Ok(metadata) => {
                                if metadata.is_dir() {
                                    self.entries.push(name);
                                } else if metadata.is_file()
                                    && FileNavigator::is_supported_audio_filename(&name)
                                {
                                    self.entries.push(name);
                                }
                            }
                            Err(e) => log::error!("Metadata error: {:?}, '{}'", e, full_path),
                        }
                    }
                }
            }
        }

        self.entries.sort()
    }

    pub fn select(&mut self) -> FileNavigatorSelection {
        match self.cursor() {
            None => {
                if let Some(_) = self.entries().first() {
                    self.cursor_stack.push(0);
                }
            }
            Some(cursor) => {
                if let Some(entry) = self.entries().get(*cursor) {
                    let file_path = vec![self.cwd(), entry.clone()].join("/");

                    if FileNavigator::is_supported_audio_filename(entry) {
                        return FileNavigatorSelection::File(file_path);
                    }

                    let out = FileNavigatorSelection::Directory(file_path.clone());

                    self.cwd_stack.push(entry.clone());
                    self.cursor_stack.push(0);
                    self.update_entries();

                    return out;
                }
            }
        }

        FileNavigatorSelection::None
    }

    pub fn go_back(&mut self) -> Result<(), FileNavigatorError> {
        match self.cwd_stack.len() {
            1 => {
                return Err(FileNavigatorError::PastRootAttempt);
            }
            _ => {
                self.cwd_stack.pop();
                self.cursor_stack.pop();
            }
        }

        self.update_entries();
        Ok(())
    }

    fn cursor(&self) -> Option<&usize> {
        self.cursor_stack.last()
    }

    fn set_cursor(&mut self, new_cursor: usize) {
        if let Some(cursor) = self.cursor_stack.last_mut() {
            *cursor = new_cursor;
        }
    }

    pub fn cwd(&self) -> String {
        self.cwd_stack.join("/")
    }

    pub fn entries(&self) -> &Vec<String> {
        &self.entries
    }

    pub fn selected(&self) -> Option<&String> {
        match self.cursor() {
            Some(cursor) => Some(&self.entries[*cursor]),
            None => None,
        }
    }
}
