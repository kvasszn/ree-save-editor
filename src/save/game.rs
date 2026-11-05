use serde::{Deserialize, Serialize};
use strum::EnumString;
use strum_macros::EnumIter;

use crate::save::crypto;

macro_rules! define_games {
    (
        $(
            $variant:ident ( 
                $name:expr, 
                $appid:expr 
                $(, seeds: $seeds:expr)? 
                $(, calc: $steam_calc:expr)?
                $(, blowfish: $blowfish:expr )? 
                $(, lime: $lime:expr )? 
            )
        ),* $(,)?
    ) => {
        #[repr(i32)]
        #[derive(EnumIter, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash, EnumString)]
        pub enum Game {
            $( $variant, )*
        }

        pub const GAME_OPTIONS: &[(&'static str, Game)] = &[
            $( ($name, Game::$variant), )*
        ];


        impl ToString for Game {
            fn to_string(&self) -> String {
                $( if *self == Game::$variant { return $name.to_string(); } )*
                String::default()
            }
        }

        impl Game {
            pub fn from_string(id: &str) -> Option<Game> {
                $( if id == stringify!($variant) { return Some(Game::$variant); } )*
                None
            }

            pub fn to_str(&self) -> &str {
                $( if *self == Game::$variant { return $name; } )*
                ""
            }

            pub fn get_mandarin_seeds(&self) -> Option<(u64, u64)> {
                match self {
                    $( 
                        #[allow(unused_assignments, unused_mut)]
                        Game::$variant => {
                            let mut seeds = None;
                            $( seeds = Some($seeds); )?
                            seeds
                        }
                    )*
                }
            }

            pub fn get_blowfish_key(&self) -> Option<&'static [u8]> {
                match self {
                    $( 
                        #[allow(unused_assignments, unused_mut)]
                        Game::$variant => {
                            let mut key = None;
                            $( key = Some($blowfish); )?
                            key
                        }
                    )*
                }
            }

            pub fn get_key_from_steamid(&self, steamid: u64) -> u64 {
                match self {
                    $( 
                        #[allow(unused_assignments, unused_mut)]
                        Game::$variant => {
                            let mut key = 0;
                            $( key = ($steam_calc)(steamid); )?
                            key
                        }
                    )*
                }
            }

            pub fn get_appid(&self) -> u64 {
                match self {
                    $( Game::$variant => $appid, )*
                }
            }

            pub fn get_is_lime(&self) -> bool {
                match self {
                    $( 
                        #[allow(unused_assignments, unused_mut)]
                        Game::$variant => {
                            let mut is_lime = false;
                            $( is_lime = $lime; )?
                            is_lime
                        }
                    )*
                }
            }
        }
    };
}

define_games! {
    MHWILDS ("MH Wilds",         2246340, seeds: (0xBFACF76C3F96, 0x7A36955255266CED),     calc: |id| id),
    DD2     ("Dragon's Dogma 2", 2054970, seeds: (0x90EDB79172FDBE51, 0x5EC646997D69AE1B), calc: |id| id & 0xffffffff),
    PRAGMATA("Pragmata",         3357650, seeds: (0x3F90D767F13ABE2E, 0x7DA24A9E1479F3D7), calc: |id| id),
    MHST3   ("MH Stories 3",     2852190, seeds: (0x4DB2A5EC6AD4005A, 0xA40139F12BA19EDB), calc: |id| id),
    RE9     ("RE Requiem",       3764200, seeds: (0, 0x61f6868699c14dfa),                  calc: |id| id),
    MHRISE  ("MH Rise",          1446780),
    SF6     ("SF6",              1364780),
    RE2     ("RE 2", 883710,  blowfish: crypto::blowfish::KEY_RE2),
    RE3     ("RE 3", 952060,  blowfish: crypto::blowfish::KEY_RE3),
    RE7     ("RE 7", 418370, blowfish: crypto::blowfish::KEY_RE7),
    RE8     ("RE 8", 1196590, blowfish: crypto::blowfish::KEY_RE8),
    RE4     ("RE 4", 2050650, calc: |id: u64| id & 0xffffffff, lime: true),
    DD2PS5  ("DD2 PS5", 0, calc: |id: u64| id, lime: true),
    MISC    ("Misc", 0),
}

impl Default for Game {
    fn default() -> Self {
        #[cfg(feature = "mhwilds")]  { return Self::MHWILDS; }
        #[cfg(feature = "dd2")]      { return Self::DD2; }
        #[cfg(feature = "pragmata")] { return Self::PRAGMATA; }
        #[cfg(feature = "mhst3")]    { return Self::MHST3; }
        #[cfg(feature = "re9")]      { return Self::RE9; }
        #[cfg(feature = "mhrise")]   { return Self::MHRISE; }
        #[cfg(feature = "sf6")]      { return Self::SF6; }
        #[cfg(feature = "re2")]      { return Self::RE2; }
        #[cfg(feature = "re3")]      { return Self::RE3; }
        #[cfg(feature = "re4")]      { return Self::RE4; }
        #[cfg(feature = "re7")]      { return Self::RE7; }
        #[cfg(feature = "re8")]      { return Self::RE8; }
        Self::MHWILDS
    }
}
