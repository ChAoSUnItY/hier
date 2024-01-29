#![allow(clippy::bad_bit_mask)]
use std::fmt::{self, Debug, Display};

use bitflags::bitflags;

macro_rules! __bitor_flags {
    ($($flags:ident),*) => {
        $(Self::$flags.bits() |)* 0
    };
}

macro_rules! __impl_flag_chk {
    ($flag:ident) => {
        __impl_flag_chk!($flag, concat!("[Modifiers::", stringify!($flag), "]"));
    };
    ($flag:ident as u16) => {
        __impl_flag_chk!(
            $flag as u16,
            concat!("[Modifiers::", stringify!($flag), "]")
        );
    };
    ($flag:ident, $flag_ref:expr) => {
        paste::paste! {
            #[doc = "Determine if provided [u16] has flag"]
            #[doc = $flag_ref]
            pub const fn [<is_ $flag:lower _bits>](bits: u16) -> bool {
                Self::from_bits_truncate(bits).[<is_ $flag:lower>]()
            }

            #[doc = "Determine if [Modifiers] has flag"]
            #[doc = $flag_ref]
            pub const fn [<is_ $flag:lower>](&self) -> bool {
                Self::contains(self, Self::$flag)
            }
        }
    };
    ($flag:ident as u16, $flag_ref:expr) => {
        paste::paste! {
            #[doc = "Determine if provided [u16] has flag"]
            #[doc = $flag_ref]
            pub const fn [<is_ $flag:lower _bits>](bits: u16) -> bool {
                bits & Self::$flag != 1
            }

            #[doc = "Determine if [Modifiers] has flag"]
            #[doc = $flag_ref]
            pub const fn [<is_ $flag:lower>](&self) -> bool {
                Self::[<is_ $flag:lower _bits>](self.bits())
            }
        }
    };
}

bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct Modifiers: u16 {
        const Public = 0x0001;
        const Private = 0x0002;
        const Protected = 0x0004;
        const Static = 0x0008;
        const Final = 0x0010;
        const Synchronized = 0x0020;
        const Volatile = 0x0040;
        const Transient = 0x0080;
        const Native = 0x0100;
        const Interface = 0x0200;
        const Abstract = 0x0400;
        const Strict = 0x0800;

        const CLASS_MODIFIERS = __bitor_flags!(Public, Protected, Private, Static, Final, Abstract, Strict);
        const INTERFACE_MODIFIERS = __bitor_flags!(Public, Protected, Private, Static, Abstract, Strict);
        const CONSTRUCTOR_MODIFIERS = __bitor_flags!(Public, Protected, Private);
        const METHOD_MODIFIERS = __bitor_flags!(Public, Protected, Private, Static, Final, Abstract, Native, Synchronized, Strict);
        const FIELD_MODIFIERS = __bitor_flags!(Public, Protected, Private, Static, Final, Transient, Volatile);
        const PARAMETER_MODIFIERS = __bitor_flags!(Final);
        const ACCESS_MODIFIERS = __bitor_flags!(Public, Protected, Private);
    }
}

impl Modifiers {
    // Bits not (yet) exposed to public API, either:
    // 1. Have different meanings for fields and methods
    // 2. Not a Java programming keywords
    #[allow(non_upper_case_globals)]
    pub(crate) const Bridge: u16 = 0x0040;
    #[allow(non_upper_case_globals)]
    pub(crate) const Varargs: u16 = 0x0080;
    #[allow(non_upper_case_globals)]
    pub(crate) const Synthetic: u16 = 0x1000;
    #[allow(non_upper_case_globals)]
    pub(crate) const Annotation: u16 = 0x2000;
    #[allow(non_upper_case_globals)]
    pub(crate) const Enum: u16 = 0x4000;
    #[allow(non_upper_case_globals)]
    pub(crate) const Mandated: u16 = 0x8000;

    __impl_flag_chk!(Public);
    __impl_flag_chk!(Private);
    __impl_flag_chk!(Protected);
    __impl_flag_chk!(Static);
    __impl_flag_chk!(Final);
    __impl_flag_chk!(Synchronized);
    __impl_flag_chk!(Volatile);
    __impl_flag_chk!(Transient);
    __impl_flag_chk!(Native);
    __impl_flag_chk!(Interface);
    __impl_flag_chk!(Abstract);
    __impl_flag_chk!(Strict);
    __impl_flag_chk!(Bridge as u16);
    __impl_flag_chk!(Varargs as u16);
    __impl_flag_chk!(Synthetic as u16);
    __impl_flag_chk!(Annotation as u16);
    __impl_flag_chk!(Enum as u16);
    __impl_flag_chk!(Mandated as u16);
}

impl Debug for Modifiers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl Display for Modifiers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
