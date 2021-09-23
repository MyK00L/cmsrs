use super::ProtoScore;
use std::cmp::Ord;
use std::iter::Sum;
use std::ops::{Add, Div, Mul};

pub trait ScoreTrait:
    Ord
    + Default
    + Add<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Sum
    + From<ProtoScore>
    + Into<ProtoScore>
{
    fn rescale(&mut self, old_min_score: Self, new_min_score: Self);
}
impl<
        T: Ord
            + Default
            + Add<Output = Self>
            + Mul<Output = Self>
            + Div<Output = Self>
            + Sum
            + From<ProtoScore>
            + Into<ProtoScore>
            + Copy,
    > ScoreTrait for T
{
    fn rescale(&mut self, old_min_score: Self, new_min_score: Self) {
        *self = (*self) * new_min_score / old_min_score;
    }
}
