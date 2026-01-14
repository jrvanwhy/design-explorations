use core::{marker::PhantomData, ptr::NonNull};

pub trait UIntLike {}
impl UIntLike for u8 {}
impl UIntLike for u16 {}
impl UIntLike for u32 {}
impl UIntLike for u64 {}
impl UIntLike for u128 {}
impl UIntLike for i8 {}
impl UIntLike for i16 {}
impl UIntLike for i32 {}
impl UIntLike for i64 {}
impl UIntLike for i128 {}

/// A Bus provides some way to access register values. Examples include: direct MMIO, LiteX (which
/// is memory-mapped but does not translate 1:1 to read_volatile/write_volatile), RISC-V CSRs, etc.
/// MMIO provides access to any UIntLike type, but other bus types may only provide access to
/// certain primitives. In that case, the bus will implement Bus<T> only for the corresponding T.
pub trait Bus<T: UIntLike>: Copy {
    const ADDRESS_SIZE: usize;
}

/// A block of registers. Every Real type implements this. This trait is used to construct the
/// register blocks, including sub-blocks for larger register blocks.
pub trait RealBlock: Copy {
    type Bus: Copy;
    /// Size this blocks occupies in the address space. Depends on Bus.
    const ADDRESS_SIZE: usize;

    /// Constructs a new register block.
    /// # Safety
    /// `pointer` must point to registers on Bus `bus` with the layout that correctly matches this
    /// type's definition.
    unsafe fn with_bus(pointer: NonNull<()>, bus: Self::Bus) -> Self;

    /// Constructs a new register block with a default Bus (e.g. an MMIO bus).
    /// # Safety
    /// `pointer` must point to registers on Bus `B` with the layout that correctly matches this
    /// type's definition.
    unsafe fn new(pointer: NonNull<()>) -> Self
    where
        Self::Bus: Default,
    {
        unsafe { Self::with_bus(pointer, Default::default()) }
    }
}

/// Interface for an array register.
// Note: We probably also want traits for infallible indexing (for registers arrays of exactly 2^8
// or 2^16 entries). Don't need to prototype here though, that's something to implement in the full
// version (if it doesn't work then we can just ignore it).
pub trait ArrayRegister: Copy {
    /// The type of an element of this array.
    type Element: Copy;
    const LEN: usize;
    fn get(self, index: usize) -> Option<Self::Element>;
}

/// Real implementation of ArrayRegister.
#[derive(Clone, Copy)]
pub struct RealArrayRegister<E: RealBlock, const LEN: usize> {
    bus: E::Bus,
    phantom: PhantomData<[E; LEN]>,
    pointer: NonNull<()>,
}

impl<E: RealBlock, const LEN: usize> ArrayRegister for RealArrayRegister<E, LEN> {
    type Element = E;
    const LEN: usize = LEN;

    fn get(self, index: usize) -> Option<E> {
        if index >= LEN {
            return None;
        }
        let base = self.pointer;
        let offset = index * E::ADDRESS_SIZE;
        Some(unsafe { E::with_bus(base.byte_add(offset), self.bus) })
    }
}

impl<E: RealBlock, const LEN: usize> RealBlock for RealArrayRegister<E, LEN> {
    type Bus = E::Bus;
    const ADDRESS_SIZE: usize = LEN * E::ADDRESS_SIZE;

    unsafe fn with_bus(pointer: NonNull<()>, bus: E::Bus) -> Self {
        RealArrayRegister {
            bus,
            phantom: PhantomData,
            pointer,
        }
    }
}

/// Inherent impl of the constuctors so callers don't have to use `<X as RealBlock>::new()` to
/// construct arrays.
impl<E: RealBlock, const LEN: usize> RealArrayRegister<E, LEN> {
    /// Constructs a new register array with a default Bus (e.g. an MMIO bus).
    /// # Safety
    /// `pointer` must point to a register array of type E and length LEN on Bus `B`.
    pub unsafe fn new(pointer: NonNull<()>) -> Self
    where
        E::Bus: Default,
    {
        // Safety: Same invariants as this function.
        unsafe { <Self as RealBlock>::new(pointer) }
    }

    /// Constructs a new register array.
    /// # Safety
    /// `pointer` must point to a register array of type E and length LEN on Bus `bus`.
    pub unsafe fn with_bus(pointer: NonNull<()>, bus: E::Bus) -> Self {
        RealArrayRegister {
            bus,
            phantom: PhantomData,
            pointer,
        }
    }
}

/// Type implemented by all registers.
pub trait Register: Copy {
    type DataType;
}
