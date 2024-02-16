// SPDX-FileCopyrightText: 2024 Nils Jochem
// SPDX-License-Identifier: MPL-2.0

use itertools::Itertools;

macro_rules! const_for {
    ($var: ident, $max: expr, $block: block) => {
        let mut $var = 0;
        while $var < $max {
            $block
            $var += 1;
        }
    };
}
/// Holds packed bits and manages access to them
///
/// From<uint> will index lowest to highest bit
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BitSet<const BYTES: usize> {
    bytes: [u8; BYTES],
}
impl<const N: usize> BitSet<N> {
    #[inline]
    fn prepare_fmt(f: &mut std::fmt::Formatter<'_>, radix_id: char) -> std::fmt::Result {
        f.write_str("Bitset(")?;
        if f.alternate() {
            write!(f, "0{radix_id}")?;
        }
        Ok(())
    }
    #[inline]
    fn finish_fmt(f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(")")
    }
    #[inline]
    fn fmt_bytes(&self, f: &mut std::fmt::Formatter<'_>, radix_id: char) -> std::fmt::Result {
        let iter = self.bytes.iter().rev();
        for &byte in iter {
            match radix_id {
                'X' => write!(f, "{byte:X}")?,
                'x' => write!(f, "{byte:x}")?,
                _ => unreachable!(),
            }
        }
        Ok(())
    }
}
impl<const N: usize> std::fmt::Debug for BitSet<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "{self:#b}")
        } else {
            write!(f, "{self:#x}")
        }
    }
}
impl<const N: usize> std::fmt::Binary for BitSet<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Self::prepare_fmt(f, 'b')?;

        for bit in self.into_iter().rev() {
            write!(f, "{:b}", bit as u8)?;
        }

        Self::finish_fmt(f)
    }
}
impl<const N: usize> std::fmt::LowerHex for BitSet<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Self::prepare_fmt(f, 'x')?;
        self.fmt_bytes(f, 'x')?;
        Self::finish_fmt(f)
    }
}
impl<const N: usize> std::fmt::UpperHex for BitSet<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Self::prepare_fmt(f, 'X')?;
        self.fmt_bytes(f, 'X')?;
        Self::finish_fmt(f)
    }
}

macro_rules! from_uint {
    ($bytes: expr, $int: ident) => {
        impl From<$int> for BitSet<$bytes> {
            fn from(value: $int) -> Self {
                Self::new(value.to_le_bytes())
            }
        }
        impl From<BitSet<$bytes>> for $int {
            fn from(value: BitSet<$bytes>) -> Self {
                Self::from_le_bytes(value.bytes)
            }
        }
    };
}
from_uint!(1, u8);
from_uint!(2, u16);
from_uint!(4, u32);
from_uint!(8, u64);
from_uint!(16, u128);
const USIZE_BYTES: usize = (usize::BITS / 8) as usize;
from_uint!(USIZE_BYTES, usize);

impl<const BYTES: usize> Default for BitSet<BYTES> {
    fn default() -> Self {
        Self::new([0; BYTES])
    }
}

impl<const BYTES: usize> BitSet<BYTES> {
    /// creates a new bitset from little endian bytes
    pub const fn new(bytes: [u8; BYTES]) -> Self {
        Self { bytes }
    }

    const fn split_index(index: usize) -> (usize, usize) {
        assert!(Self::is_in_bounds(index), "index out of bounds");
        (index % 8, index / 8)
    }
    /// checks if `index` is in bounds of this `BitSet`
    pub const fn is_in_bounds(index: usize) -> bool {
        index <= BYTES * 8
    }

    /// returns the current value of the bit at position `index`
    pub const fn get(&self, index: usize) -> bool {
        let (bit_index, byte_index) = Self::split_index(index);
        self.bytes[byte_index] & (1 << bit_index) != 0
    }

    /// flips the the bit at position `index`
    pub fn flip(&mut self, index: usize) {
        let (bit_index, byte_index) = Self::split_index(index);
        self.bytes[byte_index] ^= 1 << bit_index;
    }
    /// sets the the bit at position `index` to `value`
    pub fn set(&mut self, index: usize, value: bool) {
        const USE_ALTERNATIVE: bool = true;
        let (bit_index, byte_index) = Self::split_index(index);
        if USE_ALTERNATIVE {
            self.bytes[byte_index] ^= u8::from(value != self.get(index)) << bit_index;
        } else if value {
            self.bytes[byte_index] |= 1 << bit_index;
        } else {
            self.bytes[byte_index] &= !(1 << bit_index);
        }
    }

    /// counts the number of set bits
    pub const fn count(&self) -> usize {
        let mut count = 0;
        const_for!(i, BYTES, {
            count += self.bytes[i].count_ones() as usize;
        });
        count
    }
    /// are all bits set
    pub const fn all(&self) -> bool {
        const_for!(i, BYTES, {
            if self.bytes[i] == 0xFF {
                return false;
            }
        });
        true
    }
    /// is any bit set
    pub const fn any(&self) -> bool {
        const_for!(i, BYTES, {
            if self.bytes[i] > 0x00 {
                return true;
            }
        });
        false
    }
    /// is no bit set
    pub const fn none(&self) -> bool {
        !self.any()
    }

    /// sets all bits to false
    pub fn clear(&mut self) {
        self.bytes = [0; BYTES];
    }

    /// calculates the union between `self` and `other`
    pub const fn union(&self, other: &Self) -> Self {
        let mut data = self.bytes;
        const_for!(i, BYTES, {
            data[i] |= other.bytes[i];
        });
        Self::new(data)
    }
    /// calculates the intersection between `self` and `other`
    pub const fn intersection(&self, other: &Self) -> Self {
        let mut data = self.bytes;
        const_for!(i, BYTES, {
            data[i] &= other.bytes[i];
        });
        Self::new(data)
    }
}

macro_rules! impl_ops {
    (bit_and) => {
        impl_ops!(std::ops::BitAnd, bitand, std::ops::BitAndAssign::bitand_assign);
        impl_ops!(std::ops::BitAndAssign, bitand_assign);
    };
    (bit_or) => {
        impl_ops!(std::ops::BitOr, bitor, std::ops::BitOrAssign::bitor_assign);
        impl_ops!(std::ops::BitOrAssign, bitor_assign);
    };
    (bit_xor) => {
        impl_ops!(std::ops::BitXor, bitxor, std::ops::BitXorAssign::bitxor_assign);
        impl_ops!(std::ops::BitXorAssign, bitxor_assign);
    };
    // (shl) => {
    //     impl_ops!(std::ops::Shl, shl_, std::ops::ShlAssign::shl_assign);
    //     impl_ops!(std::ops::ShlAssign, shl_assign);
    // };

    ($($trt:ident)::*, $fn_name: ident, $assign_fn: path) => {
		impl<const BYTES: usize> $($trt)::*<&Self> for BitSet<BYTES> {
			type Output = Self;

			fn $fn_name(mut self, rhs: &Self) -> Self::Output {
				$assign_fn(&mut self, rhs);
				self
			}
		}
	};
    ($($trt:ident)::*, $fn_name: ident) => {
		impl<const BYTES: usize> $($trt)::*<&Self> for BitSet<BYTES> {
			fn $fn_name(&mut self, rhs: &Self) {
				for (byte, other) in self.bytes.iter_mut().zip(rhs.bytes) {
					byte.$fn_name(other);
				}
			}
		}
    };
}
impl_ops!(bit_and);
impl_ops!(bit_or);
impl_ops!(bit_xor);
impl<const BYTES: usize> std::ops::Not for &BitSet<BYTES> {
    type Output = BitSet<BYTES>;
    fn not(self) -> Self::Output {
        BitSet::new(self.bytes.map(std::ops::Not::not))
    }
}
impl<const BYTES: usize> std::ops::Shl<usize> for BitSet<BYTES> {
    type Output = Self;

    fn shl(mut self, rhs: usize) -> Self::Output {
        match BYTES {
            1 => self.bytes[0] <<= rhs,
            _ if rhs == 0 => {}
            _ => {
                let (s_bits, s_bytes) = Self::split_index(rhs);
                if s_bits == 0 {
                    self.bytes.rotate_right(s_bytes);
                    for i in 0..s_bytes {
                        self.bytes[i] = 0;
                    }
                } else {
                    let old = std::mem::take(&mut self);
                    old.into_iter()
                        .zip(rhs..)
                        .take_while(|&(_, i)| i < BYTES * 8)
                        .filter(|&(bit, _)| bit)
                        .for_each(|(_, i)| {
                            self.set(i, true);
                        });
                }
            }
        }
        self
    }
}

impl<const BYTES: usize> From<BitSet<BYTES>> for [[bool; 8]; BYTES] {
    fn from(value: BitSet<BYTES>) -> Self {
        TryInto::<[_; BYTES]>::try_into(
            value
                .into_iter()
                .chunks(8)
                .into_iter()
                .map(|byte| {
                    TryInto::<[_; 8]>::try_into(byte.collect_vec())
                        .unwrap_or_else(|_| unreachable!())
                })
                .collect_vec(),
        )
        .unwrap_or_else(|_| unreachable!())
    }
}

/// a wrapper to allow iteration of `BitSet`
pub struct IterWrapper<const BYTES: usize> {
    set: BitSet<BYTES>,
    pos: usize,
    end: usize,
}
impl<const BYTES: usize> Iterator for IterWrapper<BYTES> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.end {
            return None;
        }
        self.pos += 1;
        Some(self.set.get(self.pos - 1))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.end - self.pos;
        (len, Some(len))
    }
}
impl<const BYTES: usize> ExactSizeIterator for IterWrapper<BYTES> {}
impl<const BYTES: usize> DoubleEndedIterator for IterWrapper<BYTES> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.pos >= self.end {
            return None;
        }
        self.end -= 1;
        Some(self.set.get(self.end))
    }
}

impl<const BYTES: usize> IntoIterator for BitSet<BYTES> {
    type Item = <IterWrapper<BYTES> as Iterator>::Item;

    type IntoIter = IterWrapper<BYTES>;

    fn into_iter(self) -> Self::IntoIter {
        IterWrapper {
            set: self,
            pos: 0,
            end: BYTES * 8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get() {
        let set = BitSet::<2>::from(0xF00F);
        for i in (0..4).chain(12..16) {
            assert!(set.get(i), "bit {i} wasn't set");
        }
        for i in 4..12 {
            assert!(!set.get(i), "bit {i} was set");
        }
        let set = BitSet::<2>::from(0x00FF);
        for i in 0..8 {
            assert!(set.get(i), "bit {i} wasn't set");
        }
        for i in 8..16 {
            assert!(!set.get(i), "bit {i} was set");
        }
    }

    #[test]
    fn set() {
        let mut set = BitSet::<2>::from(0b1010_1010_0000_0000);
        set.set(10, true);
        assert_eq!(BitSet::from(0b1010_1110_0000_0000), set);
        set.set(11, false);
        assert_eq!(BitSet::from(0b1010_0110_0000_0000), set);
    }

    #[test]
    fn union() {
        assert_eq!(
            BitSet::<1>::from(0b1010_1111),
            BitSet::from(0b1010_1010).union(&BitSet::from(0b0000_1111))
        );
    }

    #[test]
    fn shift() {
        assert_eq!(
            BitSet::<4>::from(0x03_02_01_00u32),
            BitSet::from(0x04_03_02_01u32) << 8
        );
        assert_eq!(
            BitSet::<2>::from(0b1000_0010_0000_0000),
            BitSet::from(0b1111_1111_1100_0001) << 9
        );
    }

    #[test]
    fn debug() {
        assert_eq!(
            "Bitset(0x00000000fedcba9876543210)",
            format!("{:?}", BitSet::from(0xfedc_ba98_7654_3210u128))
        );
        assert_eq!(
            "Bitset(0b0000010110101111)",
            format!("{:#?}", BitSet::from(0b0000_0101_1010_1111u16))
        );

        assert_eq!(
            "Bitset(00000000fedcba9876543210)",
            format!("{:x}", BitSet::from(0xfedc_ba98_7654_3210u128))
        );
        assert_eq!(
            "Bitset(00000000FEDCBA9876543210)",
            format!("{:X}", BitSet::from(0xfedc_ba98_7654_3210u128))
        );
        assert_eq!(
            "Bitset(0000010110101111)",
            format!("{:b}", BitSet::from(0b0000_0101_1010_1111u16))
        );

        assert_eq!(
            "Bitset(0x00000000fedcba9876543210)",
            format!("{:#x}", BitSet::from(0xfedc_ba98_7654_3210u128))
        );
        assert_eq!(
            "Bitset(0X00000000FEDCBA9876543210)",
            format!("{:#X}", BitSet::from(0xfedc_ba98_7654_3210u128))
        );
        assert_eq!(
            "Bitset(0b0000010110101111)",
            format!("{:#b}", BitSet::from(0b0000_0101_1010_1111u16))
        );
    }
}
