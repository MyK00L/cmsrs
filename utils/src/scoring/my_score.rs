use super::score::ScoreTrait;
use super::ProtoScore;
use ordered_float::NotNan;
use std::cmp::Ord;
use std::iter::Sum;
use std::ops::{Add, Div, Mul};

#[derive(Copy, Clone, Debug)]
pub struct MyScore {
    score: NotNan<f64>,
}
impl Ord for MyScore {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.score.cmp(&o.score)
    }
}
impl PartialOrd for MyScore {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl PartialEq for MyScore {
    fn eq(&self, o: &Self) -> bool {
        self.score.eq(&o.score)
    }
}
impl Eq for MyScore {}

impl Sum for MyScore {
    fn sum<I>(it: I) -> Self
    where
        I: Iterator<Item = Self>,
    {
        Self {
            score: it.map(|x| x.score).sum(),
        }
    }
}
impl Add for MyScore {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self {
            score: self.score + o.score,
        }
    }
}
impl Mul for MyScore {
    type Output = Self;
    fn mul(self, o: Self) -> Self {
        Self {
            score: self.score * o.score,
        }
    }
}
impl Div for MyScore {
    type Output = Self;
    fn div(self, o: Self) -> Self {
        Self {
            score: self.score / o.score,
        }
    }
}
impl From<ProtoScore> for MyScore {
    fn from(score: ProtoScore) -> Self {
        Self {
            score: NotNan::new(score.score).unwrap(),
        }
    }
}
impl From<MyScore> for ProtoScore {
    fn from(score: MyScore) -> ProtoScore {
        ProtoScore {
            score: score.score.into_inner(),
        }
    }
}
impl ScoreTrait for MyScore {
    fn rescale(&mut self, old_max_score: Self, new_max_score: Self) {
        *self = (*self) * new_max_score / old_max_score;
    }
    fn one() -> Self {
        Self {
            score: NotNan::new(1.0).unwrap(),
        }
    }
    fn is_one(&self) -> bool {
        *self == Self::one()
    }
    fn zero() -> Self {
        Self {
            score: NotNan::new(0.0).unwrap(),
        }
    }
    fn is_zero(&self) -> bool {
        *self == Self::zero()
    }
}
