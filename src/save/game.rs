#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Game {
    MHWILDS = 0,
    DD2 = 1,
    PRAGMATA = 2,
}

pub const GAME_OPTIONS: [(&'static str, Game); 3] = [
    ("MH Wilds", Game::MHWILDS),
    ("Dragon's Dogma 2", Game::DD2),
    ("Pragamata", Game::PRAGMATA),
];

impl Game {
    // return (rsa_seed, enc_seed)
    pub fn get_mandarin_seeds(&self) -> Option<(u64, u64)> {
        match self {
            Self::MHWILDS => Some((0xBFACF76C3F96, 0x7A36955255266CED)),
            Self::DD2 => Some((0x90EDB79172FDBE51, 0x5EC646997D69AE1B)),
            Self::PRAGMATA => Some((0x3F90D767F13ABE2E, 0x7DA24A9E1479F3D7)),
        }
    }

    pub fn get_key_from_steamid(&self, steamid: u64) -> u64 {
        match self {
            Self::MHWILDS => steamid,
            Self::DD2 => steamid & 0xffffffff,
            Self::PRAGMATA => 19284827,
        }
    }
}
