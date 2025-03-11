#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]

//! A crate providing utilities for slice iteration.

mod tests;
mod cursor;
mod position;

pub use cursor::*;
pub use position::*;

/// Calculates line:col from a source string slice and an offset.
/// 
/// May be useful for error messages.
pub fn get_lines_and_columns(source: &str, byte_offset: usize) -> (usize, usize) {
    let mut lines = 0;
    let mut columns = 0;
    let mut i = 0;

    let mut cursor = Cursor::new(source.as_bytes());

    loop {
        if byte_offset <= i { break }

        match cursor.peek() {
            Some(b'\r') => {
                lines += 1;
                columns = 0;

                unsafe { cursor.advance_unchecked() }

                if let Some(b'\n') = cursor.peek() {
                    unsafe { cursor.advance_unchecked() }
                    i += 1;
                }
            }
            Some(b'\n') => {
                lines += 1;
                columns = 0;

                unsafe { cursor.advance_unchecked() }
            }
            Some(_) => {
                columns += 1;
                unsafe { cursor.advance_char_unchecked() }
            }
            None => break
        }

        i += 1;
    }

    (lines, columns)
}