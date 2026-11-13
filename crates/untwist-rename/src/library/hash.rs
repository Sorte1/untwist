const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
const BASE: u64 = 0x9e37_79b9_7f4a_7c15;
const SEED_LOW: u64 = 0x243f_6a88_85a3_08d3;
const SEED_HIGH: u64 = 0x1319_8a2e_0370_7344;

pub fn mix(value: u64) -> u64 {
    let mut z = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

pub fn fnv(tag: u8, bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET ^ u64::from(tag);
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    mix(hash ^ (bytes.len() as u64).rotate_left(32))
}

fn power(mut base: u64, mut exponent: u32) -> u64 {
    let mut result: u64 = 1;
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = result.wrapping_mul(base);
        }
        base = base.wrapping_mul(base);
        exponent >>= 1;
    }
    result
}

#[derive(Debug, Default)]
pub struct PolyHash {
    prefix: Vec<u64>,
}

impl PolyHash {
    pub fn new() -> Self {
        Self { prefix: vec![0] }
    }

    pub fn push(&mut self, token: u64) {
        let last = *self.prefix.last().unwrap_or(&0);
        self.prefix
            .push(last.wrapping_mul(BASE).wrapping_add(token));
    }

    pub fn range(&self, start: u32, len: u32) -> u64 {
        let start_index = start as usize;
        let end_index = start_index + len as usize;
        let (Some(&end), Some(&begin)) = (self.prefix.get(end_index), self.prefix.get(start_index))
        else {
            return 0;
        };
        mix(end.wrapping_sub(begin.wrapping_mul(power(BASE, len))))
    }
}

#[derive(Debug)]
pub struct Exact {
    low: u64,
    high: u64,
    count: u64,
}

impl Exact {
    pub fn new() -> Self {
        Self {
            low: SEED_LOW,
            high: SEED_HIGH,
            count: 0,
        }
    }

    pub fn push(&mut self, token: u64) {
        self.count += 1;
        self.low = mix(self.low ^ token).wrapping_add(self.count);
        self.high = mix(self.high.rotate_left(17) ^ token.wrapping_mul(FNV_PRIME)) ^ self.count;
    }

    pub fn finish(&self) -> u128 {
        let low = mix(self.low ^ self.count);
        let high = mix(self.high ^ self.count.rotate_left(29));
        (u128::from(high) << 64) | u128::from(low)
    }
}

impl Default for Exact {
    fn default() -> Self {
        Self::new()
    }
}
