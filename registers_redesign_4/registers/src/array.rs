use crate::*;

/// Interface for an array of registers (or register blocks, or register arrays).
pub trait RegisterArray: Copy {
    /// The type of each element of this array.
    type Element: Copy;

    /// The number of elements of this array.
    const LEN: usize;

    /// Returns the `index`-th element of this array, or `None` if `index >= LEN`.
    fn get(self, index: usize) -> Option<Self::Element> {
        if index >= Self::LEN {
            return None;
        }
        Some(unsafe { self.get_unchecked(index) })
    }

    /// Returns the `index`-th element of this array.
    /// # Safety
    /// `index` must be less than `LEN`
    unsafe fn get_unchecked(self, index: usize) -> Self::Element {
        self.get(index).unwrap_or_else(|| {
            panic!(
                "get_unchecked called with out-of-bounds index {index}; len = {}",
                Self::LEN
            )
        })
    }
}

/// Interface for a register array with a size that exactly matches its indexing type (`u8` or
/// `u16`).
pub trait ExactIndexRegisterArray: RegisterArray {
    type Index;

    /// Returns the `index`-th element of this array.
    fn get(self, index: Self::Index) -> Self::Element;
}

/// Real implementation of RegisterArray.
// Safety invariant: `address` points to an array of `LEN` consecurity `Element` registers.
#[derive(Clone, Copy)]
pub struct RealRegisterArray<Element: Block, const LEN: usize> {
    address: Element::Address,
}

impl<Element: Block, const LEN: usize> RealRegisterArray<Element, LEN> {
    /// Constructs a new register array with the given address.
    /// # Safety
    /// `address` must point to an array of `LEN` registers, each of which has a layout
    /// corresponding to `Element`.
    pub unsafe fn new(address: Element::Address) -> RealRegisterArray<Element, LEN> {
        RealRegisterArray { address }
    }
}

impl<Element: Block, const LEN: usize> Block for RealRegisterArray<Element, LEN> {
    type Address = Element::Address;
    const SIZE: usize = LEN * Element::SIZE;

    unsafe fn new(address: Element::Address) -> RealRegisterArray<Element, LEN> {
        RealRegisterArray { address }
    }
}

impl<Element: Block, const LEN: usize> RegisterArray for RealRegisterArray<Element, LEN> {
    type Element = Element;
    const LEN: usize = LEN;

    fn get(self, index: usize) -> Option<Element> {
        if index >= Self::LEN {
            return None;
        }
        Some(unsafe { self.get_unchecked(index) })
    }

    unsafe fn get_unchecked(self, index: usize) -> Element {
        let offset = index * Element::SIZE;
        // Safety:
        // We know `address` points to an array of `LEN` `Element`s. The caller guaranteed that
        // `index <= LEN`, so index * Element::SIZE is within the array's bounds. That guarantees
        // that this offset falls within the bounds of a register block (as the array itself is a
        // register block).
        let address = unsafe { self.address.byte_add(offset) };
        // Safety: `address` was a correctly calculated index into the array, and we know the
        // element type is `Element`, so `address` points to an `Element`.
        unsafe { Element::new(address) }
    }
}

impl<Element: Block> ExactIndexRegisterArray
    for RealRegisterArray<Element, { u8::MAX as usize + 1 }>
{
    type Index = u8;
    fn get(self, index: u8) -> Self::Element {
        let offset = usize::from(index) * Element::SIZE;
        // Safety:
        // `address` points to an array of length u8::MAX + 1, so `index` cannot overflow the
        // array. The safety invariant for RealRegisterArray guarantees the element type is
        // Element, so `offset` was calculated correctly.
        let address = unsafe { self.address.byte_add(offset) };
        // Safety: `address` was a correctly calculated index into the array, and we know the
        // element type is `Element`, so `address` points to an `Element`.
        unsafe { Element::new(address) }
    }
}

impl<Element: Block> ExactIndexRegisterArray
    for RealRegisterArray<Element, { u16::MAX as usize + 1 }>
{
    type Index = u16;
    fn get(self, index: u16) -> Self::Element {
        let offset = usize::from(index) * Element::SIZE;
        // Safety:
        // `address` points to an array of length u16::MAX + 1, so `index` cannot overflow the
        // array. The safety invariant for RealRegisterArray guarantees the element type is
        // Element, so `offset` was calculated correctly.
        let address = unsafe { self.address.byte_add(offset) };
        // Safety: `address` was a correctly calculated index into the array, and we know the
        // element type is `Element`, so `address` points to an `Element`.
        unsafe { Element::new(address) }
    }
}
