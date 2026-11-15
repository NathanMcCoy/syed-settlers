// Seeded random numbers: the same seed always gives the same island (and later the same dice), so
// games can be replayed and tested. The whole state is one 32-bit number, so it's cheap to copy
// (bots that look ahead copy the game) and to save. In online games the server keeps it secret.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rng {
    pub state: u32,
}

impl Rng {
    pub fn new(seed: u32) -> Rng {
        Rng { state: seed }
    }

    // A text seed (what people type and share) turned into a number: FNV-1a over its UTF-8 bytes
    pub fn from_text(text: &str) -> Rng {
        let mut h: u32 = 2166136261;
        for &b in text.as_bytes() {
            h ^= b as u32;
            h = h.wrapping_mul(16777619);
        }
        Rng::new(h)
    }

    // mulberry32: small, fast and good enough for games (not for secrets)
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_add(0x6D2B79F5);
        let mut t = self.state;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        t ^ (t >> 14)
    }

    // 0 <= x < 1
    #[inline]
    pub fn next_f64(&mut self) -> f64 {
        self.next_u32() as f64 / 4294967296.0
    }

    // 0 .. n-1, without the bias of `% n` (n must be at least 1)
    #[inline]
    pub fn below(&mut self, n: usize) -> usize {
        ((self.next_u32() as u64 * n as u64) >> 32) as usize
    }

    pub fn pick<'a, T>(&mut self, list: &'a [T]) -> &'a T {
        &list[self.below(list.len())]
    }

    pub fn shuffle<T>(&mut self, list: &mut [T]) {
        for i in (1..list.len()).rev() {
            let j = self.below(i + 1);
            list.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_numbers() {
        let mut a = Rng::from_text("betta");
        let mut b = Rng::from_text("betta");
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
        assert_ne!(Rng::from_text("betta").next_u32(), Rng::from_text("bettb").next_u32());
    }

    #[test]
    fn below_is_even_and_in_range() {
        let mut r = Rng::new(1);
        let mut counts = [0u32; 6];
        for _ in 0..60_000 {
            counts[r.below(6)] += 1;
        }
        for c in counts {
            assert!((9_400..10_600).contains(&c), "{counts:?}");
        }
    }

    #[test]
    fn shuffle_keeps_every_item() {
        let mut r = Rng::new(7);
        let mut v: Vec<u32> = (0..50).collect();
        r.shuffle(&mut v);
        let mut sorted = v.clone();
        sorted.sort();
        assert_eq!(sorted, (0..50).collect::<Vec<_>>());
        assert_ne!(v, sorted);
    }
}
