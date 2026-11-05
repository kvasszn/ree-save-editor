mod seek_ext;
mod read_ext;
use std::{io::Cursor, ops::{Add, Rem, Sub}};

pub use read_ext::*;
pub use seek_ext::*;


pub fn align_up<T: Copy + Add<Output = T> + Sub<Output = T> + Rem<Output = T>>(
    value: T, align: T,
) -> T {
    value + (align - value % align) % align
}

pub fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
    }
}

pub fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    first.to_uppercase().collect::<String>() 
                    + &chars.as_str().to_lowercase()
                }
            }
        })
        .collect()
}

