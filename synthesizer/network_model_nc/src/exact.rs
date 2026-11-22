//! The exact projection of a window's (C, B) feasible set.
//!
//! The generated B functions give, for a concrete C, the set of B that
//! explain the window.  Evaluated on a symbolic C (`ds::lin`), their result
//! is exact on a cell of C values; `pieces` refines cells until every cell's
//! result is exact.  This replaces sampling C on a grid, which can miss
//! feasible C values (and so exclude the true link from the belief).

use std::ops::Bound;

use ds::interval::IntervalList;
use ds::lin::{current_cell, split_at, widen, with_cell, Cell, Lin, Wide};
use ds::*;
use num_traits::ToPrimitive;

/// Give up on a window that needs more cells than this (it would mean the
/// generated code is not piecewise linear in C as assumed).
const MAX_CELLS: usize = 100_000;

/// Where the window is feasible: a cell of C and, on it, B's feasible set
/// with bounds linear in C (non-empty everywhere on the cell).
#[derive(Clone, Debug)]
pub struct Piece {
    pub cell: Cell,
    pub b: IntervalList<Lin>,
}

/// Every cell of `[lo, hi]` on which `b_of_c` has a non-empty B set.
/// `b_of_c` returns `None` where C itself is infeasible.
pub fn pieces(b_of_c: impl Fn(Lin) -> Option<IntervalList<Lin>>, lo: Wide, hi: Wide) -> Vec<Piece> {
    assert!(lo <= hi);
    let mut todo = vec![Cell::Point(lo)];
    if lo < hi {
        todo.push(Cell::Open(lo, hi));
        todo.push(Cell::Point(hi));
    }
    let mut ret = vec![];
    let mut evaluated = 0;
    while let Some(cell) = todo.pop() {
        evaluated += 1;
        assert!(evaluated <= MAX_CELLS, "exact::pieces: more than {} cells", MAX_CELLS);
        let (b, splits) = with_cell(cell, || b_of_c(Lin::var()).filter(|b| !b.is_empty()));
        if !splits.is_empty() {
            let Cell::Open(lo, hi) = cell else { unreachable!("a point cell never splits") };
            let mut prev = lo;
            for r in splits {
                todo.push(Cell::Open(prev, r));
                todo.push(Cell::Point(r));
                prev = r;
            }
            todo.push(Cell::Open(prev, hi));
            continue;
        }
        if let Some(b) = b {
            ret.push(Piece { cell, b });
        }
    }
    ret.sort_by(|x, y| lower(x.cell).cmp(&lower(y.cell)).then(is_open(x.cell).cmp(&is_open(y.cell))));
    ret
}

fn lower(cell: Cell) -> Wide {
    match cell {
        Cell::Open(lo, _) => lo,
        Cell::Point(p) => p,
    }
}

fn upper(cell: Cell) -> Wide {
    match cell {
        Cell::Open(_, hi) => hi,
        Cell::Point(p) => p,
    }
}

fn is_open(cell: Cell) -> bool {
    matches!(cell, Cell::Open(..))
}

/// The largest `RealNumRep` at most `x` on the 1/`GRID` grid (exact when
/// `x` is on it): an outer bound for a lower end.
pub fn round_down(x: Wide) -> RealNumRep {
    narrow(x).unwrap_or_else(|| {
        let q = (x * Wide::from_integer(GRID as i128)).floor().to_integer();
        RealNumRep::new(q.to_i64().expect("bound out of range"), GRID)
    })
}

/// The smallest `RealNumRep` at least `x`; see `round_down`.
pub fn round_up(x: Wide) -> RealNumRep {
    narrow(x).unwrap_or_else(|| {
        let q = (x * Wide::from_integer(GRID as i128)).ceil().to_integer();
        RealNumRep::new(q.to_i64().expect("bound out of range"), GRID)
    })
}

/// `x` itself if its denominator is a power of two up to `GRID`.  Bounds
/// flow into cumulative arrivals as rates, and `RealNumRep` is `Ratio<i64>`:
/// mixing denominators such as 3 and 1024 makes sums grow until they overflow (a
/// panic: release builds check overflow).  Dyadic denominators up to `GRID` stay bounded.
fn narrow(x: Wide) -> Option<RealNumRep> {
    let d = *x.denom();
    if d > GRID as i128 || d & (d - 1) != 0 {
        return None;
    }
    Some(RealNumRep::new(x.numer().to_i64()?, d as RealNumInt))
}

/// See `narrow`.
const GRID: RealNumInt = 1024;

/// The closed hull of each run of adjacent pieces, rounded outward: a
/// superset of the feasible C set.
pub fn c_set(pieces: &[Piece]) -> IntervalList<RealNumRep> {
    let mut ret = IntervalList::new(vec![]);
    for p in pieces {
        ret = ret.union(&IntervalList::interval_bounded(
            Bound::Included(round_down(lower(p.cell))),
            Bound::Included(round_up(upper(p.cell))),
        ));
    }
    ret
}

/// Bounds on B over every piece's C within `c`: the infimum and supremum
/// (closures, rounded outward), clipped to `[floor, ceil]`.  `None` if no
/// piece meets `c`.
pub fn b_range(
    pieces: &[Piece],
    c: &IntervalList<RealNumRep>,
    floor: RealNumRep,
    ceil: RealNumRep,
) -> Option<(RealNumRep, RealNumRep)> {
    let mut ret: Option<(Wide, Wide)> = None;
    for p in pieces {
        for iv in c.get_intervals() {
            // The closure of cell ∩ iv: B's bounds are linear in C on the
            // cell, so their extremes over it are at these two ends.
            let lo = std::cmp::max(lower(p.cell), bound_value(iv.start, widen(floor)));
            let hi = std::cmp::min(upper(p.cell), bound_value(iv.end, widen(ceil)));
            if lo > hi {
                continue;
            }
            let (b_lo, b_hi) = b_extremes(p, lo, hi, widen(floor), widen(ceil));
            ret = Some(match ret {
                None => (b_lo, b_hi),
                Some((x, y)) => (std::cmp::min(x, b_lo), std::cmp::max(y, b_hi)),
            });
        }
    }
    ret.map(|(lo, hi)| {
        (
            std::cmp::max(round_down(lo), floor),
            std::cmp::min(round_up(hi), ceil),
        )
    })
}

fn bound_value(b: Bound<RealNumRep>, unbounded: Wide) -> Wide {
    match b {
        Bound::Included(x) | Bound::Excluded(x) => widen(x),
        Bound::Unbounded => unbounded,
    }
}

/// B's smallest lower and largest upper end on the piece, for C in
/// `[lo, hi]`, with unbounded ends at `floor` / `ceil`.
fn b_extremes(p: &Piece, lo: Wide, hi: Wide, floor: Wide, ceil: Wide) -> (Wide, Wide) {
    let ivs = p.b.get_intervals();
    let ends = |b: Bound<Lin>, unbounded: Wide| -> [Wide; 2] {
        match b {
            Bound::Included(x) | Bound::Excluded(x) => [x.at(lo), x.at(hi)],
            Bound::Unbounded => [unbounded; 2],
        }
    };
    let [s0, s1] = ends(ivs.first().unwrap().start, floor);
    let [e0, e1] = ends(ivs.last().unwrap().end, ceil);
    (std::cmp::min(s0, s1), std::cmp::max(e0, e1))
}

/// One `RealNumRep` C inside each piece (where one is representable): a
/// feasible C to evaluate the concrete QE functions at.  Unlike bounds,
/// these need not be dyadic: they are evaluation points, and a single-point
/// piece (a backlogged link serving exactly C) is often not on the grid.
pub fn representatives(pieces: &[Piece]) -> Vec<RealNumRep> {
    pieces
        .iter()
        .filter_map(|p| match p.cell {
            Cell::Point(x) => exact_value(x),
            Cell::Open(lo, hi) => {
                let mid = narrow((lo + hi) / 2)
                    .or_else(|| Some(round_down((lo + hi) / 2)))
                    .filter(|&m| lo < widen(m) && widen(m) < hi)
                    .or_else(|| exact_value((lo + hi) / 2))?;
                (lo < widen(mid) && widen(mid) < hi).then_some(mid)
            }
        })
        .collect()
}

/// `x` exactly, if its denominator is modest: the concrete QE functions
/// evaluated at it compute in `RealNumRep`.
fn exact_value(x: Wide) -> Option<RealNumRep> {
    if *x.denom() > MAX_EVAL_DEN {
        return None;
    }
    Some(RealNumRep::new(x.numer().to_i64()?, x.denom().to_i64()?))
}

/// See `exact_value`.
const MAX_EVAL_DEN: i128 = 4096;

/// The pieces where both `x` and `y` are feasible, with B in both: the
/// (C, B) set that explains both windows, exactly.
pub fn intersect(x: &[Piece], y: &[Piece]) -> Vec<Piece> {
    let (Some(x0), Some(y0)) = (x.first(), y.first()) else { return vec![] };
    let lo = std::cmp::max(lower(x0.cell), lower(y0.cell));
    let hi = std::cmp::min(upper(x.last().unwrap().cell), upper(y.last().unwrap().cell));
    if lo > hi {
        return vec![];
    }
    pieces(
        |_| {
            let cell = current_cell();
            // Evaluate both before returning, so each records its splits.
            let (bx, by) = (covering(x, cell), covering(y, cell));
            Some(bx?.intersection(by?))
        },
        lo,
        hi,
    )
}

/// B on the piece of `ps` that contains `cell`.  If a piece boundary lies
/// strictly inside an open `cell`, it records a split there.
fn covering(ps: &[Piece], cell: Cell) -> Option<&IntervalList<Lin>> {
    let mut found = None;
    for p in ps {
        match (p.cell, cell) {
            (Cell::Point(q), Cell::Point(c)) if q == c => found = Some(&p.b),
            (Cell::Open(lo, hi), Cell::Point(c)) if lo < c && c < hi => found = Some(&p.b),
            (Cell::Open(lo, hi), Cell::Open(a, b)) if lo <= a && b <= hi => found = Some(&p.b),
            (pc, Cell::Open(..)) => {
                split_at(lower(pc));
                split_at(upper(pc));
            }
            _ => {}
        }
    }
    found
}

/// `c` as pieces with every B allowed, for clipping with `intersect`.
pub fn from_c_set(c: &IntervalList<RealNumRep>) -> Vec<Piece> {
    let all_b = IntervalList::new_interval(ds::interval::Interval::new(Bound::Unbounded, Bound::Unbounded));
    let piece = |cell| Piece { cell, b: all_b.clone() };
    let mut ret = vec![];
    for iv in c.get_intervals() {
        let (Bound::Included(lo) | Bound::Excluded(lo)) = iv.start else { panic!("unbounded C set") };
        let (Bound::Included(hi) | Bound::Excluded(hi)) = iv.end else { panic!("unbounded C set") };
        let (lo, hi) = (widen(lo), widen(hi));
        if matches!(iv.start, Bound::Included(_)) {
            ret.push(piece(Cell::Point(lo)));
        }
        if lo < hi {
            ret.push(piece(Cell::Open(lo, hi)));
            if matches!(iv.end, Bound::Included(_)) {
                ret.push(piece(Cell::Point(hi)));
            }
        }
    }
    ret
}

/// The smallest `C + B` over every piece's C within `c` (B's lower end is
/// linear in C on a piece, so it is at a cell end), rounded down; unbounded
/// B ends are clipped to `[floor, ceil]`.  `None` if no piece meets `c`.
pub fn min_c_plus_b(
    pieces: &[Piece],
    c: &IntervalList<RealNumRep>,
    floor: RealNumRep,
    ceil: RealNumRep,
) -> Option<RealNumRep> {
    let mut ret: Option<Wide> = None;
    for p in pieces {
        for iv in c.get_intervals() {
            let lo = std::cmp::max(lower(p.cell), bound_value(iv.start, widen(floor)));
            let hi = std::cmp::min(upper(p.cell), bound_value(iv.end, widen(ceil)));
            if lo > hi {
                continue;
            }
            let b_lo = |x: Wide| match p.b.get_intervals().first().unwrap().start {
                Bound::Included(l) | Bound::Excluded(l) => std::cmp::max(l.at(x), widen(floor)),
                Bound::Unbounded => widen(floor),
            };
            let v = std::cmp::min(lo + b_lo(lo), hi + b_lo(hi));
            ret = Some(ret.map_or(v, |r| std::cmp::min(r, v)));
        }
    }
    ret.map(round_down)
}
