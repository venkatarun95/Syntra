// Generated from qe/outputs/qe/cbrdelay_l0_t6 B results with
// transpile(..., c_from_b=True), for testing the C fallback against the
// plain model's exact T=6 C.  Not used outside tests.
pub fn compute_c_from_b_t_6_l_0(
    a: &[RealNumRep],
    l: &[RealNumRep],
    s: &[RealNumRep],
    L0: RealNumRep,
) -> IntervalList<RealNumRep> {
    assert!(l.len() == 0);
    let mut ret = IntervalList::new_interval(Interval::new(Bound::Unbounded, Bound::Unbounded));
    assert!(a[5] >= a[4]);
    assert!(a[4] >= a[3]);
    assert!(a[3] >= a[2]);
    assert!(a[2] >= a[1]);
    assert!(a[1] >= a[0]);
    assert!(s[5] >= s[4]);
    assert!(s[4] >= s[3]);
    assert!(s[3] >= s[2]);
    assert!(s[2] >= s[1]);
    assert!(s[1] >= s[0]);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[2] * 1 / 2 + -s[1] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[4] * 1 / 3 + -s[2] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![
        IntervalList::interval_lower(Bound::Included(a[4] + -a[3] * 1)),
        IntervalList::interval_upper(Bound::Included(s[5] + -s[3] * 1)),
    ]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[0] * 1 / 3 + s[2] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[3] * 1 / 2 + -s[2] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![
        IntervalList::interval_lower(Bound::Included(-a[2] * 1 + a[3])),
        IntervalList::interval_lower(Bound::Included(a[4] + -a[3] * 1)),
        IntervalList::interval_upper(Bound::Included(s[5] * 1 / 2 + -s[2] * 1 / 2)),
    ]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[3] * 1 / 4 + -s[0] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[3] * 1 / 3 + -s[1] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[4] * 1 / 4 + -s[1] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[5] * 1 / 5 + -s[1] * 1 / 5),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[5] * 1 / 6 + -s[0] * 1 / 6),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[5] * 1 / 4 + -s[2] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![
        IntervalList::interval_lower(Bound::Included(a[4] + -a[3] * 1)),
        IntervalList::interval_upper(Bound::Included(s[5] * 1 / 2 + -s[2] * 1 / 2)),
        IntervalList::interval_upper(Bound::Included(a[2] + -a[3] * 1 + s[5] + -s[2] * 1)),
    ]);
    ret = ret.intersection(&tmp);
    if !(a[4] + -a[3] * 1 + -s[5] * 1 + s[3] <= RealNumRep::new(0, 1)) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
            Bound::Included(s[5] + -s[3] * 1),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !(-a[2] * 1 + a[3] + -s[4] * 1 + s[2] <= RealNumRep::new(0, 1)) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
            Bound::Included(s[4] + -s[2] * 1),
        )]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[4] * 1 / 5 + -s[0] * 1 / 5),
    )]);
    ret = ret.intersection(&tmp);
    if !(a[3] + -a[5] * 1 + s[5] + -s[4] * 1 <= RealNumRep::new(0, 1)) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
            Bound::Included(s[4] + -s[2] * 1),
        )]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(RealNumRep::new(5, 1)),
    )]);
    ret = ret.intersection(&tmp);
    assert!(L0 >= RealNumRep::new(0, 1));
    assert!(!(s[5] + -a[0] * 1 + L0 >= RealNumRep::new(0, 1)));
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[4] * 1 / 2 + -s[3] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[5] * 1 / 2 + -s[4] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[5] * 1 / 3 + -s[3] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[0] * 1 / 2 + s[1] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
        Bound::Included(-s[0] * 1 + s[2]),
    )]);
    ret = ret.intersection(&tmp);
    ret
}

pub fn compute_c_from_b_t_6_l_1(
    a: &[RealNumRep],
    l: &[RealNumRep],
    s: &[RealNumRep],
    L0: RealNumRep,
) -> IntervalList<RealNumRep> {
    assert!(l.len() == 1);
    let mut ret = IntervalList::new_interval(Interval::new(Bound::Unbounded, Bound::Unbounded));
    assert!(a[0] + -l[0] * 1 <= s[5]);
    assert!(a[5] >= a[4]);
    assert!(a[4] >= a[3]);
    assert!(a[3] >= a[2]);
    assert!(a[2] >= a[1]);
    assert!(s[5] >= s[4]);
    assert!(s[4] >= s[3]);
    assert!(s[3] >= s[2]);
    assert!(s[2] >= s[1]);
    assert!(s[1] >= s[0]);
    assert!(l[0] >= L0);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[1] * 1 / 2 + -s[0] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
        Bound::Included(-s[2] * 1 / 2 + s[5] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 2 + s[2] * 1 / 2 + -a[0] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[2] * 1 / 2 + -s[1] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[2] * 1 / 3 + -s[0] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[5] * 1 / 5 + -s[1] * 1 / 5),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
        Bound::Included(-s[1] * 1 / 2 + s[4] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[3] * 1 / 3 + -s[1] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[1] * 1 / 4 + s[4] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[2] * 1 / 3 + s[4] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 4 + -a[0] * 1 / 4 + s[4] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[2] * 1 / 4 + s[5] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[0] * 1 / 5 + s[4] * 1 / 5),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 5 + s[5] * 1 / 5 + -a[0] * 1 / 5),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[5] * 1 / 6 + -s[0] * 1 / 6),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
        Bound::Included(s[3] * 1 / 2 + -s[0] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
        Bound::Included(-s[0] * 1 / 3 + s[4] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[2] * 1 / 2 + s[3] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 3 + -a[0] * 1 / 3 + s[3] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[3] * 1 / 4 + -s[0] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
        Bound::Included(s[5] + -s[3] * 1),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
        Bound::Included(s[5] * 1 / 4 + -s[0] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
        Bound::Included(s[5] * 1 / 3 + -s[1] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(RealNumRep::new(5, 1)),
    )]);
    ret = ret.intersection(&tmp);
    assert!(L0 >= RealNumRep::new(0, 1));
    assert!(!(l[0] + -a[1] * 1 + s[5] >= RealNumRep::new(0, 1)));
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
        Bound::Included(-s[2] * 1 + s[4]),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
        Bound::Included(s[3] + -s[1] * 1),
    )]);
    ret = ret.intersection(&tmp);
    assert!(l[0] + -a[0] * 1 + s[0] <= RealNumRep::new(0, 1));
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[3] * 1 / 2 + s[4] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[5] * 1 / 2 + -s[4] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
        Bound::Included(s[2] + -s[0] * 1),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] + -a[0] * 1 + s[1]),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[5] * 1 / 3 + -s[3] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    ret
}

pub fn compute_c_from_b_t_6_l_2(
    a: &[RealNumRep],
    l: &[RealNumRep],
    s: &[RealNumRep],
    L0: RealNumRep,
) -> IntervalList<RealNumRep> {
    assert!(l.len() == 2);
    let mut ret = IntervalList::new_interval(Interval::new(Bound::Unbounded, Bound::Unbounded));
    assert!(a[1] + -l[1] * 1 <= s[5]);
    assert!(a[5] >= a[4]);
    assert!(a[4] >= a[3]);
    assert!(a[3] >= a[2]);
    assert!(a[1] >= a[0]);
    assert!(s[4] >= s[3]);
    assert!(s[3] >= s[2]);
    assert!(s[2] >= s[1]);
    assert!(s[1] >= s[0]);
    assert!(l[0] >= L0);
    assert!(s[5] >= s[4]);
    assert!(!(a[2] + -l[1] * 1 <= s[5]));
    assert!((l[1] == l[0]) || (!(l[1] <= l[0])));
    if !(l[1] == l[0]) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Excluded(-a[1] * 1 + l[1] + s[5]),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !(-l[0] * 1 + -s[2] * 1 + a[1] <= RealNumRep::new(0, 1)) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
            Bound::Included(s[2] + -s[0] * 1),
        )]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 5 + s[5] * 1 / 5 + -a[0] * 1 / 5),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 2 + s[3] * 1 / 2 + -a[1] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 3 + s[4] * 1 / 3 + -a[1] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] + s[2] + -a[1] * 1),
    )]);
    ret = ret.intersection(&tmp);
    assert!(l[0] + -a[1] * 1 + s[1] <= RealNumRep::new(0, 1));
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[2] * 1 / 4 + s[5] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[5] * 1 / 6 + -s[0] * 1 / 6),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[1] * 1 / 5 + s[5] * 1 / 5),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 4 + -a[1] * 1 / 4 + s[5] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![
        IntervalList::interval_upper(Bound::Included(s[3] * 1 / 2 + -s[0] * 1 / 2)),
        IntervalList::interval_lower(Bound::Included(-l[0] * 1 + a[1] + -s[0] * 1)),
    ]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![
        IntervalList::interval_upper(Bound::Included(l[0] + s[3] + -a[1] * 1)),
        IntervalList::interval_upper(Bound::Excluded(-l[0] * 1 + a[1] + -s[0] * 1)),
    ]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![
        IntervalList::interval_upper(Bound::Included(s[4] * 1 / 3 + -s[0] * 1 / 3)),
        IntervalList::interval_upper(Bound::Included(l[0] * 1 / 2 + s[4] * 1 / 2 + -a[1] * 1 / 2)),
    ]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![
        IntervalList::interval_upper(Bound::Included(s[5] * 1 / 4 + -s[0] * 1 / 4)),
        IntervalList::interval_upper(Bound::Included(l[0] * 1 / 3 + -a[1] * 1 / 3 + s[5] * 1 / 3)),
    ]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(RealNumRep::new(5, 1)),
    )]);
    ret = ret.intersection(&tmp);
    assert!(L0 >= RealNumRep::new(0, 1));
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
        Bound::Included(s[4] * 1 / 2 + -s[1] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
        Bound::Included(s[3] + -s[1] * 1),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[3] * 1 / 2 + s[4] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[4] * 1 / 3 + -s[2] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[4] * 1 / 2 + s[5] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
        Bound::Included(-s[2] * 1 / 2 + s[5] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
        Bound::Included(s[4] + -s[2] * 1),
    )]);
    ret = ret.intersection(&tmp);
    assert!(l[0] + s[0] + -a[0] * 1 <= RealNumRep::new(0, 1));
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[3] * 1 / 3 + s[5] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
        Bound::Included(-s[3] * 1 + s[5]),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[3] * 1 / 2 + -s[2] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
        Bound::Included(-s[1] * 1 / 3 + s[5] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] + s[1] + -a[0] * 1),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[1] * 1 / 2 + -s[0] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[4] * 1 / 5 + -s[0] * 1 / 5),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[2] * 1 / 2 + -s[1] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 2 + s[2] * 1 / 2 + -a[0] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[2] * 1 / 3 + -s[0] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[3] * 1 / 3 + -s[1] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[3] * 1 / 4 + -s[0] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 3 + s[3] * 1 / 3 + -a[0] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[4] * 1 / 4 + -s[1] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 4 + s[4] * 1 / 4 + -a[0] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    ret
}

pub fn compute_c_from_b_t_6_l_3(
    a: &[RealNumRep],
    l: &[RealNumRep],
    s: &[RealNumRep],
    L0: RealNumRep,
) -> IntervalList<RealNumRep> {
    assert!(l.len() == 3);
    let mut ret = IntervalList::new_interval(Interval::new(Bound::Unbounded, Bound::Unbounded));
    assert!(a[2] + -l[2] * 1 <= s[5]);
    assert!(a[5] >= a[4]);
    assert!(a[4] >= a[3]);
    assert!(a[2] >= a[1]);
    assert!(a[1] >= a[0]);
    assert!(s[5] >= s[4]);
    assert!(s[4] >= s[3]);
    assert!(s[3] >= s[2]);
    assert!(s[2] >= s[1]);
    assert!(s[1] >= s[0]);
    assert!(l[0] >= L0);
    assert!((l[1] == l[0]) || (!(l[1] <= l[0])));
    assert!(!(a[3] + -l[2] * 1 <= s[5]));
    assert!((l[2] == l[1]) || (!(l[2] <= l[1])));
    if !((l[1] == l[0]) || (l[2] == l[1])) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_point(
            Bound::Included(a[2] + -l[2] * 1 + -a[1] * 1 + l[1]),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !(l[2] == l[1]) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Excluded(-a[2] * 1 + l[2] + s[5]),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !((l[2] == l[1]) || (!(l[1] <= l[0]))) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
            Bound::Included(-a[1] * 1 + l[0] + -l[2] * 1 + a[2]),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !(l[1] == l[0]) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Excluded(-a[1] * 1 / 2 + l[1] * 1 / 2 + s[5] * 1 / 2),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !((l[1] == l[0]) || (!(l[1] == l[2]))) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Included(a[2] + -a[1] * 1),
        )]);
        ret = ret.intersection(&tmp);
    }
    assert!(-a[1] * 1 + l[0] + s[1] <= RealNumRep::new(0, 1));
    if !(-l[0] * 1 + a[2] + -s[3] * 1 <= RealNumRep::new(0, 1)) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
            Bound::Included(-s[1] * 1 + s[3]),
        )]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 2 + s[2] * 1 / 2 + -a[0] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[2] * 1 / 2 + -s[1] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 3 + -a[1] * 1 / 3 + s[4] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[2] * 1 / 4 + s[5] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 4 + -a[1] * 1 / 4 + s[5] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] + -a[2] * 1 + s[3]),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[1] * 1 / 2 + -s[0] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 2 + -a[1] * 1 / 2 + s[3] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![
        IntervalList::interval_upper(Bound::Included(-s[0] * 1 / 3 + s[4] * 1 / 3)),
        IntervalList::interval_upper(Bound::Included(l[0] * 1 / 2 + -a[1] * 1 / 2 + s[4] * 1 / 2)),
        IntervalList::interval_lower(Bound::Included(
            -l[0] * 1 / 2 + a[2] * 1 / 2 + -s[0] * 1 / 2,
        )),
    ]);
    ret = ret.intersection(&tmp);
    if !(a[1] + -l[0] * 1 + -s[2] * 1 <= RealNumRep::new(0, 1)) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
            Bound::Included(s[2] + -s[0] * 1),
        )]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![
        IntervalList::interval_lower(Bound::Included(a[2] + -a[1] * 1)),
        IntervalList::interval_upper(Bound::Included(-s[0] * 1 / 3 + s[4] * 1 / 3)),
        IntervalList::interval_upper(Bound::Included(l[0] * 1 / 2 + -a[1] * 1 / 2 + s[4] * 1 / 2)),
    ]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] + -a[1] * 1 + s[2]),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[1] * 1 / 4 + s[4] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![
        IntervalList::interval_lower(Bound::Included(-l[0] * 1 + -s[1] * 1 + a[2])),
        IntervalList::interval_upper(Bound::Included(-s[1] * 1 / 2 + s[4] * 1 / 2)),
    ]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![
        IntervalList::interval_upper(Bound::Included(l[0] + -a[2] * 1 + s[4])),
        IntervalList::interval_upper(Bound::Excluded(-l[0] * 1 + -s[1] * 1 + a[2])),
    ]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 3 + -a[2] * 1 / 3 + s[5] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    if !(-l[0] * 1 + a[2] + -s[3] * 1 <= RealNumRep::new(0, 1)) {
        let tmp = IntervalList::from_interval_lists(vec![
            IntervalList::interval_upper(Bound::Included(-s[0] * 1 / 2 + s[3] * 1 / 2)),
            IntervalList::interval_upper(Bound::Included(l[0] + -a[1] * 1 + s[3])),
        ]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 5 + s[5] * 1 / 5 + -a[0] * 1 / 5),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[0] * 1 / 6 + s[5] * 1 / 6),
    )]);
    ret = ret.intersection(&tmp);
    assert!(l[0] + s[2] + -a[2] * 1 <= RealNumRep::new(0, 1));
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[2] * 1 / 3 + -s[0] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 3 + s[3] * 1 / 3 + -a[0] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 2 + -a[2] * 1 / 2 + s[4] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[0] * 1 / 4 + s[3] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[2] * 1 / 2 + s[3] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![
        IntervalList::interval_lower(Bound::Included(-l[0] * 1 + -s[1] * 1 + a[2])),
        IntervalList::interval_upper(Bound::Included(-s[1] * 1 / 3 + s[5] * 1 / 3)),
    ]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![
        IntervalList::interval_upper(Bound::Included(l[0] * 1 / 2 + -a[2] * 1 / 2 + s[5] * 1 / 2)),
        IntervalList::interval_upper(Bound::Included(-s[0] * 1 / 4 + s[5] * 1 / 4)),
        IntervalList::interval_upper(Bound::Included(l[0] * 1 / 3 + -a[1] * 1 / 3 + s[5] * 1 / 3)),
    ]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[0] * 1 / 5 + s[4] * 1 / 5),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![
        IntervalList::interval_upper(Bound::Included(l[0] * 1 / 2 + -a[2] * 1 / 2 + s[5] * 1 / 2)),
        IntervalList::interval_upper(Bound::Excluded(-l[0] * 1 + -s[1] * 1 + a[2])),
    ]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[1] * 1 / 5 + s[5] * 1 / 5),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![
        IntervalList::interval_upper(Bound::Included(-a[1] * 1 + a[2])),
        IntervalList::interval_upper(Bound::Included(l[0] + -a[2] * 1 + s[4])),
        IntervalList::interval_upper(Bound::Excluded(
            -l[0] * 1 / 2 + a[2] * 1 / 2 + -s[0] * 1 / 2,
        )),
    ]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![
        IntervalList::interval_upper(Bound::Included(l[0] + -a[2] * 1 + s[4])),
        IntervalList::interval_upper(Bound::Excluded(a[2] + -a[1] * 1)),
        IntervalList::interval_upper(Bound::Excluded(
            -l[0] * 1 / 2 + a[2] * 1 / 2 + -s[0] * 1 / 2,
        )),
    ]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(RealNumRep::new(5, 1)),
    )]);
    ret = ret.intersection(&tmp);
    assert!(L0 >= RealNumRep::new(0, 1));
    assert!(l[0] + s[0] + -a[0] * 1 <= RealNumRep::new(0, 1));
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] + s[1] + -a[0] * 1),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
        Bound::Included(-s[2] * 1 + s[4]),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
        Bound::Included(-s[2] * 1 / 2 + s[5] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
        Bound::Included(s[5] + -s[3] * 1),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[5] * 1 / 2 + -s[4] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[5] * 1 / 3 + -s[3] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[4] * 1 / 2 + -s[3] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[2] * 1 / 3 + s[4] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 4 + s[4] * 1 / 4 + -a[0] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[1] * 1 / 3 + s[3] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    ret
}

pub fn compute_c_from_b_t_6_l_4(
    a: &[RealNumRep],
    l: &[RealNumRep],
    s: &[RealNumRep],
    L0: RealNumRep,
) -> IntervalList<RealNumRep> {
    assert!(l.len() == 4);
    let mut ret = IntervalList::new_interval(Interval::new(Bound::Unbounded, Bound::Unbounded));
    assert!(a[3] + -l[3] * 1 <= s[5]);
    assert!(a[5] >= a[4]);
    assert!(a[3] >= a[2]);
    assert!(a[2] >= a[1]);
    assert!(a[1] >= a[0]);
    assert!(s[5] >= s[4]);
    assert!(s[4] >= s[3]);
    assert!(s[3] >= s[2]);
    assert!(s[2] >= s[1]);
    assert!(s[1] >= s[0]);
    assert!(l[0] >= L0);
    assert!((l[1] == l[0]) || (!(l[1] <= l[0])));
    assert!(!(a[4] + -l[3] * 1 <= s[5]));
    if !(l[3] == l[2]) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Excluded(-a[3] * 1 + l[3] + s[5]),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !((l[1] == l[0]) || (l[2] == l[1])) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_point(
            Bound::Included(-l[2] * 1 + a[2] + -a[1] * 1 + l[1]),
        )]);
        ret = ret.intersection(&tmp);
    }
    assert!((!(l[2] <= l[0])) || (l[2] == l[0]));
    if !((l[2] == l[1]) || (l[3] == l[2])) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_point(
            Bound::Included(a[3] + -l[3] * 1 + -a[2] * 1 + l[2]),
        )]);
        ret = ret.intersection(&tmp);
    }
    assert!((l[3] == l[2]) || (!(l[3] <= l[2])));
    if !(l[2] == l[1]) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
            Bound::Excluded(a[2] + -a[1] * 1),
        )]);
        ret = ret.intersection(&tmp);
    }
    assert!(-a[1] * 1 + l[0] + s[1] <= RealNumRep::new(0, 1));
    if !((!(l[0] == l[1])) || (l[2] == l[0])) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
            Bound::Included(-a[1] * 1 + l[0] + -l[2] * 1 + a[2]),
        )]);
        ret = ret.intersection(&tmp);
    }
    assert!(s[2] <= a[2] + -l[1] * 1);
    if !((!(l[1] <= l[0])) || (!(l[1] == l[2])) || (l[3] == l[1])) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
            Bound::Included(-a[2] * 1 + l[0] + -l[3] * 1 + a[3]),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !((l[1] == l[0]) || (!(l[1] == l[2]))) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Included(a[2] + -a[1] * 1),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !((l[2] == l[1]) || (!(l[3] <= l[2]))) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Included(a[3] + -a[2] * 1),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !(l[2] == l[1]) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Excluded(-a[2] * 1 / 2 + l[2] * 1 / 2 + s[5] * 1 / 2),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !((l[1] == l[0]) || (!(l[2] == l[3]))) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Included(a[3] * 1 / 2 + -l[2] * 1 / 2 + -a[1] * 1 / 2 + l[1] * 1 / 2),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !(l[1] == l[0]) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Excluded(-a[1] * 1 / 3 + l[1] * 1 / 3 + s[5] * 1 / 3),
        )]);
        ret = ret.intersection(&tmp);
    }
    assert!(s[3] <= a[3] + -l[2] * 1);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[4] + -a[3] * 1 + l[2]),
    )]);
    ret = ret.intersection(&tmp);
    if !(a[1] + -l[0] * 1 + -s[2] * 1 <= RealNumRep::new(0, 1)) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
            Bound::Included(-s[0] * 1 + s[2]),
        )]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![
        IntervalList::interval_upper(Bound::Included(-a[3] * 1 + l[3] + s[5])),
        IntervalList::interval_upper(Bound::Included(s[5] * 1 / 2 + -s[2] * 1 / 2)),
    ]);
    ret = ret.intersection(&tmp);
    if !(s[3] >= a[2] + -l[1] * 1) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
            Bound::Included(-s[1] * 1 + s[3]),
        )]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[0] * 1 / 4 + s[3] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-a[1] * 1 + l[0] + s[2]),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![
        IntervalList::interval_upper(Bound::Included(-a[3] * 1 + l[3] + s[5])),
        IntervalList::interval_upper(Bound::Included(-a[2] * 1 / 2 + l[2] * 1 / 2 + s[5] * 1 / 2)),
        IntervalList::interval_lower(Bound::Included(a[1] + -l[0] * 1 + -s[0] * 1)),
        IntervalList::interval_upper(Bound::Included(-s[0] * 1 / 4 + s[5] * 1 / 4)),
    ]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![
        IntervalList::interval_lower(Bound::Included(a[2] + -a[1] * 1)),
        IntervalList::interval_upper(Bound::Included(-a[3] * 1 + l[3] + s[5])),
        IntervalList::interval_upper(Bound::Included(-a[1] * 1 / 3 + l[0] * 1 / 3 + s[5] * 1 / 3)),
        IntervalList::interval_upper(Bound::Excluded(a[1] + -l[0] * 1 + -s[0] * 1)),
    ]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![
        IntervalList::interval_upper(Bound::Included(-a[2] * 1 / 2 + l[2] * 1 / 2 + s[5] * 1 / 2)),
        IntervalList::interval_upper(Bound::Included(-a[3] * 1 + l[3] + s[5])),
        IntervalList::interval_upper(Bound::Included(-s[1] * 1 / 3 + s[5] * 1 / 3)),
    ]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[1] * 1 / 2 + s[2] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[1] * 1 / 3 + s[3] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[3] + -a[2] * 1 + l[1]),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-a[1] * 1 / 2 + s[3] * 1 / 2 + l[0] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-a[1] * 1 / 3 + l[0] * 1 / 3 + s[4] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 2 + s[2] * 1 / 2 + -a[0] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[0] * 1 / 3 + s[2] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 4 + s[4] * 1 / 4 + -a[0] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[0] * 1 / 5 + s[4] * 1 / 5),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 3 + s[3] * 1 / 3 + -a[0] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[3] * 1 / 2 + -s[2] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    if !((s[4] >= a[3] + -l[1] * 1) || (!(l[3] <= l[1]))) {
        let tmp = IntervalList::from_interval_lists(vec![
            IntervalList::interval_upper(Bound::Included(s[4] + -a[2] * 1 + l[1])),
            IntervalList::interval_upper(Bound::Included(-s[0] * 1 / 3 + s[4] * 1 / 3)),
            IntervalList::interval_upper(Bound::Included(
                -a[1] * 1 / 2 + l[0] * 1 / 2 + s[4] * 1 / 2,
            )),
        ]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[4] * 1 / 2 + -a[2] * 1 / 2 + l[1] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![
        IntervalList::interval_upper(Bound::Included(-a[3] * 1 + l[3] + s[5])),
        IntervalList::interval_upper(Bound::Excluded(a[2] + -a[1] * 1)),
        IntervalList::interval_upper(Bound::Included(-a[2] * 1 / 2 + l[0] * 1 / 2 + s[5] * 1 / 2)),
        IntervalList::interval_upper(Bound::Excluded(a[1] + -l[0] * 1 + -s[0] * 1)),
    ]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[1] * 1 / 4 + s[4] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[4] * 1 / 3 + -s[2] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    if !((s[4] >= a[3] + -l[1] * 1) || (!(l[3] <= l[1]))) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
            Bound::Included(s[4] + -s[2] * 1),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !(s[3] >= a[2] + -l[1] * 1) {
        let tmp = IntervalList::from_interval_lists(vec![
            IntervalList::interval_upper(Bound::Included(-a[1] * 1 + l[0] + s[3])),
            IntervalList::interval_upper(Bound::Included(-s[0] * 1 / 2 + s[3] * 1 / 2)),
        ]);
        ret = ret.intersection(&tmp);
    }
    if !((s[4] >= a[3] + -l[1] * 1) || (!(l[3] <= l[1]))) {
        let tmp = IntervalList::from_interval_lists(vec![
            IntervalList::interval_upper(Bound::Included(-s[1] * 1 / 2 + s[4] * 1 / 2)),
            IntervalList::interval_upper(Bound::Included(s[4] + -a[2] * 1 + l[1])),
        ]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-a[1] * 1 / 4 + l[0] * 1 / 4 + s[5] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[5] * 1 / 2 + -a[3] * 1 / 2 + l[1] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 5 + s[5] * 1 / 5 + -a[0] * 1 / 5),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[1] * 1 / 5 + s[5] * 1 / 5),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[5] * 1 / 3 + -a[2] * 1 / 3 + l[1] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[3] * 1 / 3 + s[5] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[0] * 1 / 6 + s[5] * 1 / 6),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[5] * 1 / 4 + -s[2] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(RealNumRep::new(5, 1)),
    )]);
    ret = ret.intersection(&tmp);
    assert!(L0 >= RealNumRep::new(0, 1));
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] + s[1] + -a[0] * 1),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[1] * 1 / 2 + -s[0] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    assert!(l[0] + s[0] + -a[0] * 1 <= RealNumRep::new(0, 1));
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
        Bound::Included(-s[3] * 1 + s[5]),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[3] * 1 / 2 + s[4] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[5] * 1 / 2 + -s[4] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    ret
}

pub fn compute_c_from_b_t_6_l_5(
    a: &[RealNumRep],
    l: &[RealNumRep],
    s: &[RealNumRep],
    L0: RealNumRep,
) -> IntervalList<RealNumRep> {
    assert!(l.len() == 5);
    let mut ret = IntervalList::new_interval(Interval::new(Bound::Unbounded, Bound::Unbounded));
    assert!(a[4] + -l[4] * 1 <= s[5]);
    assert!(a[4] >= a[3]);
    assert!(a[3] >= a[2]);
    assert!(a[2] >= a[1]);
    assert!(a[1] >= a[0]);
    assert!(s[4] >= s[3]);
    assert!(s[3] >= s[2]);
    assert!(s[2] >= s[1]);
    assert!(s[1] >= s[0]);
    assert!(l[0] >= L0);
    assert!(!(a[5] + -l[4] * 1 <= s[5]));
    assert!((l[4] == l[3]) || (!(l[4] <= l[3])));
    assert!((l[2] == l[1]) || (!(l[2] <= l[1])));
    if !((l[1] == l[0]) || (l[2] == l[1])) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_point(
            Bound::Included(a[2] + -l[2] * 1 + -a[1] * 1 + l[1]),
        )]);
        ret = ret.intersection(&tmp);
    }
    assert!((l[3] == l[2]) || (!(l[3] <= l[2])));
    assert!((l[1] == l[0]) || (!(l[1] <= l[0])));
    if !((l[3] == l[2]) || (l[4] == l[3])) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_point(
            Bound::Included(a[4] + -l[4] * 1 + -a[3] * 1 + l[3]),
        )]);
        ret = ret.intersection(&tmp);
    }
    assert!(s[2] <= a[2] + -l[1] * 1);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[3] + -a[2] * 1 + l[1]),
    )]);
    ret = ret.intersection(&tmp);
    if !((l[3] == l[2]) || (!(l[3] == l[4]))) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Included(a[4] + -a[3] * 1),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !((!(l[0] == l[1])) || (l[2] == l[0])) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
            Bound::Included(-a[1] * 1 + l[0] + -l[2] * 1 + a[2]),
        )]);
        ret = ret.intersection(&tmp);
    }
    assert!(s[3] <= a[3] + -l[2] * 1);
    if !((l[2] == l[1]) || (l[3] == l[2])) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_point(
            Bound::Included(a[3] + -l[3] * 1 + -a[2] * 1 + l[2]),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !((l[1] == l[0]) || (!(l[1] == l[2]))) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Included(a[2] + -a[1] * 1),
        )]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[4] + -a[3] * 1 + l[1]),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[5] + -a[4] * 1 + l[3]),
    )]);
    ret = ret.intersection(&tmp);
    if !((l[2] == l[1]) || (!(l[3] == l[4]))) {
        let tmp = IntervalList::from_interval_lists(vec![
            IntervalList::interval_lower(Bound::Included(a[4] + -a[3] * 1)),
            IntervalList::interval_lower(Bound::Included(a[4] * 1 / 2 + -a[2] * 1 / 2)),
        ]);
        ret = ret.intersection(&tmp);
    }
    assert!(s[4] <= a[4] + -l[3] * 1);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[4] + -a[3] * 1 + l[2]),
    )]);
    ret = ret.intersection(&tmp);
    if !((l[1] == l[0]) || (!(l[2] <= l[1])) || (!(l[3] == l[4]))) {
        let tmp = IntervalList::from_interval_lists(vec![
            IntervalList::interval_lower(Bound::Included(a[4] + -a[3] * 1)),
            IntervalList::interval_lower(Bound::Included(a[4] * 1 / 3 + -a[1] * 1 / 3)),
        ]);
        ret = ret.intersection(&tmp);
    }
    if !((l[1] == l[0]) || (!(l[2] <= l[1])) || (!(l[2] == l[3]))) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Included(a[3] * 1 / 2 + -a[1] * 1 / 2),
        )]);
        ret = ret.intersection(&tmp);
    }
    assert!(-a[1] * 1 + l[0] + s[1] <= RealNumRep::new(0, 1));
    if !((!(l[1] <= l[0])) || (!(l[1] == l[2])) || (l[3] == l[1])) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
            Bound::Included(-a[2] * 1 + l[0] + -l[3] * 1 + a[3]),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !((l[2] == l[1]) || (!(l[2] == l[3]))) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Included(a[3] + -a[2] * 1),
        )]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] + -a[0] * 1 + s[1]),
    )]);
    ret = ret.intersection(&tmp);
    if !(a[1] + -l[0] * 1 + -s[2] * 1 <= RealNumRep::new(0, 1)) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
            Bound::Included(s[2] + -s[0] * 1),
        )]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[1] * 1 / 2 + -s[0] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    if !((!(l[1] <= l[0])) || (!(l[2] <= l[1])) || (!(l[2] == l[3])) || (l[4] == l[2])) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
            Bound::Included(-a[3] * 1 + l[0] + -l[4] * 1 + a[4]),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !(s[3] >= a[2] + -l[1] * 1) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
            Bound::Included(s[3] + -s[1] * 1),
        )]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 2 + -a[0] * 1 / 2 + s[2] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    if !(s[4] >= a[3] + -l[2] * 1) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
            Bound::Included(s[4] + -s[2] * 1),
        )]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] + -a[1] * 1 + s[2]),
    )]);
    ret = ret.intersection(&tmp);
    if !(s[3] >= a[2] + -l[1] * 1) {
        let tmp = IntervalList::from_interval_lists(vec![
            IntervalList::interval_upper(Bound::Included(s[3] * 1 / 2 + -s[0] * 1 / 2)),
            IntervalList::interval_upper(Bound::Included(l[0] + s[3] + -a[1] * 1)),
        ]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[2] * 1 / 3 + -s[0] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[2] * 1 / 2 + -s[1] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[5] * 1 / 2 + -a[3] * 1 / 2 + l[2] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[4] * 1 / 2 + -s[3] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[5] * 1 / 3 + -s[3] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[5] * 1 / 4 + -s[2] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[3] * 1 / 2 + -s[2] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[3] * 1 / 4 + -s[0] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    if !(s[4] >= a[3] + -l[2] * 1) {
        let tmp = IntervalList::from_interval_lists(vec![
            IntervalList::interval_upper(Bound::Included(s[4] + -a[2] * 1 + l[1])),
            IntervalList::interval_lower(Bound::Included(a[1] + -l[0] * 1 + -s[0] * 1)),
            IntervalList::interval_upper(Bound::Included(s[4] * 1 / 3 + -s[0] * 1 / 3)),
        ]);
        ret = ret.intersection(&tmp);
    }
    if !(s[4] >= a[3] + -l[2] * 1) {
        let tmp = IntervalList::from_interval_lists(vec![
            IntervalList::interval_upper(Bound::Included(s[4] + -a[2] * 1 + l[1])),
            IntervalList::interval_upper(Bound::Included(s[4] * 1 / 2 + -s[1] * 1 / 2)),
        ]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 3 + -a[0] * 1 / 3 + s[3] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[5] * 1 / 6 + -s[0] * 1 / 6),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[5] * 1 / 3 + -a[2] * 1 / 3 + l[1] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    if !(s[4] >= a[3] + -l[2] * 1) {
        let tmp = IntervalList::from_interval_lists(vec![
            IntervalList::interval_lower(Bound::Included(a[2] + -a[1] * 1)),
            IntervalList::interval_upper(Bound::Excluded(a[1] + -l[0] * 1 + -s[0] * 1)),
            IntervalList::interval_upper(Bound::Included(
                -a[1] * 1 / 2 + l[0] * 1 / 2 + s[4] * 1 / 2,
            )),
        ]);
        ret = ret.intersection(&tmp);
    }
    if !(s[4] >= a[3] + -l[0] * 1) {
        let tmp = IntervalList::from_interval_lists(vec![
            IntervalList::interval_upper(Bound::Included(s[4] + -a[2] * 1 + l[0])),
            IntervalList::interval_upper(Bound::Excluded(a[2] + -a[1] * 1)),
            IntervalList::interval_upper(Bound::Excluded(a[1] + -l[0] * 1 + -s[0] * 1)),
        ]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[4] * 1 / 2 + -a[2] * 1 / 2 + l[1] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[3] * 1 / 3 + -s[1] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[4] * 1 / 4 + -s[1] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[4] * 1 / 5 + -s[0] * 1 / 5),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 3 + -a[1] * 1 / 3 + s[4] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[4] * 1 / 3 + -s[2] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 5 + -a[0] * 1 / 5 + s[5] * 1 / 5),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 4 + -a[1] * 1 / 4 + s[5] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-a[1] * 1 / 2 + l[0] * 1 / 2 + s[3] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 4 + -a[0] * 1 / 4 + s[4] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[5] * 1 / 5 + -s[1] * 1 / 5),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(RealNumRep::new(5, 1)),
    )]);
    ret = ret.intersection(&tmp);
    assert!(L0 >= RealNumRep::new(0, 1));
    assert!(l[0] + -a[0] * 1 + s[0] <= RealNumRep::new(0, 1));
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[4] * 1 / 2 + s[5] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    ret
}

pub fn compute_c_from_b_t_6_l_6(
    a: &[RealNumRep],
    l: &[RealNumRep],
    s: &[RealNumRep],
    L0: RealNumRep,
) -> IntervalList<RealNumRep> {
    assert!(l.len() == 6);
    let mut ret = IntervalList::new_interval(Interval::new(Bound::Unbounded, Bound::Unbounded));
    assert!(a[5] + -l[4] * 1 <= s[5]);
    assert!(a[5] >= a[4]);
    assert!(a[4] >= a[3]);
    assert!(a[3] >= a[2]);
    assert!(a[2] >= a[1]);
    assert!(a[1] >= a[0]);
    assert!(s[4] >= s[3]);
    assert!(s[3] >= s[2]);
    assert!(s[2] >= s[1]);
    assert!(s[1] >= s[0]);
    assert!(l[0] >= L0);
    assert!((l[1] == l[0]) || (!(l[1] <= l[0])));
    if !((l[1] == l[0]) || (l[2] == l[1])) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_point(
            Bound::Included(a[2] + -l[2] * 1 + -a[1] * 1 + l[1]),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !((l[2] == l[1]) || (l[3] == l[2])) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_point(
            Bound::Included(a[3] + -l[3] * 1 + -a[2] * 1 + l[2]),
        )]);
        ret = ret.intersection(&tmp);
    }
    assert!((l[3] == l[2]) || (!(l[3] <= l[2])));
    if !((l[3] == l[2]) || (!(l[4] <= l[3]))) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Included(a[4] + -a[3] * 1),
        )]);
        ret = ret.intersection(&tmp);
    }
    assert!((l[4] == l[3]) || (!(l[4] <= l[3])));
    assert!((l[2] == l[1]) || (!(l[2] <= l[1])));
    assert!(s[4] <= a[4] + -l[3] * 1);
    assert!(s[3] <= a[3] + -l[2] * 1);
    assert!(-a[1] * 1 + l[0] + s[1] <= RealNumRep::new(0, 1));
    assert!(s[5] <= a[5] + -l[4] * 1);
    if !((l[4] == l[3]) || (l[3] <= l[2])) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_point(
            Bound::Included(a[4] + -l[4] * 1 + -a[3] * 1 + l[3]),
        )]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(a[5] + -a[4] * 1),
    )]);
    ret = ret.intersection(&tmp);
    if !((!(l[0] == l[1])) || (l[2] == l[0])) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
            Bound::Included(-a[1] * 1 + l[0] + -l[2] * 1 + a[2]),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !((l[2] == l[1]) || (!(l[2] == l[3]))) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Included(a[3] + -a[2] * 1),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !((l[2] == l[1]) || (!(l[3] <= l[2])) || (!(l[4] <= l[3]))) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Included(a[4] * 1 / 2 + -a[2] * 1 / 2),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !((l[1] == l[0]) || (!(l[1] == l[2]))) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Included(a[2] + -a[1] * 1),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !((l[1] == l[0]) || (!(l[2] <= l[1])) || (!(l[3] <= l[2])) || (!(l[3] == l[4]))) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Included(a[4] * 1 / 3 + -a[1] * 1 / 3),
        )]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[4] + -a[3] * 1 + l[2]),
    )]);
    ret = ret.intersection(&tmp);
    if !((l[1] == l[0]) || (!(l[2] == l[3]))) {
        let tmp = IntervalList::from_interval_lists(vec![
            IntervalList::interval_lower(Bound::Included(a[3] + -a[2] * 1)),
            IntervalList::interval_lower(Bound::Included(a[3] * 1 / 2 + -a[1] * 1 / 2)),
        ]);
        ret = ret.intersection(&tmp);
    }
    assert!(s[2] <= a[2] + -l[1] * 1);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] + s[1] + -a[0] * 1),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[1] * 1 / 2 + -s[0] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    if !(a[1] + -l[0] * 1 + -s[2] * 1 <= RealNumRep::new(0, 1)) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
            Bound::Included(s[2] + -s[0] * 1),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !((!(l[1] <= l[0])) || (!(l[1] == l[2])) || (l[3] == l[1])) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
            Bound::Included(-a[2] * 1 + l[0] + -l[3] * 1 + a[3]),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !((!(l[1] <= l[0])) || (!(l[2] <= l[1])) || (!(l[2] == l[3])) || (l[4] == l[2])) {
        let tmp = IntervalList::from_interval_lists(vec![
            IntervalList::interval_point(Bound::Included(a[4] + -l[4] * 1 + -a[3] * 1 + l[2])),
            IntervalList::interval_upper(Bound::Excluded(a[4] + -a[3] * 1 + -l[4] * 1 + l[0])),
        ]);
        ret = ret.intersection(&tmp);
    }
    if !(s[3] >= a[2] + -l[1] * 1) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
            Bound::Included(-s[1] * 1 + s[3]),
        )]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] + -a[1] * 1 + s[2]),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[3] + -a[2] * 1 + l[1]),
    )]);
    ret = ret.intersection(&tmp);
    if !(s[3] >= a[2] + -l[1] * 1) {
        let tmp = IntervalList::from_interval_lists(vec![
            IntervalList::interval_lower(Bound::Included(a[1] + -l[0] * 1 + -s[0] * 1)),
            IntervalList::interval_upper(Bound::Included(-s[0] * 1 / 2 + s[3] * 1 / 2)),
        ]);
        ret = ret.intersection(&tmp);
    }
    if !(s[3] >= a[2] + -l[0] * 1) {
        let tmp = IntervalList::from_interval_lists(vec![
            IntervalList::interval_upper(Bound::Excluded(a[1] + -l[0] * 1 + -s[0] * 1)),
            IntervalList::interval_upper(Bound::Included(l[0] + -a[1] * 1 + s[3])),
        ]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[2] * 1 / 3 + -s[0] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 2 + -a[0] * 1 / 2 + s[2] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    if !(s[4] >= a[3] + -l[2] * 1) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_upper(
            Bound::Included(-s[2] * 1 + s[4]),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !((!(l[1] <= l[0])) || (s[4] >= a[3] + -l[2] * 1)) {
        let tmp = IntervalList::from_interval_lists(vec![
            IntervalList::interval_upper(Bound::Included(-s[1] * 1 / 2 + s[4] * 1 / 2)),
            IntervalList::interval_lower(Bound::Included(-l[0] * 1 + -s[1] * 1 + a[2])),
        ]);
        ret = ret.intersection(&tmp);
    }
    if !(!(l[4] <= l[3])) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Included(a[5] * 1 / 2 + -a[3] * 1 / 2),
        )]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[2] * 1 / 3 + s[4] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[1] * 1 / 3 + s[3] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[2] * 1 / 2 + s[3] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(s[4] * 1 / 2 + -a[2] * 1 / 2 + l[1] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[3] * 1 / 2 + s[4] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    if !(s[4] >= a[3] + -l[2] * 1) {
        let tmp = IntervalList::from_interval_lists(vec![
            IntervalList::interval_upper(Bound::Included(s[4] + -a[2] * 1 + l[1])),
            IntervalList::interval_upper(Bound::Included(-s[0] * 1 / 3 + s[4] * 1 / 3)),
            IntervalList::interval_upper(Bound::Included(
                -a[1] * 1 / 2 + l[0] * 1 / 2 + s[4] * 1 / 2,
            )),
        ]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 3 + -a[0] * 1 / 3 + s[3] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    if !((!(l[0] == l[2])) || (s[4] >= a[3] + -l[0] * 1)) {
        let tmp = IntervalList::from_interval_lists(vec![
            IntervalList::interval_upper(Bound::Included(s[4] + -a[2] * 1 + l[0])),
            IntervalList::interval_upper(Bound::Excluded(-l[0] * 1 + -s[1] * 1 + a[2])),
        ]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-a[1] * 1 / 2 + l[0] * 1 / 2 + s[3] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[0] * 1 / 4 + s[3] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    if !((!(l[3] <= l[2])) || (!(l[4] <= l[3]))) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Included(a[5] * 1 / 3 + -a[2] * 1 / 3),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !((!(l[1] <= l[0])) || (!(l[2] <= l[1])) || (!(l[3] <= l[2])) || (!(l[4] <= l[3]))) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Included(-l[0] * 1 / 3 + a[5] * 1 / 3 + -s[3] * 1 / 3),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !((!(l[1] <= l[0])) || (!(l[2] <= l[1])) || (!(l[3] <= l[2])) || (!(l[4] <= l[3]))) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Included(-l[0] * 1 / 2 + a[5] * 1 / 2 + -s[4] * 1 / 2),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !((!(l[1] <= l[0])) || (!(l[2] <= l[1])) || (!(l[3] <= l[2])) || (!(l[4] <= l[3]))) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Included(-l[0] * 1 / 4 + -s[2] * 1 / 4 + a[5] * 1 / 4),
        )]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(l[0] * 1 / 4 + -a[0] * 1 / 4 + s[4] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[1] * 1 / 4 + s[4] * 1 / 4),
    )]);
    ret = ret.intersection(&tmp);
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-a[1] * 1 / 3 + l[0] * 1 / 3 + s[4] * 1 / 3),
    )]);
    ret = ret.intersection(&tmp);
    if !((!(l[1] <= l[0])) || (!(l[2] <= l[1])) || (!(l[3] <= l[2])) || (!(l[4] <= l[3]))) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Included(-l[0] * 1 / 6 + -s[0] * 1 / 6 + a[5] * 1 / 6),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !((!(l[2] <= l[1])) || (!(l[3] <= l[2])) || (!(l[4] <= l[3]))) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Included(-a[1] * 1 / 4 + a[5] * 1 / 4),
        )]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[0] * 1 / 5 + s[4] * 1 / 5),
    )]);
    ret = ret.intersection(&tmp);
    if !((!(l[1] <= l[0])) || (!(l[2] <= l[1])) || (!(l[3] <= l[2])) || (!(l[4] <= l[3]))) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Included(-l[0] * 1 / 5 + -s[1] * 1 / 5 + a[5] * 1 / 5),
        )]);
        ret = ret.intersection(&tmp);
    }
    if !((!(l[1] <= l[0])) || (!(l[2] <= l[1])) || (!(l[3] <= l[2])) || (!(l[4] <= l[3]))) {
        let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
            Bound::Included(-a[0] * 1 / 5 + a[5] * 1 / 5),
        )]);
        ret = ret.intersection(&tmp);
    }
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(RealNumRep::new(5, 1)),
    )]);
    ret = ret.intersection(&tmp);
    assert!(L0 >= RealNumRep::new(0, 1));
    assert!(l[0] + -a[0] * 1 + s[0] <= RealNumRep::new(0, 1));
    let tmp = IntervalList::from_interval_lists(vec![IntervalList::interval_lower(
        Bound::Included(-s[1] * 1 / 2 + s[2] * 1 / 2),
    )]);
    ret = ret.intersection(&tmp);
    ret
}
