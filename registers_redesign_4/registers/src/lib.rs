//! # Addresses and Buses
//!
//! tock-registers supports many different types of registers, including:
//! 1. MMIO registers
//! 2. Several types of LiteX registers (these are MMIO, but have padding within the registers that
//!    depends on the chip's configuration and the register's data type)
//! 3. x86 port IO
//!
//! tock-registers calls each of these classes a "bus", as the mechanism used to access the
//! registers is different. The mechanism in which a register is addressed depends on the bus, so
//! each register address type corresponds to the bus. In other words:
//!
//! * The *type* of the address pointer corresponds to the bus.
//! * The *value* of the address pointer corresponds to the register on that bus.
//!
//! Therefore, all real register types ("real" here means it's not a fake/mock/stub for testing)
//! have a generic argument for their address type. That address type implements the `Address`
//! trait. In addition, address types should implement [`Bus<T>`] for every type `T` that they support.

mod array;
mod ops;

pub use array::*;
pub use ops::*;

// Things copied from tock_registers just so the rest of the code can compile.
pub trait UIntLike {}
impl UIntLike for u8 {}
impl UIntLike for u16 {}
impl UIntLike for u32 {}
impl UIntLike for u64 {}
impl UIntLike for u128 {}
impl UIntLike for usize {}
pub trait RegisterLongName {}
impl RegisterLongName for () {}
pub struct Aliased<T: UIntLike, R: RegisterLongName = (), W: RegisterLongName = ()> {
    _value: core::cell::UnsafeCell<T>,
    associated_register: core::marker::PhantomData<(R, W)>,
}

/// Trait for addresses that can be offset without changing their type. See the module-level docs
/// for more information on address types.
pub trait Address: Copy {
    /// Adds the given offset to this address, returning an address pointing to a later location on
    /// the bus.
    /// # Safety
    /// `self` must point into a register span on the correct bus (based on `Self`). The entire
    /// range from `self` to the result (inclusive) must be in bounds of that register span. In
    /// particular, this must not "wrap around" the address space.
    unsafe fn byte_add(self, offset: usize) -> Self;
}

/// Address types should implement `Bus<T>` for each primitive type `T` that they support.
pub trait Bus<T: UIntLike>: Address {
    /// The size that a value of type T takes in this bus' address space. This exists because LiteX
    /// buses have intra-register padding for some types.
    const PADDED_SIZE: usize;
}

/// An accessor for a block of registers. Every Real type implements this. This trait is used to
/// construct the register blocks, including sub-blocks for larger register blocks and elements of
/// arrays.
pub trait Block: Copy {
    type Address: Address;
    /// Size this blocks occupies in the address space. Depends on the address type.
    const SIZE: usize;

    /// Constructs an accessor for a register block.
    /// # Safety
    /// `address` must point to registers on the bus corresponding to Self::Address with the layout
    /// that correctly matches this type's definition.
    unsafe fn new(address: Self::Address) -> Self;
}

/// Type implemented by all registers.
pub trait Register: Copy {
    /// The data type used for this register in the type specification. For register arrays, this
    /// is the innermost data type (i.e. it is not an array).
    type DataType: DataType;
}

/// Trait used to retrieve information about a register from the type given in its specification.
pub trait DataType {
    /// The register's value type. This is the type passed over the bus when accessing the
    /// register.
    type Value: UIntLike;

    /// The bitfield used when data is read from this register.
    type Read: RegisterLongName;

    /// The bitfield used when data is written to this register.
    type Write: RegisterLongName;
}

impl<U: UIntLike> DataType for U {
    type Value = U;
    type Read = ();
    type Write = ();
}

impl<T: UIntLike, R: RegisterLongName, W: RegisterLongName> DataType for Aliased<T, R, W> {
    type Value = T;
    type Read = R;
    type Write = W;
}

// register_bitfields! would be modified to implement DataType for every register type it outputs,
// with a Value of $valtype and Read/Write types of Self.
