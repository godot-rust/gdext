/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::error::Error;
use std::fmt;

use crate::classes::FileAccess;
use crate::global::Error as GodotError;
use crate::obj::Gd;

/// Error that can occur while using `gdext` IO utilities.
#[derive(Debug)]
pub struct IoError {
    data: ErrorData,
}

impl fmt::Display for IoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.data {
            ErrorData::Load(err) => err.fmt(f),
            ErrorData::Save(err) => err.fmt(f),
            ErrorData::GFile(err) => err.fmt(f),
        }
    }
}

impl Error for IoError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        // Note: inner types are not public, but the dyn trait can be used.
        match &self.data {
            ErrorData::Load(err) => Some(err),
            ErrorData::Save(err) => Some(err),
            ErrorData::GFile(err) => Some(err),
        }
    }
}

impl IoError {
    pub(crate) fn saving(error: GodotError, class: String, path: String) -> Self {
        Self {
            data: ErrorData::Save(SaverError {
                class,
                path,
                godot_error: error,
            }),
        }
    }

    pub(crate) fn loading(class: String, path: String) -> Self {
        Self {
            data: ErrorData::Load(LoaderError {
                kind: LoaderErrorKind::Load,
                class,
                path,
            }),
        }
    }

    pub(crate) fn loading_cast(class: String, path: String) -> Self {
        Self {
            data: ErrorData::Load(LoaderError {
                kind: LoaderErrorKind::Cast,
                class,
                path,
            }),
        }
    }

    pub(crate) fn check_unique_open_file_access(
        file_access: Gd<FileAccess>,
    ) -> Result<Gd<FileAccess>, Self> {
        let path = file_access.get_path();

        if !file_access.is_open() {
            return Err(Self {
                data: ErrorData::GFile(GFileError {
                    kind: GFileErrorKind::NotOpen,
                    path: path.to_string(),
                }),
            });
        }

        match file_access.try_to_unique() {
            Ok(gd) => Ok(gd),
            Err((_drop, ref_count)) => Err(Self {
                data: ErrorData::GFile(GFileError {
                    kind: GFileErrorKind::NotUniqueRef { ref_count },
                    path: path.to_string(),
                }),
            }),
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
enum ErrorData {
    Load(LoaderError),
    Save(SaverError),
    GFile(GFileError),
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
struct LoaderError {
    kind: LoaderErrorKind,
    class: String,
    path: String,
}

#[derive(Debug)]
enum LoaderErrorKind {
    Load,
    Cast,
}

impl Error for LoaderError {}

impl fmt::Display for LoaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let class = &self.class;
        let path = &self.path;

        match &self.kind {
            LoaderErrorKind::Load => write!(
                f,
                "can't load resource of class: '{class}' from path: '{path}'"
            ),
            LoaderErrorKind::Cast => write!(
                f,
                "can't cast loaded resource to class: '{class}' from path: '{path}'"
            ),
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
struct SaverError {
    class: String,
    path: String,
    godot_error: GodotError,
}

impl Error for SaverError {}

impl fmt::Display for SaverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let class = &self.class;
        let path = &self.path;
        let godot_error = &self.godot_error;

        write!(f, "can't save resource of class: '{class}' to path: '{path}'; Godot error: {godot_error:?}")
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
struct GFileError {
    kind: GFileErrorKind,
    path: String,
}

#[derive(Debug)]
enum GFileErrorKind {
    NotUniqueRef { ref_count: usize },
    NotOpen,
}

impl Error for GFileError {}

impl fmt::Display for GFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path = &self.path;

        match &self.kind {
            GFileErrorKind::NotUniqueRef { ref_count } => {
                write!(
                    f,
                    "Gd<FileAccess> for '{path}' is not unique (ref-count {ref_count})"
                )
            }
            GFileErrorKind::NotOpen => write!(f, "access to file '{path}' is not open"),
        }
    }
}
