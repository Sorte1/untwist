use super::bits::Bits;

const SAFE_MAGNITUDE: u8 = 53;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Int32,
    Uint32,
    Wrapped(u8),
    Undefined,
}

impl Kind {
    const fn magnitude(self) -> u8 {
        match self {
            Kind::Int32 => 31,
            Kind::Uint32 => 32,
            Kind::Wrapped(magnitude) => magnitude,
            Kind::Undefined => SAFE_MAGNITUDE,
        }
    }

    const fn is_exact(self) -> bool {
        matches!(self, Kind::Int32 | Kind::Uint32)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Val {
    pub bits: Bits,
    pub kind: Kind,
}

impl Val {
    pub const fn int32(bits: Bits) -> Self {
        Self {
            bits,
            kind: Kind::Int32,
        }
    }

    pub const fn undefined() -> Self {
        Self {
            bits: Bits::exact(0),
            kind: Kind::Undefined,
        }
    }

    pub const fn uint32(bits: Bits) -> Self {
        Self {
            bits,
            kind: Kind::Uint32,
        }
    }

    pub fn from_number(value: f64) -> Option<Self> {
        if !value.is_finite() || value.fract() != 0.0 {
            return None;
        }
        let magnitude = value.abs();
        if magnitude >= 9_007_199_254_740_992.0 {
            return None;
        }
        let wrapped = (value as i64) as u32;
        let bits = Bits::exact(wrapped);
        let kind = if (-2_147_483_648.0..2_147_483_648.0).contains(&value) {
            Kind::Int32
        } else if (0.0..4_294_967_296.0).contains(&value) {
            Kind::Uint32
        } else {
            Kind::Wrapped(SAFE_MAGNITUDE)
        };
        Some(Self { bits, kind })
    }

    pub fn number(self) -> Option<f64> {
        let exact = self.bits.value()?;
        match self.kind {
            Kind::Int32 => Some(f64::from(exact as i32)),
            Kind::Uint32 => Some(f64::from(exact)),
            Kind::Wrapped(_) | Kind::Undefined => None,
        }
    }

    pub fn arithmetic(self, other: Val, bits: Bits) -> Option<Self> {
        if self.kind == Kind::Undefined || other.kind == Kind::Undefined {
            return None;
        }
        let magnitude = self.kind.magnitude().max(other.kind.magnitude()) + 1;
        (magnitude <= SAFE_MAGNITUDE).then_some(Self {
            bits,
            kind: Kind::Wrapped(magnitude),
        })
    }

    pub fn negated(self) -> Option<Self> {
        if self.kind == Kind::Undefined {
            return None;
        }
        if let Some(value) = self.number() {
            return Self::from_number(-value);
        }
        Some(Self {
            bits: self.bits.neg(),
            kind: Kind::Wrapped(self.kind.magnitude()),
        })
    }

    pub fn truthy(self) -> Option<bool> {
        if self.bits.ones != 0 {
            return Some(true);
        }
        (self.kind.is_exact() && self.bits.value() == Some(0)).then_some(false)
    }

    pub fn equals(self, other: Val) -> Option<bool> {
        if !self.kind.is_exact() || !other.kind.is_exact() {
            return None;
        }
        let conflict = (self.bits.ones & other.bits.zeros) | (self.bits.zeros & other.bits.ones);
        if self.kind == other.kind {
            if conflict != 0 {
                return Some(false);
            }
        } else {
            let negative = (self.bits.ones | other.bits.ones) & 0x8000_0000 != 0;
            if conflict != 0 || negative {
                return Some(false);
            }
        }
        match (self.number(), other.number()) {
            (Some(left), Some(right)) => Some(left == right),
            _ => None,
        }
    }
}
