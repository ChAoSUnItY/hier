use std::ops::BitAnd;

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifiers {
    Public = 0x0001,
    Final = 0x0010,
    Interface = 0x0200,
}

impl BitAnd for Modifiers {
    type Output = u16;

    fn bitand(self, rhs: Self) -> Self::Output {
        (self as u16) & (rhs as u16)
    }
}

impl BitAnd<Modifiers> for u16 {
    type Output = u16;

    fn bitand(self, rhs: Modifiers) -> Self::Output {
        self & (rhs as u16)
    }
}

impl BitAnd<u16> for Modifiers {
    type Output = u16;

    fn bitand(self, rhs: u16) -> Self::Output {
        (self as u16) & rhs
    }
}
