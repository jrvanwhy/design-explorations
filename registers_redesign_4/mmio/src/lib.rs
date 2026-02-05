use core::{mem::size_of, ptr::NonNull};
use registers::*;

/// A pointer to MMIO registers.
#[derive(Clone, Copy)]
pub struct MmioPointer(NonNull<()>);

impl DynAddress for MmioPointer {
    unsafe fn byte_add(self, offset: usize) -> MmioPointer {
        // Safety: The safety requirements of Address::byte_add require self + offset to remain
        // within this register span and not wrap.
        MmioPointer(unsafe { self.0.byte_add(offset) })
    }
}

impl<T: UIntLike> Bus<T> for MmioPointer {
    const PADDED_SIZE: usize = size_of::<T>();
}

impl<T: UIntLike> BusRead<T> for MmioPointer {
    unsafe fn read(self) -> T {
        let pointer = self.0.cast::<T>();
        // Safety: The caller has guaranteed that there is a register of type T at address `self`,
        // and has complied with any safety invariants that register has.
        unsafe { pointer.read_volatile() }
    }
}

impl<T: UIntLike> BusWrite<T> for MmioPointer {
    unsafe fn write(self, value: T) {
        let pointer = self.0.cast::<T>();
        // Safety: The caller has guaranteed that there is a register of type T at address `self`,
        // and has complied with any safety invariants that register has.
        unsafe { pointer.write_volatile(value) }
    }
}
