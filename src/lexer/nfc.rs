//! Accented text is composed (NFC) on the way in. A decomposed accent,
//! the form some editors paste, becomes the single letter. The student
//! is not asked to re-save the file.

use std::borrow::Cow;

use unicode_normalization::{UnicodeNormalization, is_nfc};

/// Compose `texto` to NFC. Already-composed text is returned as-is.
pub fn em_nfc(texto: &str) -> Cow<'_, str> {
    if is_nfc(texto) {
        Cow::Borrowed(texto)
    } else {
        Cow::Owned(texto.nfc().collect())
    }
}
