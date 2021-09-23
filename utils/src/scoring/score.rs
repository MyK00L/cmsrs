use super::ProtoScore;
use std::cmp::Ord;
use std::ops::{Add, Div, Mul};

pub trait Score:
    Ord
    + Add<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + From<ProtoScore>
    + Into<ProtoScore>
{
    fn rescale(&mut self, old_min_score: Self, new_min_score: Self);
}
impl<
        T: Ord
            + Add<Output = Self>
            + Mul<Output = Self>
            + Div<Output = Self>
            + From<ProtoScore>
            + Into<ProtoScore>
            + Copy,
    > Score for T
{
    fn rescale(&mut self, old_min_score: Self, new_min_score: Self) {
        *self = (*self) * new_min_score / old_min_score;
    }
}
