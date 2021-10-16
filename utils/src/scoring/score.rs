use super::ProtoScore;
use std::cmp::Ord;
use std::iter::Sum;
use std::ops::{Add, Div, Mul};

pub trait ScoreTrait:
    Ord
    + Add<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Sum
    + From<ProtoScore>
    + Into<ProtoScore>
{
    // rescales the score linearly from [0,old_max_score] to [0,new_max_score]
    fn rescale(&mut self, old_max_score: Self, new_max_score: Self);

    // returns a score with the mathematical properties of 0, it is always the minimum score
    fn zero() -> Self;

    // returns wether the score has the mathematical properties of 0
    fn is_zero(&self) -> bool;

    // returns a score with the mathematical properties of 1, it is the maximum score for single
    // testcases
    fn one() -> Self;

    // returns wether the score has the mathematical properties of 1
    fn is_one(&self) -> bool;
}
