//! cs-stats: statistics used by the backtester and the research phase.
//!
//! Phase 3 adds the block bootstrap ([`bootstrap`]); Phase 4 adds the HAC t-test and
//! Benjamini-Hochberg FDR control ([`hyp`]) and rolling walk-forward windows ([`walkforward`]);
//! the probability model follows in Phase 5. Advisory only: nothing in this workspace places
//! orders.

pub mod bootstrap;
pub mod hyp;
pub mod walkforward;

/// `SplitMix64`: small deterministic RNG (same algorithm as `cs_patterns::util::Rng`, kept here
/// so this crate has no dependencies).
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    #[allow(clippy::cast_precision_loss)]
    pub fn f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform integer in [0, n) (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n.max(1)
    }
}
