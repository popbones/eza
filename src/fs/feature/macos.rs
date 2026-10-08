// SPDX-FileCopyrightText: 2026 Popbones
// SPDX-License-Identifier: EUPL-1.2
//! macOS visibility metadata, independent of filename and symlink targets.

use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;

use crate::fs::File;

/// Whether an entry has either of the visibility flags understood by Finder.
/// Missing, unreadable, or malformed metadata must not hide an otherwise visible entry.
pub fn is_hidden(file: &File<'_>) -> bool {
    if file.flags().0 & libc::UF_HIDDEN != 0 {
        return true;
    }

    let Ok(path) = CString::new(file.path.as_os_str().as_bytes()) else {
        return false;
    };
    let mut finder_info = [0_u8; 32];

    // SAFETY: Both strings are NUL terminated, and the writable buffer has the
    // supplied length. XATTR_NOFOLLOW checks the entry, not its symlink target.
    let size = unsafe {
        libc::getxattr(
            path.as_ptr(),
            c"com.apple.FinderInfo".as_ptr(),
            finder_info.as_mut_ptr().cast(),
            finder_info.len(),
            0,
            libc::XATTR_NOFOLLOW,
        )
    };

    // FileInfo and FolderInfo both store big-endian finderFlags at offset 8.
    // kIsInvisible is 0x4000; FinderInfo consists of 32 bytes in both cases.
    size == 32 && u16::from_be_bytes([finder_info[8], finder_info[9]]) & 0x4000 != 0
}
