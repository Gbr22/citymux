use std::ops::Rem;

use num_traits::{AsPrimitive, One, Zero, FromPrimitive};

pub trait Scalar:
    Clone + Zero + One + Copy + std::fmt::Debug + std::fmt::Display + PartialEq + Eq + std::ops::Add<Output = Self> + std::ops::Sub<Output = Self> + std::ops::Mul<Output = Self> + std::ops::Div<Output = Self> + Ord + AsPrimitive<usize> + FromPrimitive + Rem<Output = Self>
{
    fn abs(self) -> Self {
        if self < Self::zero() {
            let minus_one = Self::zero() - Self::one();
            self * minus_one
        } else {
            self
        }
    }
    fn signum(self) -> Self {
        if self > Self::zero() {
            Self::one()
        }
        else if self < Self::zero() {
            Self::zero() - Self::one()
        }
        else {
            self
        }
    }
}

impl <T: Clone + Zero + One + Copy + std::fmt::Debug + std::fmt::Display + PartialEq + Eq + std::ops::Add<Output = Self> + std::ops::Sub<Output = Self> + std::ops::Mul<Output = Self> + std::ops::Div<Output = Self> + Ord + AsPrimitive<usize> + FromPrimitive + Rem<Output = Self>> Scalar for T
{
    
}
