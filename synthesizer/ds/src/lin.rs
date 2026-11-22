//! Exact evaluation of generated QE code with C left symbolic.
//!
//! A `Lin` is `k + c * C` for the one free variable C.  The generated B
//! functions are linear in C, so running them on `Lin` values computes B's
//! bounds as linear functions of C, exactly.  The only non-linear step is a
//! comparison, whose answer can depend on C.  Comparisons are answered for
//! the current *cell* (see `with_cell`): an open interval of C, or a point.
//! If the sign of a difference changes strictly inside an open cell, the
//! answer is not valid for the whole cell; the root is recorded as a split
//! and the caller re-evaluates on the finer cells.  Once an evaluation
//! records no split, every comparison it made had one answer on the whole
//! cell, so its result is exact there.
//!
//! Arithmetic uses `i128` rationals: the roots of linear expressions in the
//! observations have larger denominators than `RealNumRep` can hold.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::ops::{Add, Div, Mul, Neg, Sub};

use num_rational::Ratio;
use num_traits::{Signed, Zero};

use crate::RealNumRep;

pub type Wide = Ratio<i128>;

pub fn widen(x: RealNumRep) -> Wide {
    Wide::new(*x.numer() as i128, *x.denom() as i128)
}

/// `k + c * C`.
#[derive(Clone, Copy, Debug)]
pub struct Lin {
    pub k: Wide,
    pub c: Wide,
}

impl Lin {
    /// The variable C itself.
    pub fn var() -> Self {
        Lin { k: Wide::zero(), c: Wide::from_integer(1) }
    }

    pub fn constant(k: Wide) -> Self {
        Lin { k, c: Wide::zero() }
    }

    pub fn at(&self, c: Wide) -> Wide {
        self.k + self.c * c
    }

    /// Where `self` is zero, if it is not constant.
    pub fn root(&self) -> Option<Wide> {
        (!self.c.is_zero()).then(|| -self.k / self.c)
    }
}

impl From<RealNumRep> for Lin {
    fn from(x: RealNumRep) -> Self {
        Lin::constant(widen(x))
    }
}

/// The C values a comparison is answered for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    /// The open interval `(lo, hi)`, `lo < hi`.
    Open(Wide, Wide),
    Point(Wide),
}

impl Cell {
    /// A C strictly inside the cell (the point itself for `Point`).
    pub fn sample(&self) -> Wide {
        match *self {
            Cell::Open(lo, hi) => (lo + hi) / 2,
            Cell::Point(p) => p,
        }
    }
}

struct CellState {
    cell: Cell,
    splits: Vec<Wide>,
}

thread_local! {
    static CELL: RefCell<Option<CellState>> = const { RefCell::new(None) };
}

/// Run `f` with comparisons of `Lin` answered on `cell`.  Returns `f`'s
/// result and the roots, strictly inside an open cell, at which some
/// comparison's answer changes.  The result is exact for the whole cell iff
/// that list is empty.
pub fn with_cell<T>(cell: Cell, f: impl FnOnce() -> T) -> (T, Vec<Wide>) {
    struct Reset(Option<CellState>);
    impl Drop for Reset {
        fn drop(&mut self) {
            CELL.with(|s| *s.borrow_mut() = self.0.take());
        }
    }
    let previous = CELL.with(|s| s.borrow_mut().replace(CellState { cell, splits: vec![] }));
    let mut reset = Reset(previous);
    let ret = f();
    let state = CELL.with(|s| std::mem::replace(&mut *s.borrow_mut(), reset.0.take()));
    std::mem::forget(reset);
    let mut splits = state.map(|s| s.splits).unwrap_or_default();
    splits.sort();
    splits.dedup();
    (ret, splits)
}

/// The cell comparisons are currently answered for.
pub fn current_cell() -> Cell {
    CELL.with(|s| s.borrow().as_ref().expect("lin::current_cell outside with_cell").cell)
}

/// Mark the current evaluation as not exact on its cell: split at `r`
/// (ignored unless strictly inside the open cell).
pub fn split_at(r: Wide) {
    CELL.with(|s| {
        let mut s = s.borrow_mut();
        let state = s.as_mut().expect("lin::split_at outside with_cell");
        if let Cell::Open(lo, hi) = state.cell {
            if lo < r && r < hi {
                state.splits.push(r);
            }
        }
    })
}

/// The sign of `d` on the current cell.
fn sign_on_cell(d: Lin) -> Ordering {
    CELL.with(|s| {
        let mut s = s.borrow_mut();
        let state = s.as_mut().expect("Lin compared outside lin::with_cell");
        if let (Cell::Open(lo, hi), Some(r)) = (state.cell, d.root()) {
            if lo < r && r < hi {
                state.splits.push(r);
            }
        }
        let v = d.at(state.cell.sample());
        if v.is_positive() {
            Ordering::Greater
        } else if v.is_negative() {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    })
}

impl Ord for Lin {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.k == other.k && self.c == other.c {
            return Ordering::Equal;
        }
        sign_on_cell(*self - *other)
    }
}

/// Equal on the current cell (not as functions of C).
impl PartialEq for Lin {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Lin {}

impl PartialOrd for Lin {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq<RealNumRep> for Lin {
    fn eq(&self, other: &RealNumRep) -> bool {
        self.cmp(&Lin::from(*other)) == Ordering::Equal
    }
}

impl PartialOrd<RealNumRep> for Lin {
    fn partial_cmp(&self, other: &RealNumRep) -> Option<Ordering> {
        Some(self.cmp(&Lin::from(*other)))
    }
}

impl Neg for Lin {
    type Output = Lin;
    fn neg(self) -> Lin {
        Lin { k: -self.k, c: -self.c }
    }
}

impl Add for Lin {
    type Output = Lin;
    fn add(self, o: Lin) -> Lin {
        Lin { k: self.k + o.k, c: self.c + o.c }
    }
}

impl Sub for Lin {
    type Output = Lin;
    fn sub(self, o: Lin) -> Lin {
        Lin { k: self.k - o.k, c: self.c - o.c }
    }
}

impl Add<RealNumRep> for Lin {
    type Output = Lin;
    fn add(self, o: RealNumRep) -> Lin {
        self + Lin::from(o)
    }
}

impl Sub<RealNumRep> for Lin {
    type Output = Lin;
    fn sub(self, o: RealNumRep) -> Lin {
        self - Lin::from(o)
    }
}

impl Add<Lin> for RealNumRep {
    type Output = Lin;
    fn add(self, o: Lin) -> Lin {
        Lin::from(self) + o
    }
}

impl Sub<Lin> for RealNumRep {
    type Output = Lin;
    fn sub(self, o: Lin) -> Lin {
        Lin::from(self) - o
    }
}

/// One factor must be constant: the generated code is linear in C.
impl Mul for Lin {
    type Output = Lin;
    fn mul(self, o: Lin) -> Lin {
        if o.c.is_zero() {
            self * o.k
        } else {
            assert!(self.c.is_zero(), "non-linear product of Lin values");
            o * self.k
        }
    }
}

impl Div for Lin {
    type Output = Lin;
    fn div(self, o: Lin) -> Lin {
        assert!(o.c.is_zero(), "division by a Lin that depends on C");
        self * (Wide::from_integer(1) / o.k)
    }
}

impl Mul<Wide> for Lin {
    type Output = Lin;
    fn mul(self, o: Wide) -> Lin {
        Lin { k: self.k * o, c: self.c * o }
    }
}

impl Mul<RealNumRep> for Lin {
    type Output = Lin;
    fn mul(self, o: RealNumRep) -> Lin {
        self * widen(o)
    }
}

impl Mul<Lin> for RealNumRep {
    type Output = Lin;
    fn mul(self, o: Lin) -> Lin {
        o * widen(self)
    }
}

impl Mul<i32> for Lin {
    type Output = Lin;
    fn mul(self, o: i32) -> Lin {
        self * Wide::from_integer(o as i128)
    }
}

impl Div<RealNumRep> for Lin {
    type Output = Lin;
    fn div(self, o: RealNumRep) -> Lin {
        self * (Wide::from_integer(1) / widen(o))
    }
}

impl Div<i32> for Lin {
    type Output = Lin;
    fn div(self, o: i32) -> Lin {
        self * Wide::new(1, o as i128)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comparison_splits_open_cell_at_root() {
        let c = Lin::var();
        let three = Lin::from(RealNumRep::from_integer(3));
        let (lt, splits) = with_cell(Cell::Open(Wide::from_integer(0), Wide::from_integer(10)), || c < three);
        assert_eq!(splits, vec![Wide::from_integer(3)]);
        assert!(!lt); // sampled at 5
        let (lt, splits) = with_cell(Cell::Open(Wide::from_integer(0), Wide::from_integer(3)), || c < three);
        assert!(splits.is_empty());
        assert!(lt);
        let (eq, splits) = with_cell(Cell::Point(Wide::from_integer(3)), || c == RealNumRep::from_integer(3));
        assert!(splits.is_empty());
        assert!(eq);
    }
}
