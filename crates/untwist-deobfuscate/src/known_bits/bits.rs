#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bits {
    pub zeros: u32,
    pub ones: u32,
}

impl Bits {
    pub const TOP: Bits = Bits { zeros: 0, ones: 0 };

    pub const fn exact(value: u32) -> Self {
        Self {
            zeros: !value,
            ones: value,
        }
    }

    pub const fn known(self) -> u32 {
        self.zeros | self.ones
    }

    pub const fn unknown(self) -> u32 {
        !self.known()
    }

    pub const fn value(self) -> Option<u32> {
        if self.unknown() == 0 {
            Some(self.ones)
        } else {
            None
        }
    }

    pub const fn admits(self, value: u32) -> bool {
        value & self.zeros == 0 && !value & self.ones == 0
    }

    pub const fn join(self, other: Bits) -> Self {
        Self {
            zeros: self.zeros & other.zeros,
            ones: self.ones & other.ones,
        }
    }

    pub const fn and(self, other: Bits) -> Self {
        Self {
            zeros: self.zeros | other.zeros,
            ones: self.ones & other.ones,
        }
    }

    pub const fn or(self, other: Bits) -> Self {
        Self {
            zeros: self.zeros & other.zeros,
            ones: self.ones | other.ones,
        }
    }

    pub const fn xor(self, other: Bits) -> Self {
        let known = self.known() & other.known();
        let value = self.ones ^ other.ones;
        Self {
            zeros: !value & known,
            ones: value & known,
        }
    }

    pub const fn not(self) -> Self {
        Self {
            zeros: self.ones,
            ones: self.zeros,
        }
    }

    const fn add_carry(self, other: Bits, carry: bool) -> Self {
        let carry = carry as u32;
        let sum_if_zero = (!self.zeros).wrapping_add(!other.zeros).wrapping_add(carry);
        let sum_if_one = self.ones.wrapping_add(other.ones).wrapping_add(carry);
        let carry_zero = !(sum_if_zero ^ self.zeros ^ other.zeros);
        let carry_one = sum_if_one ^ self.ones ^ other.ones;
        let known = self.known() & other.known() & (carry_zero | carry_one);
        Self {
            zeros: !sum_if_zero & known,
            ones: sum_if_one & known,
        }
    }

    pub const fn add(self, other: Bits) -> Self {
        self.add_carry(other, false)
    }

    pub const fn sub(self, other: Bits) -> Self {
        self.add_carry(other.not(), true)
    }

    pub const fn neg(self) -> Self {
        Bits::exact(0).sub(self)
    }

    const fn known_low_run(self) -> u32 {
        self.unknown().trailing_zeros()
    }

    const fn known_trailing_zeros(self) -> u32 {
        (!self.zeros).trailing_zeros()
    }

    pub const fn mul(self, other: Bits) -> Self {
        let zeros_below = self.known_trailing_zeros() + other.known_trailing_zeros();
        let zero_mask = low_mask(zeros_below);
        let run = min(self.known_low_run(), other.known_low_run());
        let run_mask = low_mask(run);
        let product = self.ones.wrapping_mul(other.ones);
        Self {
            zeros: zero_mask | (!product & run_mask),
            ones: product & run_mask & !zero_mask,
        }
    }

    pub const fn shl(self, count: u32) -> Self {
        let count = count & 31;
        Self {
            zeros: (self.zeros << count) | low_mask(count),
            ones: self.ones << count,
        }
    }

    pub const fn sar(self, count: u32) -> Self {
        let count = count & 31;
        Self {
            zeros: ((self.zeros as i32) >> count) as u32,
            ones: ((self.ones as i32) >> count) as u32,
        }
    }

    pub const fn shr(self, count: u32) -> Self {
        let count = count & 31;
        Self {
            zeros: (self.zeros >> count) | !(u32::MAX >> count),
            ones: self.ones >> count,
        }
    }
}

pub const fn low_mask(count: u32) -> u32 {
    if count >= 32 {
        u32::MAX
    } else {
        (1u32 << count) - 1
    }
}

const fn min(left: u32, right: u32) -> u32 {
    if left < right {
        left
    } else {
        right
    }
}
