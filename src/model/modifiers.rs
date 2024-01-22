use std::fmt::{self, Debug, Display};

use bitflags::bitflags;

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
    }
}

impl Modifiers {
    pub const CLASS_MODIFIERS: Modifiers = Self::Public
        | Self::Protected
        | Self::Private
        | Self::Static
        | Self::Final
        | Self::Abstract
        | Self::Strict;
    pub const INTERFACE_MODIFIERS: Modifiers = Self::Public
        | Self::Protected
        | Self::Private
        | Self::Abstract
        | Self::Static
        | Self::Strict;
    pub const CONSTRUCTOR_MODIFIERS: Modifiers =
        Self::Public | Self::Protected | Self::Private;
    pub const METHOD_MODIFIERS: Modifiers = Self::Public
        | Self::Protected
        | Self::Private
        | Self::Static
        | Self::Final
        | Self::Abstract
        | Self::Native
        | Self::Synchronized
        | Self::Strict;
    pub const FIELD_MODIFIERS: Modifiers = Self::Public
        | Self::Protected
        | Self::Private
        | Self::Static
        | Self::Final
        | Self::Transient
        | Self::Volatile;
    pub const PARAMETER_MODIFIERS: Modifiers = Self::Final;
    pub const ACCESS_MODIFIERS: Modifiers = Self::Public | Self::Protected | Self::Private;
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
