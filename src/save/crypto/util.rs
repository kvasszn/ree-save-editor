use num_integer::Integer;

pub struct SplitMix64;

impl SplitMix64 {
    pub const ADD: u64 = 0x9e3779b97f4a7c15;
    pub const MUL1: u64 = 0xbf58476d1ce4e5b9;
    pub const MUL2: u64 = 0x94d049bb133111eb;
    pub fn next_int(state: &mut u64) -> u64 {
        *state = state.wrapping_add(Self::ADD);
        let mut z: u64 = *state;
        z = (z ^ (z >> 30)).wrapping_mul(Self::MUL1);
        z = (z ^ (z >> 27)).wrapping_mul(Self::MUL2);
        return z ^ (z >> 31);
    }

    pub fn last_int(state: &mut u64) -> u64 {
        *state = state.wrapping_add(Self::ADD);
        let mut z: u64 = *state;
        z = (z ^ (z >> 30)).wrapping_mul(Self::MUL1);
        z = (z ^ (z >> 27)).wrapping_mul(Self::MUL2);
        return z ^ (z >> 31);
    }

    pub const INV_MUL1: u64 = 0x96de1b173f119089;
    pub const INV_MUL2: u64 = 0x319642b2d24d8ec3;
    pub fn unmix(mut z: u64) -> u64 {
        z ^= (z >> 31) ^ (z >> 62);
        // z = (z ^ (z >> 27)).wrapping_mul(Self::MUL2);
        z = z.wrapping_mul(Self::INV_MUL2);
        z ^= (z >> 27) ^ (z >> 54);
        // z = (z ^ (z >> 30)).wrapping_mul(Self::MUL1);
        z = z.wrapping_mul(Self::INV_MUL1);
        z ^= (z >> 30) ^ (z >> 60);

        z
    }
}

pub mod backend {
    // Using rug
    #[cfg(target_os = "linux")]
    pub mod rug {
        pub use rug::{Integer, integer::Order};

        pub fn mod_exp(base: &Integer, exp: &Integer, modulus: &Integer) -> Integer {
            base.pow_mod_ref(exp, modulus).unwrap().into()
        }

        pub fn bytes_to_int(bytes: &[u8]) -> Integer {
            Integer::from_digits(bytes, Order::Lsf)
        }

        pub fn int_to_bytes_le<const N: usize>(n: &Integer) -> [u8; N] {
            let mut out = [0u8; N];
            let digits = n.to_digits::<u8>(Order::Lsf);
            let len = digits.len().min(N);
            out[..len].copy_from_slice(&digits[..len]);
            out
        }
    }

    pub mod num_bigint {
        pub use num_bigint::{BigInt as Integer, Sign};

        pub fn mod_exp(base: &Integer, exp: &Integer, modulus: &Integer) -> Integer {
            base.modpow(exp, modulus)
        }

        pub fn bytes_to_int(bytes: &[u8]) -> Integer {
            Integer::from_bytes_le(Sign::Plus, bytes)
        }

        pub fn int_to_bytes_le<const N: usize>(n: &Integer) -> [u8; N] {
            let mut out = [0u8; N];
            let digits = n.to_bytes_le().1;
            let len = digits.len().min(N);
            out[..len].copy_from_slice(&digits[..len]);
            out
        }
    }
}

pub trait EccInteger: Sized + Clone + PartialOrd {
    fn from_u64(val: u64) -> Self;
    fn mod_exp(&self, exp: &Self, modulus: &Self) -> Self;
    fn from_bytes_le(bytes: &[u8]) -> Self;
    fn to_bytes_le<const N: usize>(&self) -> [u8; N];

    // Add basic arithmetic requirements
    fn add_mod(&self, other: &Self, p: &Self) -> Self;
    fn sub_mod(&self, other: &Self, p: &Self) -> Self;
    fn mul_mod(&self, other: &Self, p: &Self) -> Self;
    fn is_odd(&self) -> bool;
    fn div_2(&self) -> Self;
    fn to_u64_le_bytes(&self) -> [u8; 8] {
        let full_bytes = self.to_bytes_le::<32>();
        let mut out = [0u8; 8];
        out.copy_from_slice(&full_bytes[0..8]);
        out
    }
}

#[cfg(target_os = "linux")]
impl EccInteger for rug::Integer {
    fn from_u64(val: u64) -> Self {
        rug::Integer::from(val)
    }
    fn mod_exp(&self, exp: &Self, modulus: &Self) -> Self {
        self.pow_mod_ref(exp, modulus).unwrap().into()
    }
    fn from_bytes_le(bytes: &[u8]) -> Self {
        rug::Integer::from_digits(bytes, rug::integer::Order::Lsf)
    }
    fn to_bytes_le<const N: usize>(&self) -> [u8; N] {
        let mut out = [0u8; N];
        let digits = self.to_digits::<u8>(rug::integer::Order::Lsf);
        let len = digits.len().min(N);
        out[..len].copy_from_slice(&digits[..len]);
        out
    }
    fn add_mod(&self, other: &Self, p: &Self) -> Self {
        use rug::Complete;
        (self + other).complete() % p
    }
    fn sub_mod(&self, other: &Self, p: &Self) -> Self {
        use rug::Complete;
        let res = (self - other).complete() % p;
        if res < 0 { res + p } else { res }
    }
    fn mul_mod(&self, other: &Self, p: &Self) -> Self {
        use rug::Complete;
        (self * other).complete() % p
    }
    fn is_odd(&self) -> bool {
        self.is_odd()
    }
    fn div_2(&self) -> Self {
        self.clone() >> 1
    }
}

impl EccInteger for num_bigint::BigInt {
    fn from_u64(val: u64) -> Self {
        num_bigint::BigInt::from(val)
    }
    fn mod_exp(&self, exp: &Self, modulus: &Self) -> Self {
        self.modpow(exp, modulus)
    }
    fn from_bytes_le(bytes: &[u8]) -> Self {
        num_bigint::BigInt::from_bytes_le(num_bigint::Sign::Plus, bytes)
    }
    fn to_bytes_le<const N: usize>(&self) -> [u8; N] {
        let mut out = [0u8; N];
        let digits = self.to_bytes_le().1;
        let len = digits.len().min(N);
        out[..len].copy_from_slice(&digits[..len]);
        out
    }
    fn add_mod(&self, other: &Self, p: &Self) -> Self {
        (self + other) % p
    }
    fn sub_mod(&self, other: &Self, p: &Self) -> Self {
        let res = (self - other) % p;
        if res.sign() == num_bigint::Sign::Minus {
            res + p
        } else {
            res
        }
    }
    fn mul_mod(&self, other: &Self, p: &Self) -> Self {
        (self * other) % p
    }
    fn is_odd(&self) -> bool {
        !self.is_even()
    }
    fn div_2(&self) -> Self {
        self >> 1
    }
}

pub fn mod_inverse<T: EccInteger>(k: &T, p: &T) -> T {
    // Fermat's Little Theorem: k^(p-2) mod p
    let two = T::from_u64(2);
    let exp = p.sub_mod(&two, p);
    k.mod_exp(&exp, p)
}

pub fn point_add<T: EccInteger>(
    p1: Option<(T, T)>,
    p2: Option<(T, T)>,
    a: &T,
    p: &T,
) -> Option<(T, T)> {
    // Handle identity element (Point at Infinity)
    let (x1, y1) = match &p1 {
        Some(val) => val.clone(),
        None => return p2,
    };
    let (x2, y2) = match p2 {
        Some(val) => val,
        None => return p1,
    };

    // If x coordinates are same
    if x1 == x2 {
        // If y coordinates are different, it's P + (-P) = O
        if y1 != y2 {
            return None;
        }
        // If y is 0, doubling gives Point at Infinity
        if y1 == T::from_u64(0) {
            return None;
        }
    }

    let lambda = if x1 == x2 && y1 == y2 {
        // Case: Point Doubling
        // lambda = (3x1^2 + a) / (2y1) mod p
        let x_sq = x1.mul_mod(&x1, p);
        let num = x_sq.mul_mod(&T::from_u64(3), p).add_mod(a, p);
        let den = y1.mul_mod(&T::from_u64(2), p);
        num.mul_mod(&mod_inverse(&den, p), p)
    } else {
        // Case: Standard Addition
        // lambda = (y2 - y1) / (x2 - x1) mod p
        let num = y2.sub_mod(&y1, p);
        let den = x2.sub_mod(&x1, p);
        num.mul_mod(&mod_inverse(&den, p), p)
    };

    // x3 = lambda^2 - x1 - x2 mod p
    let lam_sq = lambda.mul_mod(&lambda, p);
    let x3 = lam_sq.sub_mod(&x1, p).sub_mod(&x2, p);

    // y3 = lambda(x1 - x3) - y1 mod p
    let x_diff = x1.sub_mod(&x3, p);
    let y3 = lambda.mul_mod(&x_diff, p).sub_mod(&y1, p);

    Some((x3, y3))
}

pub fn scalar_mult<T: EccInteger>(k: &T, point: (T, T), a: &T, p: &T) -> Option<(T, T)> {
    let mut result: Option<(T, T)> = None;
    let mut addend = Some(point);
    let mut k = k.clone();

    // Double-and-Add Algorithm
    while k > T::from_u64(0) {
        if k.is_odd() {
            result = point_add(result, addend.clone(), a, p);
        }
        // Double the point for the next bit
        if let Some(p_val) = addend {
            addend = point_add(Some(p_val.clone()), Some(p_val), a, p);
        }
        k = k.div_2();
    }
    result
}

pub mod elgamal {
    use bytemuck::{Pod, Zeroable};
    use hex_literal::hex;
    #[cfg(target_os = "linux")]
    use super::backend::rug::*;
    #[cfg(not(target_os = "linux"))]
    use super::backend::num_bigint::*;

    #[repr(C)]
    #[derive(Debug, Clone, Copy, Pod, Zeroable)]
    pub struct Pair(pub [u8; 64], pub [u8; 64]);

    // elgamal??
    #[derive(Debug, Clone)]
    pub struct Elgamal {
        p: Integer, // prime
        #[allow(unused)]
        q: Integer, // prime
        r: Integer, // integer
        s: Integer, // r ^ u mod q
        u: Integer, // id mod q
        e: Integer, // 0x14
    }

    impl Elgamal {
        pub const P: [u8; 32] =
            hex!("f33b6fb972a0b72515e45c391829e182ad8a9bdc0a64d3444d79c810ab863717");
        pub const Q: [u8; 32] =
            hex!("f99db75c39d0db920a72ae1c8c9470c156c54d6e05b269a2a63c648855c39b0b");
        pub const R: [u8; 32] =
            hex!("e66f544afcce68c5ef07b9a07b277585344a1db61376e831f73b9fbd5f44f715");

        pub fn init(u: u64) -> crate::reerr::Result<Self> {
            let p = bytes_to_int(&Self::P);
            let q = bytes_to_int(&Self::Q);
            let r = bytes_to_int(&Self::R);
            let u = Integer::from(u);
            let u = u % &q;
            let s = mod_exp(&r, &u, &p);
            let e = Integer::from(0x14u64);
            Ok(Elgamal { p, q, r, s, u, e })
        }

        /*pub fn update_u(&mut self, u: u64) {
          let u = Integer::from(u) % &self.q;
          let s = self.r.pow_mod_ref(&u, &self.p).unwrap().complete();
          self.u = u;
          self.s = s;
          }*/

        pub fn encrypt(&self, pt: [u8; 8]) -> Pair {
            let x0 = mod_exp(&self.r, &self.e, &self.p);
            let x1 = mod_exp(&self.s, &self.e, &self.p);
            let pt = bytes_to_int(&pt);
            let ct = x1 * pt;
            let res0 = int_to_bytes_le::<64>(&x0);
            let res1 = int_to_bytes_le::<64>(&ct);
            Pair(res0, res1)
        }

        pub fn decrypt_ex(chunk: &Pair, p: &Integer, u: &Integer) -> [u8; 8] {
            let x0 = bytes_to_int(&chunk.0);
            let ct = bytes_to_int(&chunk.1);
            let x = mod_exp(&x0, &u, p);
            let k = ct / x;
            int_to_bytes_le::<8>(&k)
        }

        pub fn decrypt(&self, chunk: &Pair) -> [u8; 8] {
            let x0 = bytes_to_int(&chunk.0);
            let ct = bytes_to_int(&chunk.1);
            let x = mod_exp(&x0, &self.u, &self.p);
            let k = ct / x;
            int_to_bytes_le::<8>(&k)
        }

        pub fn encrypt_bytes(&self, pt: [u8; 32]) -> [Pair; 4] {
            let mut chunks = [Pair([0u8; 64], [0u8; 64]); 4];
            for i in 0..4 {
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&pt[i * 8..i * 8 + 8]);
                chunks[i] = self.encrypt(buf);
            }
            chunks
        }

        pub fn decrypt_pairs(&self, pairs: [Pair; 4]) -> [u8; 32] {
            let mut res = [0u8; 32];
            for (i, pair) in pairs.iter().enumerate() {
                let x = self.decrypt(pair);
                res[i * 8..i * 8 + 8].copy_from_slice(&x);
            }
            res
        }

        pub fn decrypt_block(&self, block: Block) -> [u8; 32] {
            let mut res = [0u8; 32];
            for (i, chunk) in block.chunks.iter().enumerate() {
                let x = self.decrypt(chunk);
                res[i * 8..i * 8 + 8].copy_from_slice(&x);
            }
            res
        }

        pub fn decrypt_pairs_ex(pairs: &[Pair; 4], p: &Integer, u: &Integer) -> [u8; 32] {
            let mut res = [0u8; 32];
            for (i, pair) in pairs.iter().enumerate() {
                let x = Self::decrypt_ex(pair, p, u);
                res[i * 8..i * 8 + 8].copy_from_slice(&x);
            }
            res
        }
    }

    #[repr(C)]
    #[derive(Debug, Clone, Copy, Pod, Zeroable)]
    pub struct Block {
        pub chunks: [Pair; 4], // key/iv checker
        pub checksum: u64,     // computed from city64 or farmhash of decrypted data
        pub litterally_no_idea: [u8; 8],
    }
}
