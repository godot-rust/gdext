/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cmp;
use std::io::{BufRead, ErrorKind, Read, Seek, SeekFrom, Write};

use crate::builtin::{real, GString, PackedByteArray, PackedStringArray, Variant};
use crate::classes::file_access::{CompressionMode, ModeFlags};
use crate::classes::FileAccess;
use crate::global::Error;
use crate::meta::error::IoError;
use crate::meta::{arg_into_ref, AsArg};
use crate::obj::Gd;

/// Open a file for reading or writing.
///
/// This is a convenient wrapper around a [`FileAccess`] pointer with a unique reference, providing both safety and
/// quality-of-life upgrades over the inner type:
///  
/// - Exposes reading and writing capabilities of `FileAccess` in a safe way, returning [`Result<T>`](std::io::Result)
///   instead of the `T` itself.
/// - Makes the `FileAccess` handle exclusive to its instance, disallowing parallel reads and writes, which could introduce
///   hard-to-track bugs due to unpredictable cursor movement. Exclusivity also ensures that when the `GFile` moves out
///   of scope, the inner `FileAccess` does as well, automatically closing the file. Alternatively, you can [`drop()`]
///   the `GFile` to close the file manually.
/// - Implements useful Rust traits, namely: [`Read`], [`BufRead`], [`Write`], [`Seek`], allowing better file processing
///   and integrating it with various tools in the Rust ecosystem (e.g. serialization).
///
/// Files by default are always opened with little-endian, as most files are saved as such. To switch to big-endian, use
/// [`GFile::set_big_endian()`].
///
/// ## [`ModeFlags`]
///
/// Every constructor opening the access to a file (`open_*` associated functions) accepts the `flags` parameter,
/// opening the file for different types of operations. Regardless of the provided `flags` value, the cursor is always
/// positioned at the beginning of the file upon opening. To adjust its position, use [`Seek`]-provided methods.
///
/// - `ModeFlags::READ` opens the file for read operations.
/// - `ModeFlags::WRITE` opens the file for write operations. If the file doesn't exist at the provided `path`, it is
///   created. If it exists, it is truncated after the file is closed.
/// - `ModeFlags::READ_WRITE` opens the file for read and write operations. The file is not truncated after closing.
/// - `ModeFlags::WRITE_READ` opens the file for read and write operations. If the file doesn't exist at the provided
///   `path`, it is created. If it exists, it is truncated.
///
/// ## Examples
///
/// ```no_run
/// use godot::builtin::GString;
/// use godot::classes::file_access::ModeFlags;
/// use godot::tools::GFile;
///
/// fn save_game() -> Result<(), std::io::Error> {
///
///     // Open file in write mode
///     let mut my_file = GFile::open("user://save_game.sav", ModeFlags::WRITE)?;
///
///     // Write some lines into it
///     my_file.write_gstring_line("This is my saved game")?;
///     my_file.write_gstring_line("I played for 5 minutes")?;
///
///     Ok(())
///     // When GFile gets out of scope, the file is closed.
/// }
///
/// fn load_game() -> Result<(), std::io::Error> {
///
///     // Open file in read mode
///     let mut my_file = GFile::open("user://save_game.sav", ModeFlags::READ)?;
///
///     // Read lines
///     let first_line = my_file.read_gstring_line()?;
///     assert_eq!(first_line, GString::from("This is my saved game"));
///
///     let second_line = my_file.read_gstring_line()?;
///     assert_eq!(second_line, GString::from("I played for 5 minutes"));
///
///     Ok(())
/// }
/// ```
///
/// ## See also
///
/// - [`FileAccess`] class in Rust.
/// - [Godot documentation](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html) for `FileAccess`.
pub struct GFile {
    fa: Gd<FileAccess>,
    buffer: Vec<u8>,
    last_buffer_size: usize,
    write_buffer: PackedByteArray,
    file_length: Option<u64>,
}

impl GFile {
    // For now - only used internally in BufRead implementation. If needed, its setting could be exposed in some way.
    const BUFFER_SIZE: usize = 4096;

    /// Open a file.
    ///
    /// Opens a file located at `path`, creating new [`GFile`] object. For [`ModeFlags`] description check the [`GFile`]
    /// documentation.
    pub fn open(path: impl AsArg<GString>, flags: ModeFlags) -> std::io::Result<Self> {
        arg_into_ref!(path);

        let fa = FileAccess::open(path, flags).ok_or_else(|| {
            std::io::Error::other(format!(
                "can't open file {path} in mode {flags:?}; GodotError: {:?}",
                FileAccess::get_open_error()
            ))
        })?;

        Ok(Self::from_inner(fa))
    }

    /// Open a compressed file.
    ///
    /// Opens a compressed file located at `path`, creating new [`GFile`] object. Can read only files compressed by
    /// Godot compression formats. For [`ModeFlags`] description check the [`GFile`] documentation.
    pub fn open_compressed(
        path: impl AsArg<GString>,
        flags: ModeFlags,
        compression_mode: CompressionMode,
    ) -> std::io::Result<Self> {
        arg_into_ref!(path);

        let fa = FileAccess::open_compressed_ex(path, flags)
            .compression_mode(compression_mode)
            .done()
            .ok_or_else(|| {
                std::io::Error::other(format!(
                    "can't open file {path} in mode {flags:?}; GodotError: {:?}",
                    FileAccess::get_open_error()
                ))
            })?;

        Ok(Self::from_inner(fa))
    }

    /// Open a file encrypted by byte key.
    ///
    /// Opens a file encrypted by 32-byte long [`PackedByteArray`] located at `path`, creating new [`GFile`] object.
    /// For [`ModeFlags`] description check the [`GFile`] documentation.
    pub fn open_encrypted(
        path: impl AsArg<GString>,
        flags: ModeFlags,
        key: &PackedByteArray,
    ) -> std::io::Result<Self> {
        arg_into_ref!(path);

        let fa = FileAccess::open_encrypted(path, flags, key).ok_or_else(|| {
            std::io::Error::other(format!(
                "can't open file {path} in mode {flags:?}; GodotError: {:?}",
                FileAccess::get_open_error()
            ))
        })?;

        Ok(Self::from_inner(fa))
    }

    /// Open a file encrypted by password.
    ///
    /// Opens a file encrypted by a `password` located at `path`, creating new [`GFile`] object. For [`ModeFlags`]
    /// description check the [`GFile`] documentation.
    pub fn open_encrypted_with_pass(
        path: impl AsArg<GString>,
        flags: ModeFlags,
        password: impl AsArg<GString>,
    ) -> std::io::Result<Self> {
        arg_into_ref!(path);
        arg_into_ref!(password);

        let fa = FileAccess::open_encrypted_with_pass(path, flags, password).ok_or_else(|| {
            std::io::Error::other(format!(
                "can't open file {path} in mode {flags:?}; GodotError: {:?}",
                FileAccess::get_open_error()
            ))
        })?;
        Ok(Self::from_inner(fa))
    }

    /// Creates new [`GFile`] from a [`FileAccess`] pointer with a reference count of 1.
    ///
    /// For this method to work, the provided `file_access` must be unique -- no other reference to it can exist.
    /// Its state is retained: both [`ModeFlags`] with which it was open and current internal cursor position.
    ///
    /// See also [`into_inner`](Self::into_inner) for the opposite operation.
    pub fn try_from_unique(file_access: Gd<FileAccess>) -> Result<Self, IoError> {
        let file_access = IoError::check_unique_open_file_access(file_access)?;
        Ok(Self::from_inner(file_access))
    }

    /// Retrieve inner pointer to the [`FileAccess`].
    ///
    /// This instance of `GFile` will be destroyed, but the file is kept open as long as there is at least one reference
    /// pointing to it. Its state is retained: both [`ModeFlags`] with which it was opened and current internal cursor position.
    ///
    /// See also [`try_from_unique`](Self::try_from_unique) for the opposite operation.
    pub fn into_inner(self) -> Gd<FileAccess> {
        self.fa
    }

    // ----------------------------------------------------------------------------------------------------------------------------------------------
    // Remaps of the internal FileAccess methods.

    /// Get last modified time as a Unix timestamp.
    #[doc(alias = "get_modified_time")]
    pub fn modified_time(path: impl AsArg<GString>) -> std::io::Result<u64> {
        arg_into_ref!(path);
        let modified_time = FileAccess::get_modified_time(path);

        if modified_time == 0 {
            Err(std::io::Error::other(format!(
                "can't retrieve last modified time: {path}"
            )))
        } else {
            Ok(modified_time)
        }
    }

    /// Calculates the MD5 checksum of the file at the given path.
    #[doc(alias = "get_md5")]
    pub fn md5(path: impl AsArg<GString>) -> std::io::Result<GString> {
        arg_into_ref!(path);
        let md5 = FileAccess::get_md5(path);

        if md5.is_empty() {
            Err(std::io::Error::other(format!(
                "failed to compute file's MD5 checksum: {path}"
            )))
        } else {
            Ok(md5)
        }
    }

    /// Calculates the SHA-256 checksum of the file at the given path.
    #[doc(alias = "get_sha256")]
    pub fn sha256(path: impl AsArg<GString>) -> std::io::Result<GString> {
        arg_into_ref!(path);
        let sha256 = FileAccess::get_sha256(path);

        if sha256.is_empty() {
            Err(std::io::Error::other(format!(
                "failed to compute file's SHA-256 checksum: {path}"
            )))
        } else {
            Ok(sha256)
        }
    }

    /// Reads the next byte from the file as [`u8`].
    ///
    /// Underlying Godot method:
    /// [`FileAccess::get_8`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-get-8).
    #[doc(alias = "get_8")]
    pub fn read_u8(&mut self) -> std::io::Result<u8> {
        let val = self.fa.get_8();
        self.check_error()?;
        Ok(val)
    }

    /// Reads the next 2 bytes from the file as [`u16`].
    ///
    /// Underlying Godot method:
    /// [`FileAccess::get_16`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-get-16).
    #[doc(alias = "get_16")]
    pub fn read_u16(&mut self) -> std::io::Result<u16> {
        let val = self.fa.get_16();
        self.check_error()?;
        Ok(val)
    }

    /// Reads the next 4 bytes from the file as [`u32`].
    ///
    /// Underlying Godot method:
    /// [`FileAccess::get_32`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-get-32).
    #[doc(alias = "get_32")]
    pub fn read_u32(&mut self) -> std::io::Result<u32> {
        let val = self.fa.get_32();
        self.check_error()?;
        Ok(val)
    }

    /// Reads the next 8 bytes from the file as [`u64`].
    ///
    /// Underlying Godot method:
    /// [`FileAccess::get_64`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-get-64).
    #[doc(alias = "get_64")]
    pub fn read_u64(&mut self) -> std::io::Result<u64> {
        let val = self.fa.get_64();
        self.check_error()?;
        Ok(val)
    }

    /// Reads a Pascal string (length-prefixed) from the current position.
    ///
    /// A _Pascal string_ is useful for writing and retrieving variable-length string data from binary files. It is saved with a
    /// length prefix (as opposed to C strings, which end with a null terminator). Text is interpreted as UTF-8 encoded.
    ///
    /// See also:
    /// - [Wikipedia article](https://en.wikipedia.org/wiki/String_(computer_science)#Length-prefixed)
    /// - [Godot `FileAccess::get_pascal_string`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-get-pascal-string)
    #[doc(alias = "get_pascal_string")]
    pub fn read_pascal_string(&mut self) -> std::io::Result<GString> {
        let val = self.fa.get_pascal_string();
        self.check_error()?;
        Ok(val)
    }

    /// Reads the next line of the file as [`GString`].
    ///
    /// To retrieve the file as [`String`] instead, use the [`Read`] trait method
    /// [`read_to_string()`](https://doc.rust-lang.org/std/io/trait.Read.html#method.read_to_string).
    ///
    /// Underlying Godot method:
    /// [`FileAccess::get_line`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-get-line).
    #[doc(alias = "get_line")]
    pub fn read_gstring_line(&mut self) -> std::io::Result<GString> {
        let val = self.fa.get_line();
        self.check_error()?;
        Ok(val)
    }

    /// Reads the whole file as UTF-8 [`GString`].
    ///
    /// If `skip_cr` is set to `true`, carriage return (`'\r'`) will be ignored, and only line feed (`'\n'`) indicates a new line.
    ///
    /// To retrieve the file as [`String`] instead, use the [`Read`] trait method
    /// [`read_to_string()`](https://doc.rust-lang.org/std/io/trait.Read.html#method.read_to_string).
    ///
    /// Underlying Godot method:
    /// [`FileAccess::get_as_text`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-get-as-text).
    // For Godot versions before `skip_cr` has been removed, see: https://github.com/godotengine/godot/pull/110867.
    #[doc(alias = "get_as_text")]
    #[cfg(before_api = "4.6")]
    pub fn read_as_gstring_entire(&mut self, skip_cr: bool) -> std::io::Result<GString> {
        let val = self.fa.get_as_text_ex().skip_cr(skip_cr).done();
        self.check_error()?;
        Ok(val)
    }

    /// Reads the whole file as UTF-8 [`GString`].
    ///
    /// To retrieve the file as [`String`] instead, use the [`Read`] trait method
    /// [`read_to_string()`](https://doc.rust-lang.org/std/io/trait.Read.html#method.read_to_string).
    ///
    /// Underlying Godot method:
    /// [`FileAccess::get_as_text`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-get-as-text).
    #[doc(alias = "get_as_text")]
    #[cfg(since_api = "4.6")]
    pub fn read_as_gstring_entire(&mut self) -> std::io::Result<GString> {
        let val = self.fa.get_as_text();
        self.check_error()?;
        Ok(val)
    }

    /// Reads the next line of the file in delimiter-separated file.
    ///
    /// For reading traditional `CSV` format, provide comma (`','`) as `delim`.
    ///
    /// Underlying Godot method:
    /// [`FileAccess::get_csv_line`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-get-csv-line).
    #[doc(alias = "get_csv_line")]
    pub fn read_csv_line(
        &mut self,
        delim: impl AsArg<GString>,
    ) -> std::io::Result<PackedStringArray> {
        arg_into_ref!(delim);

        // FIXME: pass by-ref
        let val = self.fa.get_csv_line_ex().delim(delim).done();
        self.check_error()?;
        Ok(val)
    }

    /// Reads the next 4 bytes from file as [`f32`].
    ///
    /// Underlying Godot method:
    /// [`FileAccess::get_float`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-get-float).
    #[doc(alias = "get_float")]
    pub fn read_f32(&mut self) -> std::io::Result<f32> {
        let val = self.fa.get_float();
        self.check_error()?;
        Ok(val)
    }

    /// Reads the next 8 bytes from file as [`f64`].
    ///
    /// Underlying Godot method:
    /// [`FileAccess::get_double`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-get-double).
    #[doc(alias = "get_double")]
    pub fn read_f64(&mut self) -> std::io::Result<f64> {
        let val = self.fa.get_double();
        self.check_error()?;
        Ok(val)
    }

    /// Reads the next 4 or 8 bytes from file as `real`, depending on configuration.
    ///
    /// See [`real`][type@real] type for more information.
    ///
    /// Underlying Godot method:
    /// [`FileAccess::get_float`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-get-float) or
    /// [`FileAccess::get_double`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-get-double)
    /// (note that [`FileAccess::get_real`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-get-real)
    /// does not return an actual `real`).
    ///
    /// <div class="warning">
    /// <strong>Warning:</strong>
    /// Since this involves a configuration-dependent type, you may not be able to read the value back if Godot uses different precision setting
    /// (single or double) than the one used to write the value.
    /// </div>
    #[doc(alias = "get_real")]
    pub fn read_real(&mut self) -> std::io::Result<real> {
        #[cfg(feature = "double-precision")]
        let val = self.fa.get_double();

        #[cfg(not(feature = "double-precision"))]
        let val = self.fa.get_float();

        self.check_error()?;
        Ok(val)
    }

    /// Reads the next [`Variant`] value from file.
    ///
    /// If `allow_objects` is set to `true`, objects will be decoded.
    ///
    /// <div class="warning">
    /// <strong>Warning:</strong> Deserialized objects can contain code which gets executed. Do not use this option if the serialized object
    /// comes from untrusted sources, to avoid potential security threats such as remote code execution.
    /// </div>
    ///
    /// Underlying Godot method:
    /// [`FileAccess::get_var`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-get-var).
    #[doc(alias = "get_var")]
    pub fn read_variant(&mut self, allow_objects: bool) -> std::io::Result<Variant> {
        let val = self.fa.get_var_ex().allow_objects(allow_objects).done();
        self.check_error()?;
        Ok(val)
    }

    /// Writes [`u8`] as the next byte in the file.
    ///
    /// Underlying Godot method:
    /// [`FileAccess::store_8`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-store-8).
    #[doc(alias = "store_8")]
    pub fn write_u8(&mut self, value: u8) -> std::io::Result<()> {
        self.fa.store_8(value);
        self.clear_file_length();
        self.check_error()?;
        Ok(())
    }

    /// Writes [`u16`] as the next 2 bytes in the file.
    ///
    /// Underlying Godot method:
    /// [`FileAccess::store_16`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-store-16).
    #[doc(alias = "store_16")]
    pub fn write_u16(&mut self, value: u16) -> std::io::Result<()> {
        self.fa.store_16(value);
        self.clear_file_length();
        self.check_error()?;
        Ok(())
    }

    /// Writes [`u32`] as the next 4 bytes in the file.
    ///
    /// Underlying Godot method:
    /// [`FileAccess::store_32`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-store-32).
    #[doc(alias = "store_32")]
    pub fn write_u32(&mut self, value: u32) -> std::io::Result<()> {
        self.fa.store_32(value);
        self.clear_file_length();
        self.check_error()?;
        Ok(())
    }

    /// Writes [`u64`] as the next 8 bytes in the file.
    ///
    /// Underlying Godot method:
    /// [`FileAccess::store_64`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-store-64).
    #[doc(alias = "store_64")]
    pub fn write_u64(&mut self, value: u64) -> std::io::Result<()> {
        self.fa.store_64(value);
        self.clear_file_length();
        self.check_error()?;
        Ok(())
    }

    /// Writes [`f32`] as the 32 bits in the file.
    ///
    /// Underlying Godot method:
    /// [`FileAccess::store_float`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-store-float).
    #[doc(alias = "store_float")]
    pub fn write_f32(&mut self, value: f32) -> std::io::Result<()> {
        self.fa.store_float(value);
        self.clear_file_length();
        self.check_error()?;
        Ok(())
    }

    /// Writes [`f64`] as the 64 bits in the file.
    ///
    /// Underlying Godot method:
    /// [`FileAccess::store_double`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-store-double).
    #[doc(alias = "store_double")]
    pub fn write_f64(&mut self, value: f64) -> std::io::Result<()> {
        self.fa.store_double(value);
        self.clear_file_length();
        self.check_error()?;
        Ok(())
    }

    /// Writes a `real` (`f32` or `f64`) as the next 4 or 8 bytes in the file, depending on configuration.
    ///
    /// See [`real`][type@real] type for more information.
    ///
    /// Underlying Godot method:
    /// [`FileAccess::store_float`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-store-float) or
    /// [`FileAccess::store_double`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-store-double)
    /// (note that [`FileAccess::store_real`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-store-real)
    /// does not accept an actual `real`).
    ///
    /// <div class="warning">
    /// <strong>Warning:</strong>
    /// Since this involves a configuration-dependent type, you may not be able to read the value back if Godot uses different precision setting
    /// (single or double) than the one used to write the value.
    /// </div>
    #[doc(alias = "store_real")]
    pub fn write_real(&mut self, value: real) -> std::io::Result<()> {
        // FileAccess::store_real() does not accept an actual real_t; work around this.

        #[cfg(feature = "double-precision")]
        self.fa.store_double(value);

        #[cfg(not(feature = "double-precision"))]
        self.fa.store_float(value);

        self.clear_file_length();
        self.check_error()?;
        Ok(())
    }

    /// Writes string to the file.
    ///
    /// This function is meant to be used in text files. To store a string in a binary file, use `store_pascal_string()`
    ///
    /// Underlying Godot method:
    /// [`FileAccess::store_string`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-store-string).
    #[doc(alias = "store_string")]
    pub fn write_gstring(&mut self, value: impl AsArg<GString>) -> std::io::Result<()> {
        arg_into_ref!(value);

        self.fa.store_string(value);
        self.clear_file_length();
        self.check_error()?;
        Ok(())
    }

    /// Writes string to the file as Pascal String.
    ///
    /// This function is meant to be used in binary files. To store a string in a text file, use `store_string()`
    ///
    /// Pascal String is useful for writing and retrieving verying-length string data from binary files. It is saved
    /// with length-prefix, instead of null terminator as in C strings.
    ///
    /// See also:
    /// - [Wikipedia article](https://en.wikipedia.org/wiki/String_(computer_science)#Length-prefixed)
    /// - [Godot `FileAccess::store_pascal_string`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-store-pascal-string)
    #[doc(alias = "store_pascal_string")]
    pub fn write_pascal_string(&mut self, value: impl AsArg<GString>) -> std::io::Result<()> {
        arg_into_ref!(value);

        self.fa.store_pascal_string(value);
        self.clear_file_length();
        self.check_error()?;
        Ok(())
    }

    /// Write string to the file as a line.
    ///
    /// Underlying Godot method:
    /// [`FileAccess::store_line`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-store-line).
    #[doc(alias = "store_line")]
    pub fn write_gstring_line(&mut self, value: impl AsArg<GString>) -> std::io::Result<()> {
        arg_into_ref!(value);

        self.fa.store_line(value);
        self.clear_file_length();
        self.check_error()?;
        Ok(())
    }

    /// Write [`PackedStringArray`] to the file as delimited line.
    ///
    /// For writing traditional `CSV` format, provide comma (`','`) as `delim`.
    ///
    /// Underlying Godot method:
    /// [`FileAccess::store_csv_line`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-store-csv-line).
    #[doc(alias = "store_csv_line")]
    pub fn write_csv_line(
        &mut self,
        values: &PackedStringArray,
        delim: impl AsArg<GString>,
    ) -> std::io::Result<()> {
        arg_into_ref!(delim);

        self.fa.store_csv_line_ex(values).delim(delim).done();
        self.clear_file_length();
        self.check_error()?;
        Ok(())
    }

    /// Write [`Variant`] to the file.
    ///
    /// If `full_objects` is set to `true`, encoding objects is allowed (and can potentially include GDScript code). Not
    /// all properties of the Variant are included. Only properties that are exported (have `#[export]` derive attribute)
    /// will be serialized.
    ///
    /// Underlying Godot method:
    /// [`FileAccess::store_var`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html#class-fileaccess-method-store-var).
    #[doc(alias = "store_var")]
    pub fn write_variant(&mut self, value: Variant, full_objects: bool) -> std::io::Result<()> {
        self.fa
            .store_var_ex(&value)
            .full_objects(full_objects)
            .done();
        self.clear_file_length();
        self.check_error()?;
        Ok(())
    }

    /// Set true to use big-endian, false to use little-endian.
    ///
    /// Endianness can be set mid-file, not only at the start position. It makes it possible to write different sections
    /// of binary file with different endianness, though it is not recommended - can lead to confusion and mistakes during
    /// consequent read operations.
    pub fn set_big_endian(&mut self, value: bool) {
        self.fa.set_big_endian(value);
    }

    /// Check endianness of current file access.
    pub fn is_big_endian(&self) -> bool {
        self.fa.is_big_endian()
    }

    /// Get path of the opened file.
    #[doc(alias = "get_path")]
    pub fn path(&self) -> GString {
        self.fa.get_path()
    }

    /// Get absolute path of the opened file.
    #[doc(alias = "get_path_absolute")]
    pub fn path_absolute(&self) -> GString {
        self.fa.get_path_absolute()
    }

    /// Returns the current cursor position.
    #[doc(alias = "get_position")]
    pub fn position(&self) -> u64 {
        self.fa.get_position()
    }

    /// Get file length in bytes.
    #[doc(alias = "get_length")]
    pub fn length(&self) -> u64 {
        self.fa.get_length()
    }

    /// Checks if the file cursor has read past the end of the file.
    pub fn eof_reached(&self) -> bool {
        self.fa.eof_reached()
    }

    // ----------------------------------------------------------------------------------------------------------------------------------------------
    // Private methods.

    // Error handling utility function.
    fn check_error(&self) -> Result<(), std::io::Error> {
        let error = self.fa.get_error();
        if error == Error::OK {
            return Ok(());
        }

        Err(std::io::Error::other(format!("GodotError: {error:?}")))
    }

    // File length cache is stored and kept when possible because `FileAccess::get_length()` turned out to be slowing down
    // reading operations on big files with methods stemming from `std::io::Read` and `std::io::BufRead`.
    fn check_file_length(&mut self) -> u64 {
        if let Some(length) = self.file_length {
            return length;
        }
        let file_length = self.fa.get_length();
        self.file_length = Some(file_length);
        file_length
    }

    // The file length cache is cleared during writing operations, as this is the only place when the file length could be
    // changed - unless file is modified by some other `GFile`, but we cannot do anything about it then.
    fn clear_file_length(&mut self) {
        self.file_length = None;
    }

    // Private constructor function.
    fn from_inner(fa: Gd<FileAccess>) -> Self {
        let file_length = Some(fa.get_length());
        Self {
            fa,
            buffer: vec![0; Self::BUFFER_SIZE],
            last_buffer_size: 0,
            write_buffer: PackedByteArray::new(),
            file_length,
        }
    }

    // Writer utilities.
    fn extend_write_buffer(&mut self, len: usize) {
        if self.write_buffer.len() >= len {
            return;
        }
        self.write_buffer.resize(len)
    }

    fn pack_into_write_buffer(&mut self, buf: &[u8]) {
        self.extend_write_buffer(buf.len());
        let write_slice = self.write_buffer.as_mut_slice();
        write_slice[0..buf.len()].copy_from_slice(buf);
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Trait implementations.

impl Read for GFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let length = self.check_file_length();
        let position = self.fa.get_position();
        if position >= length {
            return Ok(0);
        }

        let remaining_bytes = (length - position) as usize;
        let bytes_to_read = cmp::min(buf.len(), remaining_bytes);
        if bytes_to_read == 0 {
            return Ok(0);
        }

        let gd_buffer = self.fa.get_buffer(bytes_to_read as i64);
        let bytes_read = gd_buffer.len();
        buf[0..bytes_read].copy_from_slice(gd_buffer.as_slice());

        self.check_error()?;

        Ok(bytes_read)
    }
}

impl Write for GFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.pack_into_write_buffer(buf);
        self.fa
            .store_buffer(&self.write_buffer.subarray(0..buf.len()));
        self.clear_file_length();
        self.check_error()?;

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.fa.flush();
        self.check_error()?;
        Ok(())
    }
}

impl Seek for GFile {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        match pos {
            SeekFrom::Start(position) => {
                self.fa.seek(position);
                self.check_error()?;
                Ok(position)
            }
            SeekFrom::End(offset) => {
                if (self.check_file_length() as i64) < offset {
                    return Err(std::io::Error::new(
                        ErrorKind::InvalidInput,
                        "Position can't be set before the file beginning",
                    ));
                }
                self.fa.seek_end_ex().position(offset).done();
                self.check_error()?;
                Ok(self.fa.get_position())
            }
            SeekFrom::Current(offset) => {
                let new_pos = self.fa.get_position() as i64 + offset;
                if new_pos < 0 {
                    return Err(std::io::Error::new(
                        ErrorKind::InvalidInput,
                        "Position can't be set before the file beginning",
                    ));
                }
                let new_pos = new_pos as u64;
                self.fa.seek(new_pos);
                self.check_error()?;
                Ok(new_pos)
            }
        }
    }
}

impl BufRead for GFile {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        // We need to determine number of remaining bytes - otherwise the `FileAccess::get_buffer return in an error`.
        let remaining_bytes = self.check_file_length() - self.fa.get_position();
        let buffer_read_size = cmp::min(remaining_bytes as usize, Self::BUFFER_SIZE);

        // We need to keep the amount of last read side to be able to adjust cursor position in `consume`.
        self.last_buffer_size = buffer_read_size;
        self.buffer = vec![0; Self::BUFFER_SIZE];

        let gd_buffer = self.fa.get_buffer(buffer_read_size as i64);
        self.check_error()?;

        let read_buffer = &mut self.buffer[0..gd_buffer.len()];

        read_buffer.copy_from_slice(gd_buffer.as_slice());

        Ok(read_buffer)
    }

    fn consume(&mut self, amt: usize) {
        // Cursor is being moved by `FileAccess::get_buffer()` call, so we need to adjust it.
        let offset = (self.last_buffer_size - amt) as i64;
        let pos = SeekFrom::Current(-offset);

        self.seek(pos).expect("failed to consume bytes during read");
    }
}
