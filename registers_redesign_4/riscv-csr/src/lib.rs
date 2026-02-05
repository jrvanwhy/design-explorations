use registers::*;

/// A CSR number.
#[derive(Clone, Copy)]
pub struct CsrNumber<const NUMBER: u16>;

impl<const NUMBER: u16> Bus<u32> for CsrNumber<NUMBER> {
    const PADDED_SIZE: usize = 4;
}

impl<const NUMBER: u16> Bus<usize> for CsrNumber<NUMBER> {
    const PADDED_SIZE: usize = 4;
}

impl<const NUMBER: u16> BusRead<u32> for CsrNumber<NUMBER> {
    unsafe fn read(self) -> u32 {
        todo!("Implemented in asm")
    }
}

impl<const NUMBER: u16> BusRead<usize> for CsrNumber<NUMBER> {
    unsafe fn read(self) -> usize {
        todo!("Implemented in asm")
    }
}

impl<const NUMBER: u16> BusWrite<u32> for CsrNumber<NUMBER> {
    unsafe fn write(self, _: u32) {
        todo!("Implemented in asm")
    }
}

impl<const NUMBER: u16> BusWrite<usize> for CsrNumber<NUMBER> {
    unsafe fn write(self, _: usize) {
        todo!("Implemented in asm")
    }
}
