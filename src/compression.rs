use crate::bitfield::BitField;

#[allow(dead_code)]
pub enum CompressionType {
    R8G8B8A8Unorm,
    R8G8Unorm,
    R8Unorm,
    Bc1Unorm,
    Bc2Unorm,
    Bc3Unorm,
    Bc4Unorm,
    Bc5Unorm,
    Bc6Unorm,
    Bc7Unorm,
    Unknown
}

impl CompressionType {

    pub fn get_bpps(format: u32) -> u32 {
        let t = Self::get_type(format);
        Self::bpps(t)
    }

    pub fn bpps(val: CompressionType) -> u32 {
        match val {
            Self::R8G8B8A8Unorm => 8,
            Self::R8G8Unorm => 16,
            Self::R8Unorm => 8,
            Self::Bc1Unorm | Self::Bc4Unorm => 4,
            Self::Bc2Unorm | Self::Bc3Unorm | Self::Bc5Unorm | Self::Bc6Unorm | Self::Bc7Unorm => 8,
            Self::Unknown => 0,
        }
    }
    pub fn get_type(format: u32) -> CompressionType {
        match format {
            0x1C | 0x1D => Self::R8G8B8A8Unorm,
            0x31 => Self::R8G8Unorm,
            0x3D => Self::R8Unorm,
            0x47 | 0x48 => Self::Bc1Unorm,
            0x4D | 0x4E => Self::Bc3Unorm,
            0x50 => Self::Bc4Unorm,
            0x53 => Self::Bc5Unorm,
            0x62 | 0x63 => Self::Bc7Unorm,
            _ => Self::Unknown
        }
    }
}

const WEIGHTS2: [u32; 4] = [0, 21, 43, 64];
const WEIGHTS3: [u32; 8] = [0, 9, 18, 27, 37, 46, 55, 64];
const WEIGHTS4: [u32; 16] = [0, 4, 9, 13, 17, 21, 26, 30, 34, 38, 43, 47, 51, 55, 60, 64];

#[rustfmt::skip]
#[allow(dead_code)]
const PARTITION2: [usize; 64 * 16] = [
    0,0,1,1,0,0,1,1,0,0,1,1,0,0,1,1,        0,0,0,1,0,0,0,1,0,0,0,1,0,0,0,1,        0,1,1,1,0,1,1,1,0,1,1,1,0,1,1,1,        0,0,0,1,0,0,1,1,0,0,1,1,0,1,1,1,        0,0,0,0,0,0,0,1,0,0,0,1,0,0,1,1,        0,0,1,1,0,1,1,1,0,1,1,1,1,1,1,1,        0,0,0,1,0,0,1,1,0,1,1,1,1,1,1,1,        0,0,0,0,0,0,0,1,0,0,1,1,0,1,1,1,
    0,0,0,0,0,0,0,0,0,0,0,1,0,0,1,1,        0,0,1,1,0,1,1,1,1,1,1,1,1,1,1,1,        0,0,0,0,0,0,0,1,0,1,1,1,1,1,1,1,        0,0,0,0,0,0,0,0,0,0,0,1,0,1,1,1,        0,0,0,1,0,1,1,1,1,1,1,1,1,1,1,1,        0,0,0,0,0,0,0,0,1,1,1,1,1,1,1,1,        0,0,0,0,1,1,1,1,1,1,1,1,1,1,1,1,        0,0,0,0,0,0,0,0,0,0,0,0,1,1,1,1,
    0,0,0,0,1,0,0,0,1,1,1,0,1,1,1,1,        0,1,1,1,0,0,0,1,0,0,0,0,0,0,0,0,        0,0,0,0,0,0,0,0,1,0,0,0,1,1,1,0,        0,1,1,1,0,0,1,1,0,0,0,1,0,0,0,0,        0,0,1,1,0,0,0,1,0,0,0,0,0,0,0,0,        0,0,0,0,1,0,0,0,1,1,0,0,1,1,1,0,        0,0,0,0,0,0,0,0,1,0,0,0,1,1,0,0,        0,1,1,1,0,0,1,1,0,0,1,1,0,0,0,1,
    0,0,1,1,0,0,0,1,0,0,0,1,0,0,0,0,        0,0,0,0,1,0,0,0,1,0,0,0,1,1,0,0,        0,1,1,0,0,1,1,0,0,1,1,0,0,1,1,0,        0,0,1,1,0,1,1,0,0,1,1,0,1,1,0,0,        0,0,0,1,0,1,1,1,1,1,1,0,1,0,0,0,        0,0,0,0,1,1,1,1,1,1,1,1,0,0,0,0,        0,1,1,1,0,0,0,1,1,0,0,0,1,1,1,0,        0,0,1,1,1,0,0,1,1,0,0,1,1,1,0,0,
    0,1,0,1,0,1,0,1,0,1,0,1,0,1,0,1,        0,0,0,0,1,1,1,1,0,0,0,0,1,1,1,1,        0,1,0,1,1,0,1,0,0,1,0,1,1,0,1,0,        0,0,1,1,0,0,1,1,1,1,0,0,1,1,0,0,        0,0,1,1,1,1,0,0,0,0,1,1,1,1,0,0,        0,1,0,1,0,1,0,1,1,0,1,0,1,0,1,0,        0,1,1,0,1,0,0,1,0,1,1,0,1,0,0,1,        0,1,0,1,1,0,1,0,1,0,1,0,0,1,0,1,
    0,1,1,1,0,0,1,1,1,1,0,0,1,1,1,0,        0,0,0,1,0,0,1,1,1,1,0,0,1,0,0,0,        0,0,1,1,0,0,1,0,0,1,0,0,1,1,0,0,        0,0,1,1,1,0,1,1,1,1,0,1,1,1,0,0,        0,1,1,0,1,0,0,1,1,0,0,1,0,1,1,0,        0,0,1,1,1,1,0,0,1,1,0,0,0,0,1,1,        0,1,1,0,0,1,1,0,1,0,0,1,1,0,0,1,        0,0,0,0,0,1,1,0,0,1,1,0,0,0,0,0,
    0,1,0,0,1,1,1,0,0,1,0,0,0,0,0,0,        0,0,1,0,0,1,1,1,0,0,1,0,0,0,0,0,        0,0,0,0,0,0,1,0,0,1,1,1,0,0,1,0,        0,0,0,0,0,1,0,0,1,1,1,0,0,1,0,0,        0,1,1,0,1,1,0,0,1,0,0,1,0,0,1,1,        0,0,1,1,0,1,1,0,1,1,0,0,1,0,0,1,        0,1,1,0,0,0,1,1,1,0,0,1,1,1,0,0,        0,0,1,1,1,0,0,1,1,1,0,0,0,1,1,0,
    0,1,1,0,1,1,0,0,1,1,0,0,1,0,0,1,        0,1,1,0,0,0,1,1,0,0,1,1,1,0,0,1,        0,1,1,1,1,1,1,0,1,0,0,0,0,0,0,1,        0,0,0,1,1,0,0,0,1,1,1,0,0,1,1,1,        0,0,0,0,1,1,1,1,0,0,1,1,0,0,1,1,        0,0,1,1,0,0,1,1,1,1,1,1,0,0,0,0,        0,0,1,0,0,0,1,0,1,1,1,0,1,1,1,0,        0,1,0,0,0,1,0,0,0,1,1,1,0,1,1,1
];

#[allow(dead_code)]
#[rustfmt::skip]
const PARTITION3: [usize; 64 * 16] = [
    0,0,1,1,0,0,1,1,0,2,2,1,2,2,2,2,        0,0,0,1,0,0,1,1,2,2,1,1,2,2,2,1,        0,0,0,0,2,0,0,1,2,2,1,1,2,2,1,1,        0,2,2,2,0,0,2,2,0,0,1,1,0,1,1,1,        0,0,0,0,0,0,0,0,1,1,2,2,1,1,2,2,        0,0,1,1,0,0,1,1,0,0,2,2,0,0,2,2,        0,0,2,2,0,0,2,2,1,1,1,1,1,1,1,1,        0,0,1,1,0,0,1,1,2,2,1,1,2,2,1,1,
    0,0,0,0,0,0,0,0,1,1,1,1,2,2,2,2,        0,0,0,0,1,1,1,1,1,1,1,1,2,2,2,2,        0,0,0,0,1,1,1,1,2,2,2,2,2,2,2,2,        0,0,1,2,0,0,1,2,0,0,1,2,0,0,1,2,        0,1,1,2,0,1,1,2,0,1,1,2,0,1,1,2,        0,1,2,2,0,1,2,2,0,1,2,2,0,1,2,2,        0,0,1,1,0,1,1,2,1,1,2,2,1,2,2,2,        0,0,1,1,2,0,0,1,2,2,0,0,2,2,2,0,
    0,0,0,1,0,0,1,1,0,1,1,2,1,1,2,2,        0,1,1,1,0,0,1,1,2,0,0,1,2,2,0,0,        0,0,0,0,1,1,2,2,1,1,2,2,1,1,2,2,        0,0,2,2,0,0,2,2,0,0,2,2,1,1,1,1,        0,1,1,1,0,1,1,1,0,2,2,2,0,2,2,2,        0,0,0,1,0,0,0,1,2,2,2,1,2,2,2,1,        0,0,0,0,0,0,1,1,0,1,2,2,0,1,2,2,        0,0,0,0,1,1,0,0,2,2,1,0,2,2,1,0,
    0,1,2,2,0,1,2,2,0,0,1,1,0,0,0,0,        0,0,1,2,0,0,1,2,1,1,2,2,2,2,2,2,        0,1,1,0,1,2,2,1,1,2,2,1,0,1,1,0,        0,0,0,0,0,1,1,0,1,2,2,1,1,2,2,1,        0,0,2,2,1,1,0,2,1,1,0,2,0,0,2,2,        0,1,1,0,0,1,1,0,2,0,0,2,2,2,2,2,        0,0,1,1,0,1,2,2,0,1,2,2,0,0,1,1,        0,0,0,0,2,0,0,0,2,2,1,1,2,2,2,1,
    0,0,0,0,0,0,0,2,1,1,2,2,1,2,2,2,        0,2,2,2,0,0,2,2,0,0,1,2,0,0,1,1,        0,0,1,1,0,0,1,2,0,0,2,2,0,2,2,2,        0,1,2,0,0,1,2,0,0,1,2,0,0,1,2,0,        0,0,0,0,1,1,1,1,2,2,2,2,0,0,0,0,        0,1,2,0,1,2,0,1,2,0,1,2,0,1,2,0,        0,1,2,0,2,0,1,2,1,2,0,1,0,1,2,0,        0,0,1,1,2,2,0,0,1,1,2,2,0,0,1,1,
    0,0,1,1,1,1,2,2,2,2,0,0,0,0,1,1,        0,1,0,1,0,1,0,1,2,2,2,2,2,2,2,2,        0,0,0,0,0,0,0,0,2,1,2,1,2,1,2,1,        0,0,2,2,1,1,2,2,0,0,2,2,1,1,2,2,        0,0,2,2,0,0,1,1,0,0,2,2,0,0,1,1,        0,2,2,0,1,2,2,1,0,2,2,0,1,2,2,1,        0,1,0,1,2,2,2,2,2,2,2,2,0,1,0,1,        0,0,0,0,2,1,2,1,2,1,2,1,2,1,2,1,
    0,1,0,1,0,1,0,1,0,1,0,1,2,2,2,2,        0,2,2,2,0,1,1,1,0,2,2,2,0,1,1,1,        0,0,0,2,1,1,1,2,0,0,0,2,1,1,1,2,        0,0,0,0,2,1,1,2,2,1,1,2,2,1,1,2,        0,2,2,2,0,1,1,1,0,1,1,1,0,2,2,2,        0,0,0,2,1,1,1,2,1,1,1,2,0,0,0,2,        0,1,1,0,0,1,1,0,0,1,1,0,2,2,2,2,        0,0,0,0,0,0,0,0,2,1,1,2,2,1,1,2,
    0,1,1,0,0,1,1,0,2,2,2,2,2,2,2,2,        0,0,2,2,0,0,1,1,0,0,1,1,0,0,2,2,        0,0,2,2,1,1,2,2,1,1,2,2,0,0,2,2,        0,0,0,0,0,0,0,0,0,0,0,0,2,1,1,2,        0,0,0,2,0,0,0,1,0,0,0,2,0,0,0,1,        0,2,2,2,1,2,2,2,0,2,2,2,1,2,2,2,        0,1,0,1,2,2,2,2,2,2,2,2,2,2,2,2,        0,1,1,1,2,0,1,1,2,2,0,1,2,2,2,0,
];

#[rustfmt::skip]
#[allow(dead_code)]
const ANCHOR_SECOND: [usize; 64] = [
    15,15,15,15,15,15,15,15,        15,15,15,15,15,15,15,15,        15, 2, 8, 2, 2, 8, 8,15,        2, 8, 2, 2, 8, 8, 2, 2,        15,15, 6, 8, 2, 8,15,15,        2, 8, 2, 2, 2,15,15, 6,        6, 2, 6, 8,15,15, 2, 2,        15,15,15,15,15, 2, 2,15
];

#[rustfmt::skip]
#[allow(dead_code)]
const ANCHOR_THIRD1: [usize; 64] = [
    3, 3,15,15, 8, 3,15,15,        8, 8, 6, 6, 6, 5, 3, 3,        3, 3, 8,15, 3, 3, 6,10,        5, 8, 8, 6, 8, 5,15,15,        8,15, 3, 5, 6,10, 8,15,        15, 3,15, 5,15,15,15,15,        3,15, 5, 5, 5, 8, 5,10,        5,10, 8,13,15,12, 3, 3
];

#[rustfmt::skip]
#[allow(dead_code)]
const ANCHOR_THIRD2: [usize; 64] = [
    15, 8, 8, 3,15,15, 3, 8,        15,15,15,15,15,15,15, 8,        15, 8,15, 3,15, 8,15, 8,        3,15, 6,10,15,15,10, 8,        15, 3,15,10,10, 8, 9,10,        6,15, 8,15, 3, 6, 6, 8,        15, 3,15,15,15,15,15,15,        15,15,15,15, 3,15,15, 8
];

struct InputBitStream {
    data: u128,
    bits_read: u32,
}

impl InputBitStream {
    fn new(data: u128) -> InputBitStream {
        InputBitStream { data, bits_read: 0 }
    }

    fn get_bits_read(&self) -> u32 {
        self.bits_read
    }

    fn read_bits32(&mut self, n_bits: u32) -> u32 {
        debug_assert!(n_bits <= 32);
        self.bits_read += n_bits;
        debug_assert!(self.bits_read <= 128);
        let ret = self.data & ((1 << n_bits) - 1);
        self.data >>= n_bits;
        ret as u32
    }
}

fn bc7_dequant_pbit(val: u32, pbit: u32, val_bits: u32) -> u32 {
    debug_assert!(val < (1 << val_bits));
    debug_assert!(pbit < 2);
    debug_assert!((4..=8).contains(&val_bits));
    let total_bits = val_bits + 1;
    let mut val = (val << 1) | pbit;
    val <<= 8 - total_bits;
    val |= val >> total_bits;
    debug_assert!(val <= 255);
    val
}
fn bc7_dequant(mut val: u32, val_bits: u32) -> u32 {
    debug_assert!(val < (1 << val_bits));
    debug_assert!((4..=8).contains(&val_bits));
    val <<= 8 - val_bits;
    val |= val >> val_bits;
    debug_assert!(val <= 255);
    val
}

fn bc7_interp2(l: u32, h: u32, w: usize) -> u8 {
    ((l * (64 - WEIGHTS2[w]) + h * WEIGHTS2[w] + 32) >> 6) as u8
}
fn bc7_interp3(l: u32, h: u32, w: usize) -> u8 {
    ((l * (64 - WEIGHTS3[w]) + h * WEIGHTS3[w] + 32) >> 6) as u8
}
fn bc7_interp23(l: u32, h: u32, w: usize, bits: u32) -> u8 {
    debug_assert!(l <= 255 && h <= 255);
    match bits {
        2 => bc7_interp2(l, h, w),
        3 => bc7_interp3(l, h, w),
        _ => unreachable!(),
    }
}

fn unpack_bc7_mode0_2<F: FnMut(usize, usize, [u8; 4])>(mode: u32, block: u128, mut writer: F) {
    let weight_bits = if mode == 0 { 3 } else { 2 };
    let endpoint_bits = if mode == 0 { 4 } else { 5 };
    let pb = if mode == 0 { 6 } else { 0 };
    let weight_vals = 1 << weight_bits;

    let mut stream = InputBitStream::new(block);

    assert_eq!(stream.read_bits32(mode + 1), 1 << mode);

    let part = stream.read_bits32(if mode == 0 { 4 } else { 6 }) as usize;

    let mut endpoints = [[0; 6]; 3];

    for c in &mut endpoints {
        for e in c {
            *e = stream.read_bits32(endpoint_bits);
        }
    }

    let mut pbits = [0; 6];
    for p in &mut pbits[0..pb] {
        *p = stream.read_bits32(1);
    }

    let mut weights = [0; 16];
    for (i, w) in weights.iter_mut().enumerate() {
        let weight_bits = if i == 0 || i == ANCHOR_THIRD1[part] || i == ANCHOR_THIRD2[part] {
            weight_bits - 1
        } else {
            weight_bits
        };
        *w = stream.read_bits32(weight_bits);
    }

    debug_assert!(stream.get_bits_read() == 128);

    for c in &mut endpoints {
        for (e, p) in c.iter_mut().zip(pbits) {
            *e = if pb != 0 {
                bc7_dequant_pbit(*e, p, endpoint_bits)
            } else {
                bc7_dequant(*e, endpoint_bits)
            };
        }
    }

    let mut block_colors = [[[0, 0, 0, 255]; 8]; 3];
    for (s, se) in block_colors.iter_mut().enumerate() {
        for (i, see) in se[0..weight_vals].iter_mut().enumerate() {
            for (color, e) in see[0..3].iter_mut().zip(endpoints) {
                *color = bc7_interp23(e[s * 2], e[s * 2 + 1], i, weight_bits);
            }
        }
    }

    for y in 0..4 {
        for x in 0..4 {
            let i = x + y * 4;
            writer(
                x,
                y,
                block_colors[PARTITION3[part * 16 + i]][weights[i] as usize],
                )
        }
    }
}

fn unpack_bc7_mode1_3_7<F: FnMut(usize, usize, [u8; 4])>(mode: u32, block: u128, mut writer: F) {
    let comps = if mode == 7 { 4 } else { 3 };
    let weight_bits = if mode == 1 { 3 } else { 2 };
    let endpoint_bits = match mode {
        7 => 5,
        1 => 6,
        3 => 7,
        _ => unreachable!(),
    };
    let pb = if mode == 1 { 2 } else { 4 };
    let shared_pbits = mode == 1;
    let weight_vals = 1 << weight_bits;

    let mut stream = InputBitStream::new(block);

    assert_eq!(stream.read_bits32(mode + 1), 1 << mode);

    let part = stream.read_bits32(6) as usize;

    let mut endpoints = [[0; 4]; 4];
    for c in &mut endpoints[0..comps] {
        for e in c {
            *e = stream.read_bits32(endpoint_bits);
        }
    }

    let mut pbits = [0; 4];
    for p in &mut pbits[0..pb] {
        *p = stream.read_bits32(1);
    }

    let mut weights = [0; 16];
    for (i, w) in weights.iter_mut().enumerate() {
        let weight_bits = if i == 0 || i == ANCHOR_SECOND[part] {
            weight_bits - 1
        } else {
            weight_bits
        };
        *w = stream.read_bits32(weight_bits);
    }

    debug_assert!(stream.get_bits_read() == 128);

    for c in &mut endpoints[0..comps] {
        for (e, ep) in c.iter_mut().enumerate() {
            *ep = bc7_dequant_pbit(
                *ep,
                pbits[if shared_pbits { e >> 1 } else { e }],
                endpoint_bits,
                );
        }
    }

    let mut block_colors = [[[0, 0, 0, 255]; 8]; 2];
    for (s, se) in block_colors.iter_mut().enumerate() {
        for (i, see) in se[0..weight_vals].iter_mut().enumerate() {
            for (color, e) in see[0..comps].iter_mut().zip(endpoints) {
                *color = bc7_interp23(e[s * 2], e[s * 2 + 1], i, weight_bits);
            }
        }
    }

    for y in 0..4 {
        for x in 0..4 {
            let i = x + y * 4;
            writer(
                x,
                y,
                block_colors[PARTITION2[part * 16 + i]][weights[i] as usize],
                )
        }
    }
}

fn unpack_bc7_mode4_5<F: FnMut(usize, usize, [u8; 4])>(mode: u32, block: u128, mut writer: F) {
    let weight_bits = 2;
    let a_weight_bits = if mode == 4 { 3 } else { 2 };
    let endpoint_bits = if mode == 4 { 5 } else { 7 };
    let a_endpoint_bits = if mode == 4 { 6 } else { 8 };

    let mut stream = InputBitStream::new(block);

    assert_eq!(stream.read_bits32(mode + 1), 1 << mode);

    let comp_rot = stream.read_bits32(2);
    let index_mode = if mode == 4 { stream.read_bits32(1) } else { 0 };

    let mut endpoints = [[0; 2]; 4];
    for (c, cc) in endpoints.iter_mut().enumerate() {
        for e in cc {
            *e = stream.read_bits32(if c == 3 {
                a_endpoint_bits
            } else {
                endpoint_bits
            });
        }
    }
    let weights_bits = if index_mode != 0 {
        [a_weight_bits, weight_bits]
    } else {
        [weight_bits, a_weight_bits]
    };

    let mut weights = [0; 16];
    let mut a_weights = [0; 16];
    let (first, second) = if index_mode != 0 {
        (&mut a_weights, &mut weights)
    } else {
        (&mut weights, &mut a_weights)
    };

    for (i, w) in first.iter_mut().enumerate() {
        let bit_decrease = u32::from(i == 0);
        *w = stream.read_bits32(weight_bits - bit_decrease);
    }

    for (i, w) in second.iter_mut().enumerate() {
        let bit_decrease = u32::from(i == 0);
        *w = stream.read_bits32(a_weight_bits - bit_decrease);
    }

    debug_assert!(stream.get_bits_read() == 128);

    for (c, cc) in endpoints.iter_mut().enumerate() {
        for e in cc {
            *e = bc7_dequant(
                *e,
                if c == 3 {
                    a_endpoint_bits
                } else {
                    endpoint_bits
                },
                );
        }
    }

    let mut block_colors = [[0; 4]; 8];
    for (i, b) in block_colors[0..1 << weights_bits[0]].iter_mut().enumerate() {
        for (color, e) in b.iter_mut().zip(endpoints) {
            *color = bc7_interp23(e[0], e[1], i, weights_bits[0]);
        }
    }

    for (i, b) in block_colors[0..1 << weights_bits[1]].iter_mut().enumerate() {
        b[3] = bc7_interp23(endpoints[3][0], endpoints[3][1], i, weights_bits[1]);
    }

    for y in 0..4 {
        for x in 0..4 {
            let i = x + y * 4;
            let mut color = block_colors[weights[i] as usize];
            color[3] = block_colors[a_weights[i] as usize][3];

            if comp_rot >= 1 {
                color.swap(3, (comp_rot - 1) as usize);
            }
            writer(x, y, color)
        }
    }
}

fn unpack_bc7_mode6<F: FnMut(usize, usize, [u8; 4])>(block: u128, mut writer: F) {
    let mut stream = InputBitStream::new(block);

    assert_eq!(stream.read_bits32(7), 1 << 6);

    let r0 = stream.read_bits32(7);
    let r1 = stream.read_bits32(7);
    let g0 = stream.read_bits32(7);
    let g1 = stream.read_bits32(7);
    let b0 = stream.read_bits32(7);
    let b1 = stream.read_bits32(7);
    let a0 = stream.read_bits32(7);
    let a1 = stream.read_bits32(7);
    let p0 = stream.read_bits32(1);

    let p1 = stream.read_bits32(1);

    let mut s = [0; 16];
    for (i, w) in s.iter_mut().enumerate() {
        let bits = if i == 0 { 3 } else { 4 };
        *w = stream.read_bits32(bits);
    }

    let r0 = (r0 << 1) | p0;
    let g0 = (g0 << 1) | p0;
    let b0 = (b0 << 1) | p0;
    let a0 = (a0 << 1) | p0;
    let r1 = (r1 << 1) | p1;
    let g1 = (g1 << 1) | p1;
    let b1 = (b1 << 1) | p1;
    let a1 = (a1 << 1) | p1;

    let mut vals = [[0; 4]; 16];
    for (val, w) in vals.iter_mut().zip(WEIGHTS4) {
        let iw = 64 - w;
        *val = [
            ((r0 * iw + r1 * w + 32) >> 6) as u8,
            ((g0 * iw + g1 * w + 32) >> 6) as u8,
            ((b0 * iw + b1 * w + 32) >> 6) as u8,
            ((a0 * iw + a1 * w + 32) >> 6) as u8,
        ];
    }

    for y in 0..4 {
        for x in 0..4 {
            let i = x + y * 4;
            writer(x, y, vals[s[i] as usize])
        }
    }
}

pub fn bc7_decompress_block<F: FnMut(usize, usize, [u8; 4])>(
    in_buf: &[u8; 16],
    mut writer: F,
    ) -> bool {
    let first_byte = in_buf[0];
    let block = u128::from_le_bytes(*in_buf);

    for mode in 0..=7 {
        if first_byte & (1 << mode) != 0 {
            match mode {
                0 | 2 => unpack_bc7_mode0_2(mode, block, writer),
                1 | 3 | 7 => unpack_bc7_mode1_3_7(mode, block, writer),
                4 | 5 => unpack_bc7_mode4_5(mode, block, writer),
                6 => unpack_bc7_mode6(block, writer),
                _ => unreachable!(),
            }
            return true;
        }
    }

    for y in 0..4 {
        for x in 0..4 {
            writer(x, y, [0xFF, 0, 0xFF, 0xFF])
        }
    }
    false
}

#[allow(dead_code)]
const PACKET_LEN: usize = 16;
#[allow(dead_code)]
const BLOCK_LEN: usize = PACKET_LEN * 4 * 8;

fn step<'a>(data: &'_ mut &'a [u8], max_len: usize) -> &'a [u8] {
    let len = std::cmp::min(data.len(), max_len);
    let ret = &data[0..len];
    *data = &data[len..];
    ret
}

#[allow(dead_code)]
pub trait TexCodec<const CELL_LEN: usize> {
    const CELL_WIDTH: usize;
    const CELL_HEIGHT: usize;
    type T;

    fn decode<F: FnMut(usize, usize, Self::T)>(cell: &[u8; CELL_LEN], writer: F);

    fn decode_image<F: FnMut(usize, usize, Self::T)>(
        data: &[u8],
        width: usize,
        height: usize,
        layout: u32,
        writer: F
    ) {
        match layout {
            0xFFFFFFFF => Self::decode_image_linear(data, width, height, writer),
            _ => ()
            //_ => Self::decode_image_nsw(data, width, height, super_width, super_height, writer),
        }
    }

    fn decode_image_linear<F: FnMut(usize, usize, Self::T)>(
        mut data: &[u8],
        width: usize,
        height: usize,
        mut writer: F,
    ) {
        let mut writer = |x, y, v| {
            if x >= width || y >= height {
                return;
            }
            writer(x, y, v)
        };

        let x_cells = (width + Self::CELL_WIDTH - 1) / Self::CELL_WIDTH;
        let y_cells = (height + Self::CELL_HEIGHT - 1) / Self::CELL_HEIGHT;

        for y_cell in 0..y_cells {
            for x_cell in 0..x_cells {
                let mut cell_buf = [0; CELL_LEN];
                let cell = step(&mut data, CELL_LEN);
                cell_buf[0..cell.len()].copy_from_slice(cell);
                Self::decode(&cell_buf, |x, y, v| {
                    writer(
                        x + x_cell * Self::CELL_WIDTH,
                        y + y_cell * Self::CELL_HEIGHT,
                        v,
                    )
                })
            }
        }
    }

    fn decode_block<F: FnMut(usize, usize, Self::T)>(
        mut block: &[u8], /* BLOCK_LEN or less */
        mut writer: F,
    ) {
        let cells_per_packet = PACKET_LEN / CELL_LEN;
        for i in 0..32 {
            if block.is_empty() {
                return;
            }
            let packet = step(&mut block, PACKET_LEN);
            let mut packet_buf = [0; PACKET_LEN];
            packet_buf[0..packet.len()].copy_from_slice(packet);
            let bx = ((i & 2) >> 1) | ((i & 16) >> 3);
            let by = (i & 1) | ((i & 4) >> 1) | ((i & 8) >> 1);
            for cell in 0..cells_per_packet {
                let cell_buf = &packet_buf[cell * CELL_LEN..][..CELL_LEN]
                    .try_into()
                    .unwrap();
                Self::decode(cell_buf, |x, y, v| {
                    writer(
                        x + cell * Self::CELL_WIDTH + bx * Self::CELL_WIDTH * cells_per_packet,
                        y + by * Self::CELL_HEIGHT,
                        v,
                    )
                })
            }
        }
    }

    fn decode_image_nsw<F: FnMut(usize, usize, Self::T)>(
        mut data: &[u8],
        width: usize,
        height: usize,
        super_width: usize,
        super_height: usize,
        mut writer: F,
    ) {
        let mut writer = |x, y, v| {
            if x >= width || y >= height {
                return;
            }
            writer(x, y, v)
        };

        let cells_per_packet = PACKET_LEN / CELL_LEN;

        let block_width = Self::CELL_WIDTH * cells_per_packet * 4;
        let block_height = Self::CELL_HEIGHT * 8;
        let super_block_width = block_width * super_width;
        let super_block_height = block_height * super_height;
        let hyper_width = (width + super_block_width - 1) / super_block_width;
        let hyper_height = (height + super_block_height - 1) / super_block_height;

        for hyper_y in 0..hyper_height {
            for hyper_x in 0..hyper_width {
                for super_x in 0..super_width {
                    for super_y in 0..super_height {
                        if data.is_empty() {
                            return;
                        }
                        let block = step(&mut data, BLOCK_LEN);
                        Self::decode_block(block, |x, y, v| {
                            writer(
                                x + block_width * super_x + super_block_width * hyper_x,
                                y + block_height * super_y + super_block_height * hyper_y,
                                v,
                            )
                        })
                    }
                }
            }
        }
    }
}

fn color5to8(value: u8) -> u8 {
    (value << 3) | (value >> 2)
}

fn color6to8(value: u8) -> u8 {
    (value << 2) | (value >> 4)
}

pub struct Bc1Unorm;

impl Bc1Unorm {
    fn decode_half<F: FnMut(usize, usize, [u8; 4])>(cell: &[u8; 8], mut writer: F) {
        let c0 = u16::from_le_bytes(cell[0..2].try_into().unwrap());
        let c1 = u16::from_le_bytes(cell[2..4].try_into().unwrap());
        let mut colors = [[0; 4]; 4];
        fn decode_color(c: u16) -> [u8; 4] {
            let (b, g, r) = c.bit_split((5, 6, 5));
            [
                color5to8(r as u8),
                color6to8(g as u8),
                color5to8(b as u8),
                0xFF,
            ]
        }
        colors[0] = decode_color(c0);
        colors[1] = decode_color(c1);
        if c0 > c1 {
            colors[2] = [
                ((2 * colors[0][0] as u32 + colors[1][0] as u32) / 3) as u8,
                ((2 * colors[0][1] as u32 + colors[1][1] as u32) / 3) as u8,
                ((2 * colors[0][2] as u32 + colors[1][2] as u32) / 3) as u8,
                0xFF,
            ];
            colors[3] = [
                ((2 * colors[1][0] as u32 + colors[0][0] as u32) / 3) as u8,
                ((2 * colors[1][1] as u32 + colors[0][1] as u32) / 3) as u8,
                ((2 * colors[1][2] as u32 + colors[0][2] as u32) / 3) as u8,
                0xFF,
            ];
        } else {
            colors[2] = [
                ((colors[0][0] as u32 + colors[1][0] as u32) / 2) as u8,
                ((colors[0][1] as u32 + colors[1][1] as u32) / 2) as u8,
                ((colors[0][2] as u32 + colors[1][2] as u32) / 2) as u8,
                0xFF,
            ];
            colors[3] = [0, 0, 0, 0];
        }
        for (y, &b) in cell[4..8].iter().enumerate() {
            let (b0, b1, b2, b3) = b.bit_split((2, 2, 2, 2));
            writer(0, y, colors[b0 as usize]);
            writer(1, y, colors[b1 as usize]);
            writer(2, y, colors[b2 as usize]);
            writer(3, y, colors[b3 as usize]);
        }
    }
}

impl TexCodec<8> for Bc1Unorm {
    const CELL_WIDTH: usize = 4;
    const CELL_HEIGHT: usize = 4;
    type T = [u8; 4];

    fn decode<F: FnMut(usize, usize, Self::T)>(cell: &[u8; 8], mut writer: F) {
        Self::decode_half(cell, &mut writer);
    }
}


pub struct Bc7Unorm;

impl TexCodec<16> for Bc7Unorm {
    const CELL_WIDTH: usize = 4;
    const CELL_HEIGHT: usize = 4;
    type T = [u8; 4];

    fn decode<F: FnMut(usize, usize, Self::T)>(cell: &[u8; 16], writer: F) {
        bc7_decompress_block(cell, writer);
    }
}


/*struct Astc<const W: usize, const H: usize>;

impl<const W: usize, const H: usize> TexCodec<16> for Astc<W, H> {
    const CELL_WIDTH: usize = W;
    const CELL_HEIGHT: usize = H;
    type T = [u8; 4];

    fn decode<F: FnMut(usize, usize, Self::T)>(cell: &[u8; 16], mut writer: F) {
        astc_decode::astc_decode_block(
            cell,
            astc_decode::Footprint::new(W as u32, H as u32),
            |x, y, v| writer(x as usize, y as usize, v),
        );
    }
}*/


pub struct Bc3Unorm;

impl TexCodec<16> for Bc3Unorm {
    const CELL_WIDTH: usize = 4;
    const CELL_HEIGHT: usize = 4;
    type T = [u8; 4];

    fn decode<F: FnMut(usize, usize, Self::T)>(cell: &[u8; 16], mut writer: F) {
        let mut color_buf = [[[0; 3]; 4]; 4];
        let mut alpha_buf = [[0; 4]; 4];
        Bc4Unorm::decode_half(cell[0..8].try_into().unwrap(), |x, y, v| {
            alpha_buf[x][y] = v[0]
        });
        Bc1Unorm::decode_half(cell[8..16].try_into().unwrap(), |x, y, v| {
            color_buf[x][y] = [v[0], v[1], v[2]]
        });
        for x in 0..4 {
            for y in 0..4 {
                let color = color_buf[x][y];
                writer(x, y, [color[0], color[1], color[2], alpha_buf[x][y]])
            }
        }
    }
}

pub struct Bc4Unorm;

impl Bc4Unorm {
    fn decode_half<F: FnMut(usize, usize, [u8; 4])>(cell: &[u8; 8], mut writer: F) {
        let mut c = [0; 8];
        let c0 = cell[0];
        let c1 = cell[1];
        c[0] = c0;
        c[1] = c1;
        if c[0] > c[1] {
            for (i, cc) in c[2..8].iter_mut().enumerate() {
                let f0 = 6 - i as u32;
                let f1 = i as u32 + 1;
                *cc = ((f0 * c0 as u32 + f1 * c1 as u32) / 7) as u8;
            }
        } else {
            for (i, cc) in c[2..6].iter_mut().enumerate() {
                let f0 = 4 - i as u32;
                let f1 = i as u32 + 1;
                *cc = ((f0 * c0 as u32 + f1 * c1 as u32) / 5) as u8;
            }
            c[6] = 0;
            c[7] = 255;
        }
        let mut buf = [0; 4];
        for super_y in 0..2 {
            buf[0..3].copy_from_slice(&cell[2 + super_y * 3..][..3]);
            let mut a = u32::from_le_bytes(buf);
            for y in 0..2 {
                for x in 0..4 {
                    let color = c[(a & 7) as usize];
                    writer(x, y + super_y * 2, [color, color, color, 255]);
                    a >>= 3;
                }
            }
        }
    }
}

impl TexCodec<8> for Bc4Unorm {
    const CELL_WIDTH: usize = 4;
    const CELL_HEIGHT: usize = 4;
    type T = [u8; 4];

    fn decode<F: FnMut(usize, usize, Self::T)>(cell: &[u8; 8], mut writer: F) {
        Self::decode_half(cell, &mut writer);
    }
}

pub struct Bc5Unorm;

impl TexCodec<16> for Bc5Unorm {
    const CELL_WIDTH: usize = 4;
    const CELL_HEIGHT: usize = 4;
    type T = [u8; 4];

    fn decode<F: FnMut(usize, usize, Self::T)>(cell: &[u8; 16], mut writer: F) {
        let mut red_buf = [[0; 4]; 4];
        let mut green_buf = [[0; 4]; 4];
        Bc4Unorm::decode_half(cell[0..8].try_into().unwrap(), |x, y, v| {
            red_buf[x][y] = v[0]
        });
        Bc4Unorm::decode_half(cell[8..16].try_into().unwrap(), |x, y, v| {
            green_buf[x][y] = v[0]
        });
        for x in 0..4 {
            for y in 0..4 {
                writer(x, y, [red_buf[x][y], green_buf[x][y], 0, 255])
            }
        }
    }
}

pub struct R8G8B8A8Unorm;

impl TexCodec<4> for R8G8B8A8Unorm {
    const CELL_WIDTH: usize = 1;
    const CELL_HEIGHT: usize = 1;
    type T = [u8; 4];

    fn decode<F: FnMut(usize, usize, Self::T)>(cell: &[u8; 4], mut writer: F) {
        writer(0, 0, *cell);
    }
}

pub struct R8Unorm;

impl TexCodec<1> for R8Unorm {
    const CELL_WIDTH: usize = 1;
    const CELL_HEIGHT: usize = 1;
    type T = [u8; 4];

    fn decode<F: FnMut(usize, usize, Self::T)>(cell: &[u8; 1], mut writer: F) {
        let c = cell[0];
        writer(0, 0, [c, c, c, 255])
    }
}

pub struct R8G8Unorm;

impl TexCodec<2> for R8G8Unorm {
    const CELL_WIDTH: usize = 1;
    const CELL_HEIGHT: usize = 1;
    type T = [u8; 4];

    fn decode<F: FnMut(usize, usize, Self::T)>(cell: &[u8; 2], mut writer: F) {
        let r = cell[0];
        let g = cell[1];
        writer(0, 0, [r, g, 0, 255])
    }
}
