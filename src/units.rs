use derive_more::{
    Add, AddAssign, Deref, DerefMut, Display, DivAssign, From, Into, MulAssign, Sub, SubAssign, Sum,
};

#[derive(
    Debug,
    Copy,
    Clone,
    Default,
    Display,
    From,
    Into,
    Deref,
    DerefMut,
    PartialEq,
    PartialOrd,
    Add,
    Sub,
    Sum,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
)]
pub struct Pt(pub f32);

impl<T: Into<f32>> std::ops::Mul<T> for Pt {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        let rhs: f32 = rhs.into();
        Pt(self.0 * rhs)
    }
}

impl<T: Into<f32>> std::ops::Div<T> for Pt {
    type Output = Self;

    fn div(self, rhs: T) -> Self::Output {
        let rhs: f32 = rhs.into();
        Pt(self.0 / rhs)
    }
}

#[derive(
    Debug,
    Copy,
    Clone,
    Default,
    Display,
    From,
    Into,
    Deref,
    DerefMut,
    PartialEq,
    PartialOrd,
    Add,
    Sub,
    Sum,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
)]
pub struct In(pub f32);

impl<T: Into<f32>> std::ops::Mul<T> for In {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        let rhs: f32 = rhs.into();
        In(self.0 * rhs)
    }
}

impl<T: Into<f32>> std::ops::Div<T> for In {
    type Output = Self;

    fn div(self, rhs: T) -> Self::Output {
        let rhs: f32 = rhs.into();
        In(self.0 / rhs)
    }
}

#[derive(
    Debug,
    Copy,
    Clone,
    Default,
    Display,
    From,
    Into,
    Deref,
    DerefMut,
    PartialEq,
    PartialOrd,
    Add,
    Sub,
    Sum,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
)]
pub struct Mm(pub f32);

impl<T: Into<f32>> std::ops::Mul<T> for Mm {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        let rhs: f32 = rhs.into();
        Mm(self.0 * rhs)
    }
}

impl<T: Into<f32>> std::ops::Div<T> for Mm {
    type Output = Self;

    fn div(self, rhs: T) -> Self::Output {
        let rhs: f32 = rhs.into();
        Mm(self.0 / rhs)
    }
}

impl From<In> for Pt {
    fn from(inches: In) -> Self {
        Pt(*inches * 72.0)
    }
}

impl From<Mm> for Pt {
    fn from(mm: Mm) -> Self {
        Pt(*mm * 72.0 / 25.4)
    }
}

impl From<Pt> for In {
    fn from(points: Pt) -> Self {
        In(*points / 72.0)
    }
}

impl From<Pt> for Mm {
    fn from(points: Pt) -> Self {
        Mm(*points / 72.0 * 25.4)
    }
}

impl From<In> for Mm {
    fn from(inches: In) -> Self {
        Mm(*inches * 25.4)
    }
}

impl From<Mm> for In {
    fn from(mm: Mm) -> Self {
        In(*mm / 25.4)
    }
}
