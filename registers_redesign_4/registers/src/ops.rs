//! In general, operations can be defined by external crates. However, because Read and Write show
//! up for multiple bus types (MMIO, LiteX, CSRs, port IO), they're in the common tock-registers
//! crate.

use crate::*;

/// A register that can be read.
pub trait Read: Register {
    fn read(self) -> <Self::DataType as DataType>::Value;
}

/// The macro that goes along with the Read trait. We don't expect this macro to be used by
/// tock_register's users, instead it is invoked by the generated code.
#[macro_export]
macro_rules! Read {
    // Provides a real implementation of the trait. The trailing $rest argument is for future
    // compatibility: it allows the procedural macro to pass additional arguments in the future
    // without breaking compatibility with this implementation of Read!.
    (real_impl, $name:ident, $datatype:ty, $($rest:tt)*) => {
        impl<B: Bus + $crate::BusRead<<$datatype as $crate::DataType>::Value>> $crate::Read
            for $name<B>
        {
            fn read(self) -> <$datatype as $crate::DataType>::Value {
                // Safety: The caller assured this GenericReal points at a register on bus B with
                // value type $datatype::Value that is safe to read.
                unsafe { self.0.read() }
            }
        }
    };
}

/// A Bus that implements BusRead<T> can support Read implementations with DataType T. Other crates
/// (e.g. LiteX registers) can implement this on their own buses so that Read works with them as
/// well.
pub trait BusRead<T: UIntLike>: Bus<T> {
    /// # Safety
    /// There must be a register of type T at `pointer`, and if the register itself has safety
    /// invariants (i.e. it is `UnsafeRead`) the caller must satisfy those.
    unsafe fn read(self) -> T;
}

/// A register that can be written.
pub trait Write: Register {
    fn write(self, value: <Self::DataType as DataType>::Value);
}

/// The macro that goes along with the Write trait. We don't expect this macro to be used by
/// tock_register's users, instead it is invoked by the generated code.
#[macro_export]
macro_rules! Write {
    // Provides a real implementation of the trait. The trailing $rest argument is for future
    // compatibility: it allows the procedural macro to pass additional arguments in the future
    // without breaking compatibility with this implementation of Read!.
    (real_impl, $name:ident, $datatype:ty, $($rest:tt)*) => {
        impl<B: Bus + $crate::BusWrite<<$datatype as $crate::DataType>::Value>> $crate::Write
            for $name<B>
        {
            fn write(self, value: <$datatype as $crate::DataType>::Value) {
                // Safety: The caller assured this GenericReal points at a register on bus B with
                // value type $datatype::Value that is safe to write.
                unsafe { self.0.write(value) }
            }
        }
    };
}

/// A Bus that implements BusWrite<T> can support Write implementations with DataType T. Other
/// crates (e.g. LiteX registers) can implement this on their own buses so that Write works with
/// them as well.
pub trait BusWrite<T: UIntLike>: Bus<T> {
    /// # Safety
    /// There must be a register of type T at `pointer`, and if the register itself has safety
    /// invariants (i.e. it is `UnsafeWrite`) the caller must satisfy those.
    unsafe fn write(self, value: T);
}

// We would also have UnsafeRead and UnsafeWrite operations, and furthermore those operations would
// be implemented for everything that implements Read or Write (correspondingly).
