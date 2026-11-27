//! Rolling walk-forward windows over a sequence of periods (e.g. calendar months).

use std::ops::Range;

/// One walk-forward step: train on `train` periods, then test on `test` periods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    pub train: Range<usize>,
    pub test: Range<usize>,
}

/// Rolling windows over `n` periods: train `train` periods, test the following `test` periods,
/// move by `step`. Only complete windows are returned.
pub fn rolling(n: usize, train: usize, test: usize, step: usize) -> Vec<Window> {
    let mut v = Vec::new();
    if train == 0 || test == 0 || step == 0 {
        return v;
    }
    let mut s = 0;
    while s + train + test <= n {
        v.push(Window {
            train: s..s + train,
            test: s + train..s + train + test,
        });
        s += step;
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_one_over_nineteen_months() {
        let w = rolling(19, 3, 1, 1);
        assert_eq!(w.len(), 16);
        assert_eq!(
            w[0],
            Window {
                train: 0..3,
                test: 3..4
            }
        );
        assert_eq!(
            w[15],
            Window {
                train: 15..18,
                test: 18..19
            }
        );
        // Test periods never overlap their own train periods and tile 3..19.
        let tests: Vec<usize> = w.iter().map(|x| x.test.start).collect();
        assert_eq!(tests, (3..19).collect::<Vec<_>>());
        assert_eq!(rolling(3, 3, 1, 1), Vec::new());
        assert_eq!(rolling(10, 0, 1, 1), Vec::new());
    }
}
