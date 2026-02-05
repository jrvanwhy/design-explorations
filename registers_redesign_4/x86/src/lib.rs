use registers::*;

/// An x86 port IO address.
#[derive(Clone, Copy)]
pub struct IoPort(u16);

impl Address for IoPort {
    unsafe fn byte_add(self, offset: usize) -> IoPort {
        // Theoretically we could use self.0.unchecked_add(offset.try_into().unchecked_unwrap())
        // but safe math is probably the way to go here.
        IoPort(self.0 + offset as u16)
    }
}

impl Bus<u8> for IoPort {
    const PADDED_SIZE: usize = 1;
}
impl BusRead<u8> for IoPort {
    unsafe fn read(self) -> u8 {
        todo!()
    }
}
impl BusWrite<u8> for IoPort {
    unsafe fn write(self, _: u8) {
        todo!()
    }
}

impl Bus<u16> for IoPort {
    const PADDED_SIZE: usize = 2;
}
impl BusRead<u16> for IoPort {
    unsafe fn read(self) -> u16 {
        todo!()
    }
}
impl BusWrite<u16> for IoPort {
    unsafe fn write(self, _: u16) {
        todo!()
    }
}

impl Bus<u32> for IoPort {
    const PADDED_SIZE: usize = 4;
}
impl BusRead<u32> for IoPort {
    unsafe fn read(self) -> u32 {
        todo!()
    }
}
impl BusWrite<u32> for IoPort {
    unsafe fn write(self, _: u32) {
        todo!()
    }
}

impl Bus<usize> for IoPort {
    const PADDED_SIZE: usize = 4;
}
impl BusRead<usize> for IoPort {
    unsafe fn read(self) -> usize {
        todo!()
    }
}
impl BusWrite<usize> for IoPort {
    unsafe fn write(self, _: usize) {
        todo!()
    }
}
