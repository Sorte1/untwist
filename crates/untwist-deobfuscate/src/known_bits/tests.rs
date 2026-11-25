use super::bits::Bits;
use super::scatter;
use super::value::{Kind, Val};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 16) as u32
    }

    fn bits(&mut self) -> (Bits, u32) {
        let value = self.next();
        let known = match self.next() % 4 {
            0 => u32::MAX,
            1 => self.next() & self.next(),
            2 => !(self.next() & self.next() & self.next()),
            _ => self.next(),
        };
        (
            Bits {
                zeros: !value & known,
                ones: value & known,
            },
            value,
        )
    }

    fn member(&mut self, bits: Bits) -> u32 {
        bits.ones | (self.next() & bits.unknown())
    }
}

fn check(name: &str, abstract_op: fn(Bits, Bits) -> Bits, concrete_op: fn(u32, u32) -> u32) {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for _ in 0..20_000 {
        let (left, _) = rng.bits();
        let (right, _) = rng.bits();
        let out = abstract_op(left, right);
        assert_eq!(out.zeros & out.ones, 0, "{name}: contradictory bits");
        for _ in 0..8 {
            let a = rng.member(left);
            let b = rng.member(right);
            let value = concrete_op(a, b);
            assert!(
                out.admits(value),
                "{name}: {left:?} {right:?} -> {out:?} but {a:#x},{b:#x} gives {value:#x}"
            );
        }
    }
}

#[test]
fn every_transfer_function_admits_every_concrete_result() {
    check("and", Bits::and, |a, b| a & b);
    check("or", Bits::or, |a, b| a | b);
    check("xor", Bits::xor, |a, b| a ^ b);
    check("add", Bits::add, u32::wrapping_add);
    check("sub", Bits::sub, u32::wrapping_sub);
    check("mul", Bits::mul, u32::wrapping_mul);
    check("join", Bits::join, |a, _| a);
    check("not", |a, _| a.not(), |a, _| !a);
    check("neg", |a, _| a.neg(), |a, _| a.wrapping_neg());
    for count in [0, 1, 5, 19, 31, 32, 33] {
        let mut rng = Rng(u64::from(count) + 7);
        for _ in 0..5_000 {
            let (bits, _) = rng.bits();
            let value = rng.member(bits);
            assert!(bits.shl(count).admits(value << (count & 31)));
            assert!(bits
                .sar(count)
                .admits(((value as i32) >> (count & 31)) as u32));
            assert!(bits.shr(count).admits(value >> (count & 31)));
        }
    }
}

#[test]
fn exact_inputs_give_exact_results() {
    let a = Bits::exact(0x1234_5678);
    let b = Bits::exact(0x9abc_def1);
    assert_eq!(
        a.add(b).value(),
        Some(0x1234_5678u32.wrapping_add(0x9abc_def1))
    );
    assert_eq!(
        a.sub(b).value(),
        Some(0x1234_5678u32.wrapping_sub(0x9abc_def1))
    );
    assert_eq!(
        a.mul(b).value(),
        Some(0x1234_5678u32.wrapping_mul(0x9abc_def1))
    );
}

#[test]
fn an_odd_operand_keeps_the_low_bit_of_a_product_with_its_even_successor() {
    let seed = Bits {
        zeros: 0xfff8_0002,
        ones: 0,
    };
    let odd = seed.or(Bits::exact(109));
    let product = odd.mul(odd.add(Bits::exact(1)));
    assert_eq!(product.and(Bits::exact(1)).value(), Some(0));
}

#[test]
fn numbers_classify_by_range() {
    assert_eq!(Val::from_number(-1.0).map(|v| v.kind), Some(Kind::Int32));
    assert_eq!(
        Val::from_number(3_000_000_000.0).map(|v| v.kind),
        Some(Kind::Uint32)
    );
    assert_eq!(
        Val::from_number(1e10).map(|v| v.kind),
        Some(Kind::Wrapped(53))
    );
    assert_eq!(Val::from_number(0.5), None);
    assert_eq!(Val::from_number(f64::NAN), None);
}

#[test]
fn equality_respects_the_number_each_kind_stands_for() {
    let minus_one = Val::from_number(-1.0).unwrap();
    let big = Val::from_number(4_294_967_295.0).unwrap();
    assert_eq!(minus_one.bits, big.bits);
    assert_eq!(minus_one.equals(big), Some(false));
    let low_bit = Val::int32(Bits { zeros: !1, ones: 0 });
    assert_eq!(low_bit.equals(Val::from_number(2.0).unwrap()), Some(false));
    assert_eq!(low_bit.equals(Val::from_number(1.0).unwrap()), None);
    let wrapped = Val {
        bits: Bits::exact(0),
        kind: Kind::Wrapped(33),
    };
    assert_eq!(wrapped.equals(Val::from_number(0.0).unwrap()), None);
    assert_eq!(wrapped.truthy(), None);
}

#[test]
fn scatter_places_bits_in_mask_order() {
    assert_eq!(scatter(0b11, 0b1010), 0b1010);
    assert_eq!(scatter(0b01, 0b1010), 0b0010);
    assert_eq!(scatter(0b10, 0b1010), 0b1000);
}
