//! Streaming building blocks: ring buffer, ATR, EMA, SMA, rolling stats, ADX, candle parts and a
//! tiny deterministic RNG. All of them allocate once at construction and never again.

use cs_core::Candle;

/// Fixed-capacity ring buffer. Allocates its storage once in [`Ring::new`]; `push` never
/// allocates (the oldest element is overwritten once full).
#[derive(Debug, Clone)]
pub struct Ring<T> {
    buf: Vec<T>,
    head: usize,
    len: usize,
}

impl<T: Copy + Default> Ring<T> {
    pub fn new(cap: usize) -> Ring<T> {
        Ring {
            buf: vec![T::default(); cap.max(1)],
            head: 0,
            len: 0,
        }
    }

    pub fn cap(&self) -> usize {
        self.buf.len()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn is_full(&self) -> bool {
        self.len == self.buf.len()
    }

    pub fn clear(&mut self) {
        self.head = 0;
        self.len = 0;
    }

    pub fn push(&mut self, x: T) {
        let cap = self.buf.len();
        if self.len < cap {
            self.buf[(self.head + self.len) % cap] = x;
            self.len += 1;
        } else {
            self.buf[self.head] = x;
            self.head = (self.head + 1) % cap;
        }
    }

    /// `k`-th most recent element (0 = newest).
    pub fn back(&self, k: usize) -> Option<T> {
        if k >= self.len {
            return None;
        }
        let cap = self.buf.len();
        Some(self.buf[(self.head + self.len - 1 - k) % cap])
    }

    /// `i`-th oldest element (0 = oldest).
    pub fn get(&self, i: usize) -> Option<T> {
        if i >= self.len {
            return None;
        }
        Some(self.buf[(self.head + i) % self.buf.len()])
    }

    /// Oldest to newest.
    pub fn iter(&self) -> impl Iterator<Item = T> + '_ {
        (0..self.len).map(move |i| self.buf[(self.head + i) % self.buf.len()])
    }
}

/// Push into a capacity-bounded `Vec` without ever growing it: when full, the element chosen by
/// `evict` (an index) is removed first. The `Vec` must have been created with this capacity.
pub fn bounded_push<T>(v: &mut Vec<T>, cap: usize, x: T, evict: impl Fn(&[T]) -> usize) {
    if v.len() >= cap {
        let k = evict(v).min(v.len().saturating_sub(1));
        if !v.is_empty() {
            v.remove(k);
        }
    }
    v.push(x);
}

/// Wilder ATR on true range. Before `n` candles it is the mean true range so far.
#[derive(Debug, Clone)]
pub struct Atr {
    n: usize,
    count: usize,
    prev_close: f64,
    sum: f64,
    value: f64,
}

impl Atr {
    pub fn new(n: usize) -> Atr {
        Atr {
            n: n.max(1),
            count: 0,
            prev_close: f64::NAN,
            sum: 0.0,
            value: f64::NAN,
        }
    }

    pub fn update(&mut self, c: &Candle) -> f64 {
        let tr = if self.count == 0 {
            c.high - c.low
        } else {
            (c.high - c.low)
                .max((c.high - self.prev_close).abs())
                .max((c.low - self.prev_close).abs())
        };
        self.prev_close = c.close;
        self.count += 1;
        if self.count <= self.n {
            self.sum += tr;
            self.value = self.sum / self.count as f64;
        } else {
            self.value = (self.value * (self.n - 1) as f64 + tr) / self.n as f64;
        }
        self.value
    }

    pub fn ready(&self) -> bool {
        self.count >= self.n
    }

    /// Current value (`NaN` before the first candle).
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Value when ready, else `NaN`.
    pub fn get(&self) -> f64 {
        if self.ready() { self.value } else { f64::NAN }
    }
}

/// EMA seeded with the SMA of the first `n` values; `ready` after `n` values.
#[derive(Debug, Clone)]
pub struct Ema {
    n: usize,
    count: usize,
    sum: f64,
    value: f64,
    alpha: f64,
}

impl Ema {
    pub fn new(n: usize) -> Ema {
        let n = n.max(1);
        Ema {
            n,
            count: 0,
            sum: 0.0,
            value: f64::NAN,
            alpha: 2.0 / (n as f64 + 1.0),
        }
    }

    pub fn update(&mut self, x: f64) -> f64 {
        self.count += 1;
        match self.count.cmp(&self.n) {
            std::cmp::Ordering::Less => self.sum += x,
            std::cmp::Ordering::Equal => {
                self.sum += x;
                self.value = self.sum / self.n as f64;
            }
            std::cmp::Ordering::Greater => self.value += self.alpha * (x - self.value),
        }
        self.get()
    }

    pub fn ready(&self) -> bool {
        self.count >= self.n
    }

    pub fn get(&self) -> f64 {
        if self.ready() { self.value } else { f64::NAN }
    }
}

/// Rolling mean and standard deviation of the last `n` values.
#[derive(Debug, Clone)]
pub struct RollStats {
    win: Ring<f64>,
    sum: f64,
    sumsq: f64,
}

impl RollStats {
    pub fn new(n: usize) -> RollStats {
        RollStats {
            win: Ring::new(n),
            sum: 0.0,
            sumsq: 0.0,
        }
    }

    pub fn push(&mut self, x: f64) {
        if self.win.is_full()
            && let Some(old) = self.win.get(0)
        {
            self.sum -= old;
            self.sumsq -= old * old;
        }
        self.win.push(x);
        self.sum += x;
        self.sumsq += x * x;
    }

    pub fn ready(&self) -> bool {
        self.win.is_full()
    }

    /// Mean when full, else `NaN`.
    pub fn mean(&self) -> f64 {
        if self.ready() {
            self.sum / self.win.len() as f64
        } else {
            f64::NAN
        }
    }

    /// Population standard deviation when full, else `NaN`.
    pub fn std(&self) -> f64 {
        if !self.ready() {
            return f64::NAN;
        }
        let n = self.win.len() as f64;
        let m = self.sum / n;
        (self.sumsq / n - m * m).max(0.0).sqrt()
    }
}

/// Wilder ADX with +DI / -DI. `ready` after `2n` candles.
#[derive(Debug, Clone)]
pub struct Adx {
    n: usize,
    count: usize,
    prev: Option<Candle>,
    tr_s: f64,
    pdm_s: f64,
    mdm_s: f64,
    dx_sum: f64,
    dx_count: usize,
    value: f64,
    pdi: f64,
    mdi: f64,
}

impl Adx {
    pub fn new(n: usize) -> Adx {
        Adx {
            n: n.max(1),
            count: 0,
            prev: None,
            tr_s: 0.0,
            pdm_s: 0.0,
            mdm_s: 0.0,
            dx_sum: 0.0,
            dx_count: 0,
            value: f64::NAN,
            pdi: f64::NAN,
            mdi: f64::NAN,
        }
    }

    pub fn update(&mut self, c: &Candle) {
        let Some(p) = self.prev.replace(*c) else {
            return;
        };
        let up = c.high - p.high;
        let down = p.low - c.low;
        let pdm = if up > down && up > 0.0 { up } else { 0.0 };
        let mdm = if down > up && down > 0.0 { down } else { 0.0 };
        let tr = (c.high - c.low)
            .max((c.high - p.close).abs())
            .max((c.low - p.close).abs());
        self.count += 1;
        let n = self.n as f64;
        if self.count <= self.n {
            self.tr_s += tr;
            self.pdm_s += pdm;
            self.mdm_s += mdm;
            if self.count < self.n {
                return;
            }
        } else {
            self.tr_s = self.tr_s - self.tr_s / n + tr;
            self.pdm_s = self.pdm_s - self.pdm_s / n + pdm;
            self.mdm_s = self.mdm_s - self.mdm_s / n + mdm;
        }
        if self.tr_s <= 0.0 {
            return;
        }
        self.pdi = 100.0 * self.pdm_s / self.tr_s;
        self.mdi = 100.0 * self.mdm_s / self.tr_s;
        let s = self.pdi + self.mdi;
        let dx = if s > 0.0 {
            100.0 * (self.pdi - self.mdi).abs() / s
        } else {
            0.0
        };
        if self.dx_count < self.n {
            self.dx_sum += dx;
            self.dx_count += 1;
            if self.dx_count == self.n {
                self.value = self.dx_sum / n;
            }
        } else {
            self.value = (self.value * (n - 1.0) + dx) / n;
        }
    }

    /// (ADX, +DI, -DI); ADX is `NaN` until ready.
    pub fn get(&self) -> (f64, f64, f64) {
        (self.value, self.pdi, self.mdi)
    }
}

/// Candle parts (knowledge S1).
pub fn body(c: &Candle) -> f64 {
    (c.close - c.open).abs()
}

pub fn range(c: &Candle) -> f64 {
    c.high - c.low
}

pub fn upper(c: &Candle) -> f64 {
    c.high - c.open.max(c.close)
}

pub fn lower(c: &Candle) -> f64 {
    c.open.min(c.close) - c.low
}

pub fn body_top(c: &Candle) -> f64 {
    c.open.max(c.close)
}

pub fn body_bot(c: &Candle) -> f64 {
    c.open.min(c.close)
}

pub fn is_bull(c: &Candle) -> bool {
    c.close > c.open
}

pub fn is_bear(c: &Candle) -> bool {
    c.close < c.open
}

/// `SplitMix64`: small deterministic RNG for tests, synthetic data and gallery sampling.
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
    pub fn f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform integer in [0, n) (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n.max(1)
    }

    /// Standard normal (Box-Muller).
    pub fn normal(&mut self) -> f64 {
        let u1 = self.f64().max(1e-300);
        let u2 = self.f64();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(o: f64, h: f64, l: f64, cl: f64) -> Candle {
        Candle {
            open_time: 0,
            open: o,
            high: h,
            low: l,
            close: cl,
            volume: 1.0,
            quote_volume: 1.0,
            trades: 1,
            taker_buy_base: 0.5,
            taker_buy_quote: 0.5,
        }
    }

    #[test]
    fn ring_wraps() {
        let mut r = Ring::new(3);
        for x in 1..=5 {
            r.push(x);
        }
        assert_eq!(r.iter().collect::<Vec<_>>(), vec![3, 4, 5]);
        assert_eq!(r.back(0), Some(5));
        assert_eq!(r.back(2), Some(3));
        assert_eq!(r.back(3), None);
        assert_eq!(r.get(0), Some(3));
    }

    #[test]
    fn atr_constant_range() {
        let mut a = Atr::new(14);
        let mut p = 100.0;
        for _ in 0..30 {
            a.update(&c(p, p + 5.0, p - 5.0, p));
            p += 0.0;
        }
        assert!((a.get() - 10.0).abs() < 1e-9);
    }

    #[test]
    fn ema_seed_and_converge() {
        let mut e = Ema::new(3);
        assert!(e.update(1.0).is_nan());
        assert!(e.update(2.0).is_nan());
        assert!((e.update(3.0) - 2.0).abs() < 1e-12);
        assert!((e.update(4.0) - 3.0).abs() < 1e-12);
    }

    #[test]
    fn rollstats() {
        let mut s = RollStats::new(4);
        for x in [1.0, 2.0, 3.0, 4.0, 5.0] {
            s.push(x);
        }
        assert!((s.mean() - 3.5).abs() < 1e-12);
        assert!((s.std() - 1.25f64.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn adx_trending_is_high() {
        let mut a = Adx::new(14);
        let mut p = 100.0;
        for _ in 0..60 {
            a.update(&c(p, p + 2.0, p - 0.5, p + 1.5));
            p += 1.5;
        }
        let (adx, pdi, mdi) = a.get();
        assert!(adx > 50.0 && pdi > mdi, "{adx} {pdi} {mdi}");
    }
}
