#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]

//! A crate providing utilities for slice iteration.

mod cursor;
mod position;
mod tests;

pub use cursor::*;
pub use position::*;

/// Calculates line:col from a source string slice and an offset. CRLF sequences are treated as
/// one line break.
///
/// May be useful for error messages.
///
/// **Lines and columns are 0-indexed**, meaning the first line (or the first column) is 0.
pub fn get_lines_and_columns(source: &str, byte_offset: usize) -> (usize, usize) {
    compute_location(source.as_bytes(), byte_offset)
}

/// Computes the line number in inline offset of a byte `offset`. The computed
/// offset is a byte offset from the last linebreak if `bytes` is an ASCII
/// string without "\r\n"s. Otherwise, this function considers the below rules.
///
/// 1. If a byte is bigger than `0x7F` and it constitutes a valid UTF-8 character
///    encoding, it is contracted and only increases the inline offset by one.
///    If it _not_ part of a valid UTF-8 character encoding, each byte is taken
///    seperately, e.g. `110X_XXXX` followed by a byte with a zero MSB.
/// 2. The bytestring `b"\r\n"` is a single linebreak.
/// 3. Grapheme clusters may or may not be counted as multiple characters. This
///    includes Emojis and sequences thereof joined using U+200D ZERO WIDTH
///    JOINER. For example `Â` (U+00C2 LATIN CAPITAL LETTER A WITH CIRCUMFLEX)
///    and the ASCII `A` with a `◌̂` (U+0302 COMBINING CIRCUMFLEX ACCENT) may
///    or may not both report the same character count.
/// 4. If `offset` is in the middle of a UTF-8 encoding or a multicharacter
///    sequence with special handling, only the bytes before `offset` are
///    considered.
///
/// Note that this function is most likely not suited if `bytes` is not mostly
/// encoded as UTF-8. However, this is at worst the byte offset from the last
/// linebreak.
fn compute_location(bytes: &[u8], offset: usize) -> (usize, usize) {
    assert!(bytes.len() >= offset);

    let mut index = offset;
    let mut inline_offset = 0;
    loop {
        index -= 1;
        match bytes[index] {
            b'\n' | b'\r' => break,
            0..=0x7F | 0xC0..=u8::MAX => (),
            // The last byte of a UTF-8 encoded codepoint. This might be a
            // multibyte sequence.
            0x80..=0xBF => compute_utf8_bytes_len(bytes, &mut index),
        };
        inline_offset += 1;
        if index == 0 {
            return (0, inline_offset);
        }
    }

    let line = count_linebreaks(&bytes[0..=index]);
    (line, inline_offset)
}

/// Extract the `N` most significant bits from the byte at `bytes[N-1]`.
fn mask_utf8_byte<const N: u8>(bytes: &[u8], index: usize) -> Option<u8> {
    assert!((2..4).contains(&N));

    let mask = ((2 << N) - 1) << (u8::BITS - N as u32);
    index
        .checked_sub((N - 1).into())
        .and_then(|i| bytes.get(i))
        .map(|b| *b & mask)
}

/// Computes the size of a UTF-8 encoded codepoint and subtracts it from index.
/// Assumes a byte of the form `0b10XXXXXX` at `bytes[*index]`. Works backwards
/// from there, i.e. the `bytes[*index]` is the _last_ byte of the encoded
/// codepoint.
fn compute_utf8_bytes_len(bytes: &[u8], index: &mut usize) {
    match mask_utf8_byte::<2>(bytes, *index) {
        // This is a two-byte UTF-8 encoded codepoint.
        Some(0b110_00000) => {
            *index -= 1;
            return;
        }
        // This might be a three- or four-byte UTF-8 encoded codepoint or none.
        Some(0x80 | 0xA0) => (),
        // This does not look like a UTF-8 encoded codepoint.
        _ => return,
    }

    match mask_utf8_byte::<3>(bytes, *index) {
        // This is a three-byte UTF-8 encoded codepoint.
        Some(0b1110_000) => {
            *index -= 2;
            return;
        }
        // This might be a four-byte UTF-8 encoded codepoint or none.
        Some(0x80 | 0x90 | 0xA0 | 0xB0) => (),
        // This does not look like a UTF-8 encoded codepoint.
        _ => return,
    }

    match mask_utf8_byte::<4>(bytes, *index) {
        // This is a four-byte UTF-8 encoded codepoint.
        Some(0b11110_000) => {
            *index -= 3;
            return;
        }
        // This does not look like a UTF-8 encoded codepoint.
        _ => return,
    }
}

/// Counts `\n`, `\r` and `\r\n`.
fn count_linebreaks(bytes: &[u8]) -> usize {
    let mut count = 0;
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\r' => {
                count += 1;
                index += match bytes.get(index + 1) {
                    Some(b'\n') => 2,
                    _ => 1,
                }
            }
            b'\n' => count += 1,
            _ => (),
        }
    }
    count
}
