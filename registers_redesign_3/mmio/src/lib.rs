// MMIO operations would be defined in the core tock-register crate, but other types (such as the
// LiteX bus and RISC-V CSRs) should be defined in separate crates. To test that, this workspace
// implements the MMIO operations in a separate crate.

use core::mem::size_of;
use core::ptr::{read_volatile, write_volatile};
use registers::*;

/// A bus that performs direct MMIO operations. This does not support LiteX.
#[derive(Clone, Copy, Default)]
pub struct Mmio;

impl Bus for Mmio {
    type Address = core::ptr::NonNull<()>;
}
impl<T: UIntLike> BusValue<T> for Mmio {
    const ADDRESS_SIZE: usize = size_of::<T>();
}

/// A Bus that implements BusRead<T> can support Read implementations with DataType T. Other crates
/// (e.g. LiteX registers) can implement this on their own buses so that Read works with them as
/// well.
pub trait BusRead<T: UIntLike>: BusValue<T> {
    /// # Safety
    /// There must be a register of type T at `pointer`, and if the register itself has safety
    /// invariants (i.e. it is `UnsafeRead`) the caller must satisfy those.
    unsafe fn read(self, pointer: *const T) -> T;
}

impl<T: UIntLike> BusRead<T> for Mmio {
    unsafe fn read(self, pointer: *const T) -> T {
        unsafe { read_volatile(pointer) }
    }
}

/// A Bus that implements BusWrite<T> can support Write implementations with DataType T. Other
/// crates (e.g. LiteX registers) can implement this on their own buses so that Write works with
/// them as well.
pub trait BusWrite<T: UIntLike>: BusValue<T> {
    /// # Safety
    /// There must be a register of type T at `pointer`, and if the register itself has safety
    /// invariants (i.e. it is `UnsafeWrite`) the caller must satisfy those.
    unsafe fn write(self, pointer: *mut T, value: T);
}

impl<T: UIntLike> BusWrite<T> for Mmio {
    unsafe fn write(self, pointer: *mut T, value: T) {
        unsafe { write_volatile(pointer, value) }
    }
}

/// A register that can be read.
pub trait Read: Register {
    fn read(self) -> Self::DataType;
}

/// The macro that goes along with the Read trait. We don't expect this macro to be used by
/// tock_register's users, instead it is invoked by the generated code.
#[macro_export]
macro_rules! Read {
    // Provides a real implementation of the trait. The trailing $rest argument is for future
    // compatibility: it allows the procedural macro to pass additional arguments in the future
    // without breaking compatibility with this implementation of Read!.
    (real_impl, $real_type:ident, $datatype:ty, $($rest:tt)*) => {
        impl<B: Bus + $crate::BusRead<$datatype>> $crate::Read for $real_type<B> {
            fn read(self) -> $datatype {
                unsafe {
                    self.bus
                        .read(self.pointer.cast::<$datatype>().as_ptr().cast_const())
                }
            }
        }
    };
}

/// A register that can be written.
pub trait Write: Register {
    fn write(self, value: Self::DataType);
}

/// The macro that goes along with the Write trait. We don't expect this macro to be used by
/// tock_register's users, instead it is invoked by the generated code.
#[macro_export]
macro_rules! Write {
    // Provides a real implementation of the trait. The trailing $rest argument is for future
    // compatibility: it allows the procedural macro to pass additional arguments in the future
    // without breaking compatibility with this implementation of Read!.
    (real_impl, $real_type:ident, $datatype:ty, $($rest:tt)*) => {
        impl<B: Bus + $crate::BusWrite<$datatype>> $crate::Write for $real_type<B> {
            fn write(self, value: $datatype) {
                unsafe {
                    self.bus
                        .write(self.pointer.cast::<$datatype>().as_ptr(), value)
                }
            }
        }
    };
}
