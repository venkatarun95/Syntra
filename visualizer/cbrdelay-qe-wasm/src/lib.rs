//! Browser-only adapter for the generated CBR-delay QE functions.
//! Kept separate from both the generic visualizer and generated QE output.
use ds::{interval::IntervalList, RealNumRep};
use std::ops::Bound;

mod qe_generated;
use qe_generated::*;

const MAX_POINTS: usize = 4096;
static mut OUTPUT: [f64; 1 + MAX_POINTS * 3] = [0.0; 1 + MAX_POINTS * 3];

fn real(x: f64) -> RealNumRep { RealNumRep::new((x * 1000.0).round() as i32, 1000) }
fn decimal(x: RealNumRep) -> f64 { (*x.numer() as f64) / (*x.denom() as f64) }
fn clipped(x: IntervalList<RealNumRep>, limit: RealNumRep) -> IntervalList<RealNumRep> {
    x.intersection(&IntervalList::interval_bounded(Bound::Included(0.into()), Bound::Included(limit)))
}
fn samples(x: &IntervalList<RealNumRep>, n: usize) -> Vec<RealNumRep> {
    let mut out=Vec::new(); for i in x.get_intervals() {
        let lo=match i.start {Bound::Included(v)|Bound::Excluded(v)=>v,Bound::Unbounded=>0.into()};
        let hi=match i.end {Bound::Included(v)|Bound::Excluded(v)=>v,Bound::Unbounded=>lo};
        for j in 0..n.max(1) {out.push(lo+(hi-lo)*(j as i32)/((n.max(1)-1).max(1) as i32));}
    } out.sort();out.dedup();out
}
fn q_fun(t: i32) -> &'static std::collections::HashMap<(i32,i32), QeFun> {match t {2=>&COMPUTE_Q_1,3=>&COMPUTE_Q_2,4=>&COMPUTE_Q_3,5=>&COMPUTE_Q_4,6=>&COMPUTE_Q_5,_=>panic!("T must be 2..6")}}

#[no_mangle]
pub extern "C" fn qe_output_ptr() -> *const f64 { std::ptr::addr_of!(OUTPUT) as *const f64 }

/// Inputs are contiguous f64 A, S, and L arrays. Output is [count, C,B,Q...].
#[no_mangle]
pub unsafe extern "C" fn qe_sample(a_ptr:*const f64,s_ptr:*const f64,l_ptr:*const f64,t:usize,nloss:usize,n:usize,limit:f64) -> usize {
    if !(2..=6).contains(&t)||nloss>t||n==0 {OUTPUT[0]=0.0;return 0}
    let a=std::slice::from_raw_parts(a_ptr,t).iter().copied().map(real).collect::<Vec<_>>();
    let s=std::slice::from_raw_parts(s_ptr,t).iter().copied().map(real).collect::<Vec<_>>();
    let l=std::slice::from_raw_parts(l_ptr,nloss).iter().copied().map(real).collect::<Vec<_>>();
    let key=(t as i32,nloss as i32); let l0=l.first().copied().unwrap_or_else(||0.into());
    let cs=samples(&clipped(COMPUTE_C[&key](&a,&l,&s,l0),real(limit)),n.min(10)); let mut count=0;
    for c in cs {for b in samples(&clipped(COMPUTE_B[&key](&a,&l,&s,l0,c),real(limit)),n.min(10)) {for q in samples(&clipped(q_fun(t as i32)[&key](&a,&l,&s,l0,c,b),real(limit)),n.min(10)) {
        if count==MAX_POINTS {break} let base=1+count*3;OUTPUT[base]=decimal(c);OUTPUT[base+1]=decimal(b);OUTPUT[base+2]=decimal(q);count+=1;
    }}}
    OUTPUT[0]=count as f64;count
}
