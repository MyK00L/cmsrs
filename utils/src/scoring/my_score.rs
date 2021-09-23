use super::ProtoScore;
use ordered_float::NotNan;
use std::cmp::Ord;
use std::iter::Sum;
use std::ops::{Add, Div, Mul};

#[derive(Copy, Clone, Debug)]
pub struct MyScore {
    score: NotNan<f64>,
    is_bool: bool,
}
impl MyScore {
    pub fn from_f64(s: f64) -> Self {
        // temporary because protos have max_score saved as double and note OneOfScore
        Self {
            score: NotNan::new(s).unwrap(),
            is_bool: false,
        }
    }
}
impl Default for MyScore {
    fn default() -> Self {
        Self {
            score: NotNan::new(0.0).unwrap(),
            is_bool: false,
        }
    }
}
impl Ord for MyScore {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        assert_eq!(self.is_bool, o.is_bool);
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
        assert_eq!(self.is_bool, o.is_bool);
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
            is_bool: false,
        }
    }
}
impl Add for MyScore {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        assert_eq!(self.is_bool, o.is_bool);
        Self {
            score: self.score + o.score,
            is_bool: false,
        }
    }
}
impl Mul for MyScore {
    type Output = Self;
    fn mul(self, o: Self) -> Self {
        assert_eq!(self.is_bool, o.is_bool);
        Self {
            score: self.score * o.score,
            is_bool: false,
        }
    }
}
impl Div for MyScore {
    type Output = Self;
    fn div(self, o: Self) -> Self {
        assert_eq!(self.is_bool, o.is_bool);
        Self {
            score: self.score / o.score,
            is_bool: false,
        }
    }
}
impl From<ProtoScore> for MyScore {
    fn from(score: ProtoScore) -> Self {
        match score.score.unwrap() {
            protos::scoring::one_of_score::Score::BoolScore(s) => Self {
                score: NotNan::new(if s { 1.0 } else { 0.0 }).unwrap(),
                is_bool: true,
            },
            protos::scoring::one_of_score::Score::DoubleScore(s) => Self {
                score: NotNan::new(s).unwrap(),
                is_bool: false,
            },
        }
    }
}
impl From<MyScore> for ProtoScore {
    fn from(score: MyScore) -> ProtoScore {
        ProtoScore {
            score: Some(if score.is_bool {
                protos::scoring::one_of_score::Score::BoolScore(
                    if score.score == NotNan::new(0.0).unwrap() {
                        false
                    } else if score.score == NotNan::new(1.0).unwrap() {
                        true
                    } else {
                        panic!("Bool score not 0 or 1, should not happen")
                    },
                )
            } else {
                protos::scoring::one_of_score::Score::DoubleScore(score.score.into_inner())
            }),
        }
    }
}
