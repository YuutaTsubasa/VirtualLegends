//! SplitMix64。戰鬥裡所有的亂數都從 Server 發的種子來，Server 重算時才會一模一樣，
//! 所以不用 `rand`（換版本可能換演算法）。

#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// 0..n（n 很小，取餘數的偏差可以忽略）
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }

    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = self.below(i as u64 + 1) as usize;
            v.swap(i, j);
        }
    }

    pub fn state(&self) -> u64 {
        self.0
    }
}
