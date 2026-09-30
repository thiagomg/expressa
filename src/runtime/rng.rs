//! Tiny xorshift RNG for `aleatorio` / `semente`. No extra crate.

use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) struct Rng {
    state: u64,
}

impl Rng {
    pub(crate) fn new() -> Self {
        Self::seeded(mix_seed())
    }

    pub(crate) fn seeded(seed: u64) -> Self {
        let mut rng = Self { state: 1 };
        rng.set_seed(seed);
        rng
    }

    pub(crate) fn set_seed(&mut self, seed: u64) {
        let mut x = splitmix64(seed);
        if x == 0 {
            x = 0x9E37_79B9_7F4A_7C15;
        }
        self.state = x;
    }

    pub(crate) fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    pub(crate) fn inclusive(&mut self, min: i64, max: i64) -> i64 {
        debug_assert!(min <= max);
        let span = (max as i128) - (min as i128) + 1;
        if span > u64::MAX as i128 {
            return self.next_u64() as i64;
        }
        min.wrapping_add((self.next_u64() % (span as u64)) as i64)
    }
}

fn splitmix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn mix_seed() -> u64 {
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1);
    t ^ (std::process::id() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inclusive_stays_in_range() {
        let mut rng = Rng::seeded(1);
        for _ in 0..200 {
            let n = rng.inclusive(1, 6);
            assert!((1..=6).contains(&n), "{n}");
        }
    }

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::seeded(42);
        let mut b = Rng::seeded(42);
        for _ in 0..20 {
            assert_eq!(a.inclusive(-3, 9), b.inclusive(-3, 9));
        }
    }

    #[test]
    fn singleton_range() {
        let mut rng = Rng::seeded(7);
        assert_eq!(rng.inclusive(4, 4), 4);
    }
}
