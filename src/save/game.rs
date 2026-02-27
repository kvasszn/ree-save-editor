use serde::Deserialize;

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Hash)]
pub enum Game {
    MHWILDS = 0,
    DD2 = 1,
    PRAGMATA = 2,
    MHST3 = 3,
    RE9 = 4,
}

pub const GAME_OPTIONS: [(&'static str, Game); 5] = [
    ("MH Wilds", Game::MHWILDS),
    ("Dragon's Dogma 2", Game::DD2),
    ("Pragmata", Game::PRAGMATA),
    ("MH Stories 3", Game::MHST3),
    ("RE Requiem", Game::RE9)
];

impl Game {
    pub fn from_string(id: &str) -> Option<Game> {
        match id {
            "MHWILDS" => Some(Game::MHWILDS),
            "DD2" => Some(Game::DD2),
            "PRAGMATA" => Some(Game::PRAGMATA),
            "MHST3" => Some(Game::MHST3),
            "RE9" => Some(Game::RE9),
            _ => None,
        }
    }

    // return (rsa_seed, enc_seed)
    pub fn get_mandarin_seeds(&self) -> Option<(u64, u64)> {
        match self {
            Self::MHWILDS => Some((0xBFACF76C3F96, 0x7A36955255266CED)),
            Self::DD2 => Some((0x90EDB79172FDBE51, 0x5EC646997D69AE1B)),
            Self::PRAGMATA => Some((0x3F90D767F13ABE2E, 0x7DA24A9E1479F3D7)),
            Self::MHST3 => Some((0x4DB2A5EC6AD4005A, 0xA40139F12BA19EDB)),
            Self::RE9 => Some((0, 0x61f6868699c14dfa))
        }
    }

    pub fn get_key_from_steamid(&self, steamid: u64) -> u64 {
        match self {
            Self::MHWILDS => steamid,
            Self::DD2 => steamid & 0xffffffff,
            Self::PRAGMATA => 19284827,
            Self::MHST3 => steamid,
            Self::RE9 => steamid,
        }
    }

    pub fn get_appid(&self) -> u64 {
        match self {
            Self::MHWILDS => 2246340,
            Self::DD2 => 2054970,
            Self::PRAGMATA => 3357650,
            Self::MHST3 => 2852190,
            Self::RE9 => 3764200,
        }
    }
}
