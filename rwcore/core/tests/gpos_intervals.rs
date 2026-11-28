use rwcore::*;
use GposHorizontalFlow::*;
fn c(start:usize,end:usize,width:i64)->GposIntervalConstraint{GposIntervalConstraint{start,end,width}}
fn solve(n:&[i32],c:&[GposIntervalConstraint],flow:GposHorizontalFlow)->Result<Vec<i32>,Error>{
    solve_gpos_interval_widths(n,c,flow,Default::default())
}
fn check(out:&[i32],cs:&[GposIntervalConstraint]){
    for c in cs{assert_eq!(out[c.start..c.end].iter().map(|&x|i64::from(x)).sum::<i64>(),c.width);}
}
#[test]fn interval_direction_and_anchor_conversion(){
    let a=GposIntervalConstraint::from_logical_pair(0,2,130,10,-5,LeftToRight).unwrap();
    let b=GposIntervalConstraint::from_logical_pair(0,2,-130,-10,5,RightToLeft).unwrap();
    assert_eq!(a,c(0,2,145));assert_eq!(b,c(1,3,145));
    assert_eq!(solve(&[80,20,80],&[a],LeftToRight).unwrap(),vec![125,20,80]);
    assert_eq!(solve(&[80,20,80],&[b],RightToLeft).unwrap(),vec![80,20,125]);
}
#[test]fn interval_nested_same_start_and_crossing(){
    let cs=[c(0,3,130),c(0,1,50),c(1,4,170)];
    for flow in [LeftToRight,RightToLeft]{
        let x=solve(&[80,20,40,50],&cs,flow).unwrap();check(&x,&cs);
        assert_eq!(x[0],50);assert_eq!(x[3],90);
    }
    let cs=[c(0,2,100),c(1,3,120)];
    assert_eq!(solve(&[80,20,80],&cs,LeftToRight).unwrap(),vec![60,40,80]);
    assert_eq!(solve(&[80,20,80],&cs,RightToLeft).unwrap(),vec![80,20,100]);
}
#[test]fn interval_redundant_and_inconsistent_cycles(){
    let cs=[c(0,1,2),c(1,2,3),c(0,2,5),c(0,2,5)];
    assert_eq!(solve(&[0,0],&cs,LeftToRight).unwrap(),vec![2,3]);
    assert!(matches!(solve(&[0,0],&[c(0,1,2),c(1,2,3),c(0,2,6)],LeftToRight),Err(Error::Invalid("gpos interval inconsistent"))));
    assert!(solve(&[0],&[c(0,1,1),c(0,1,2)],RightToLeft).is_err());
}
#[test]fn interval_independent_exhaustive_reference(){
    // Exhaustively enumerate every width tuple in [-6,6]^3, and rank by
    // retention priorities. All tested target systems have a solution in
    // that range and the unconstrained width candidates remain in range.
    for a in -2..=2{for b in -2..=2{
        let cs=[c(0,2,a),c(1,3,b)];
        let nominal=[1,-1,2];
        for flow in [LeftToRight,RightToLeft]{
            let mut best=None;
            for x in -6..=6{for y in -6..=6{for z in -6..=6{
                if i64::from(x+y)!=a||i64::from(y+z)!=b{continue;}
                let v=[x,y,z];
                let score=match flow{LeftToRight=>[z==2,y== -1,x==1],RightToLeft=>[x==1,y== -1,z==2]};
                if best.as_ref().is_none_or(|(s,_)|score>*s){best=Some((score,v));}
            }}}
            let got=solve(&nominal,&cs,flow).unwrap();check(&got,&cs);
            assert_eq!(got,best.unwrap().1.to_vec());
        }
    }}
}
#[test]fn interval_permutation_and_no_constraints(){
    let n=[80,10,20,30,40];let mut cs=[c(0,3,150),c(1,4,90),c(2,5,100)];
    for flow in [LeftToRight,RightToLeft]{
        let x=solve(&n,&cs,flow).unwrap();cs.reverse();assert_eq!(x,solve(&n,&cs,flow).unwrap());check(&x,&cs);
        assert_eq!(solve(&n,&[],flow).unwrap(),n.to_vec());
        assert!(solve(&[],&[],flow).unwrap().is_empty());
    }
}
#[test]fn interval_bounds_and_overflow_are_atomic(){
    let n=[1,2];
    for cs in [[c(0,0,0)],[c(1,3,0)],[c(2,1,0)],[c(0,1,i64::MAX)]]{
        assert!(solve(&n,&cs,LeftToRight).is_err());
    }
    assert!(solve(&n,&[c(0,1,i64::MAX),c(1,2,1)],LeftToRight).is_err());
    assert!(GposIntervalConstraint::from_logical_pair(0,usize::MAX,1,0,0,RightToLeft).is_err());
    assert_eq!(n,[1,2]);
}
#[test]fn interval_limits_and_deep_chain(){
    let n=vec![1;20000];let cs:Vec<_>=(0..n.len()).map(|i|c(i,i+1,2)).collect();
    let l=GposIntervalLimits{max_glyphs:n.len(),max_constraints:cs.len(),max_work:300000};
    assert_eq!(solve_gpos_interval_widths(&n,&cs,LeftToRight,l).unwrap(),vec![2;n.len()]);
    for bad in [GposIntervalLimits{max_glyphs:n.len()-1,..l},GposIntervalLimits{max_constraints:cs.len()-1,..l},GposIntervalLimits{max_work:1,..l}]{
        assert!(solve_gpos_interval_widths(&n,&cs,LeftToRight,bad).is_err());
    }
}
