use serde::Deserialize;

macro_rules! define_games {
    (
        $(
            // We make the last two arguments optional by wrapping them in $( , $arg1, $arg2 )?
            $variant:ident ( 
                $name:expr, 
                $appid:expr 
                $(, $seeds:expr, $steam_calc:expr)? 
            )
        ),* $(,)?
    ) => {
        #[repr(i32)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Hash)]
        pub enum Game {
            $( $variant, )*
        }

        pub const GAME_OPTIONS: &[(&'static str, Game)] = &[
            $( ($name, Game::$variant), )*
        ];

        impl Game {
            pub fn from_string(id: &str) -> Option<Game> {
                $( if id == stringify!($variant) { return Some(Game::$variant); } )*
                None
            }

            pub fn get_mandarin_seeds(&self) -> Option<(u64, u64)> {
                match self {
                    $( 
                        #[allow(unused_assignments, unused_mut)]
                        Game::$variant => {
                            let mut seeds = None;
                            $( seeds = $seeds; )?
                            seeds
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
        }
    };
}

define_games! {
    MHWILDS  ( "MH Wilds",         2246340, Some((0xBFACF76C3F96, 0x7A36955255266CED)),     |id| id             ),
    DD2      ( "Dragon's Dogma 2", 2054970, Some((0x90EDB79172FDBE51, 0x5EC646997D69AE1B)), |id| id & 0xffffffff),
    PRAGMATA ( "Pragmata",         3357650, Some((0x3F90D767F13ABE2E, 0x7DA24A9E1479F3D7)), |id| id             ),
    MHST3    ( "MH Stories 3",     2852190, Some((0x4DB2A5EC6AD4005A, 0xA40139F12BA19EDB)), |id| id             ),
    RE9      ( "RE Requiem",       3764200, Some((0, 0x61f6868699c14dfa)),                  |id| id             ),
    MHRISE   ( "MH Rise",          1446780),
    SF6      ( "SF6",              1364780),
    RE2      ( "RE 2",              883710),
}
