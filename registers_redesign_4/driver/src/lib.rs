use mmio::*;
use registers::*;
use x86::*;

// tock_registers::registers! {
//     // An individual register, which can be re-used later.
//     // Roughly equivalent to (from tock registers v1):
//     //     type Status = ReadOnly<u8>;
//     pub status: u8 { Read },
// }

pub mod status {
    use super::*;

    pub trait Interface: Register<DataType = u8> + Read {}

    pub trait Bus: Copy + sealed::Bus + registers::Bus<u8> {}
    impl Bus for Mmio {}
    mod sealed {
        use crate::*;
        pub trait Bus {}
        impl Bus for Mmio {}
    }

    #[derive(Clone, Copy)]
    pub struct Real<B: Bus>(B);

    impl<B: Bus> Interface for Real<B> where Self: Register<DataType = u8> + Read {}

    // Register blocks use Block to construct register types. Most users of tock_registers
    // won't use Block directly; they will use the inherent methods instead.
    impl<B: Bus> Block for Real<B> {
        type Address = B;
        const SIZE: usize = <B as registers::Bus<u8>>::PADDED_SIZE;
        unsafe fn new(address: B) -> Self {
            Real(address)
        }
    }

    /// Public constructors for this Real. Note that this is omitted in the other examples
    /// and README.md (mostly to reduce code size, partially because I wrote this last and don't
    /// want to update everything).
    impl<B: Bus> Real<B> {
        /// Constructs a new register with a default Bus (e.g. an MMIO bus).
        /// # Safety
        /// `pointer` must point to a register of this type on bus `B`.
        pub unsafe fn new(address: B) -> Self {
            // Safety: Same invariants
            unsafe { <Self as Block>::new(address) }
        }
    }

    impl<B: Bus> Register for Real<B> {
        type DataType = u8;
    }

    Read!(real_impl, Real, u8,);
}

// tock_registers::registers! {
//     #[buses(IoPort)]
//     pub ports {
//         0x0 => control: u8 { Read },
//     }
// }

pub mod ports {
    use super::*;
    pub trait Interface: Register<DataType = u8> + Read {}
    pub trait Bus: Copy + sealed::Bus + registers::Bus<<u8 as DataType>::Value> {}
    impl Bus for IoPort {}
    mod sealed {
        use crate::*;
        pub trait Bus {}
        impl Bus for IoPort {}
    }
    #[derive(Clone, Copy)]
    pub struct Real<B: Bus>(B);
    impl<B: Bus> Interface for Real<B> where Self: Register<DataType = u8> + Read {}
    impl<B: Bus> Block for Real<B> {
        type Address = B;
        const SIZE: usize = <B as registers::Bus<<u8 as DataType>::Value>>::PADDED_SIZE;
        unsafe fn new(address: B) -> Self {
            Real(address)
        }
    }
    impl<B: Bus> Real<B> {
        /// # Safety
        /// Don't mess this up (this comment to make Clippy happy).
        pub unsafe fn new(address: B) -> Self {
            Real(address)
        }
    }
    impl<B: Bus> Register for Real<B> {
        type DataType = u8;
    }
    Read!(real_impl, Real, u8,);
}

// tock_registers::registers! {
//     // Arrays can be created by referring to other registers too.
//     // Roughly equivalent to (from tock registers v1):
//     //     type StatusArray = [Status; 4];
//     pub status_array: [status; 4],
// }

pub mod status_array {
    use super::*;

    // RegisterArray is a trait defined in tock_registers.
    pub trait Interface: RegisterArray<Element: status::Interface> {}

    pub trait Bus: Copy + sealed::Bus + status::Bus {}
    impl Bus for Mmio {}
    mod sealed {
        use crate::*;
        pub trait Bus {}
        impl Bus for Mmio {}
    }

    // Array registers do not require emitting a new struct, as they can be implemented as a
    // generic type in tock_registers.
    pub type Real<B> = RealRegisterArray<status::Real<B>, 4>;

    impl<B: Bus> Interface for Real<B> where Self: RegisterArray<Element: status::Interface> {}
}

// tock_registers::registers! {
//     pub foo {
//         // status and status array refer to the previous two register types.
//         0x0 => status: status,
//         0x1 => status_array: status_array,
//
//         // An inline register definition.
//         0x5 => control: u8 { Read + Write },
//     }
// }

pub mod foo {
    #![allow(non_camel_case_types)]
    use super::*;

    pub trait Interface: Copy {
        type status: status::Interface;
        fn status(self) -> Self::status;

        type status_array: status_array::Interface;
        fn status_array(self) -> Self::status_array;

        type control: Register<DataType = u8> + Read + Write;
        fn control(self) -> Self::control;
    }

    #[allow(non_upper_case_globals)]
    pub trait Bus: Copy + sealed::Bus + status::Bus + status_array::Bus {
        // These values are the offsets of each field.
        const status: usize;
        const status_array: usize;
        const control: usize;
    }
    impl Bus for Mmio {
        const status: usize = 0x0;
        const status_array: usize = const {
            assert!(Self::status + status::Real::<Self>::SIZE == 0x1);
            0x1
        };
        const control: usize = const {
            assert!(Self::status_array + status_array::Real::<Self>::SIZE == 0x5);
            0x5
        };
    }
    mod sealed {
        use crate::*;
        pub trait Bus {}
        impl Bus for Mmio {}
    }

    #[derive(Clone, Copy)]
    pub struct Real<B: Bus> {
        address: B,
    }

    // To avoid name conflicts, the real implementations of inline-defined registers have real_
    // prepended to them.
    #[derive(Clone, Copy)]
    pub struct real_control<B: Bus>(B);

    impl<B: Bus> Block for real_control<B> {
        type Address = B;
        const SIZE: usize = <B as registers::Bus<u8>>::PADDED_SIZE;
        unsafe fn new(address: B) -> Self {
            Self(address)
        }
    }

    impl<B: Bus> Register for real_control<B> {
        type DataType = u8;
    }

    Read!(real_impl, real_control, u8,);
    Write!(real_impl, real_control, u8,);

    impl<B: Bus> Interface for Real<B>
    where
        status::Real<B>: status::Interface,
        status_array::Real<B>: status_array::Interface,
        real_control<B>: Register<DataType = u8> + Read + Write,
    {
        type status = status::Real<B>;
        fn status(self) -> Self::status {
            unsafe { Self::status::new(self.address.byte_add(B::status)) }
        }
        type status_array = status_array::Real<B>;
        fn status_array(self) -> Self::status_array {
            unsafe { Self::status_array::new(self.address.byte_add(B::status_array)) }
        }
        type control = real_control<B>;
        fn control(self) -> Self::control {
            unsafe { Self::control::new(self.address.byte_add(B::control)) }
        }
    }

    impl<B: Bus> Block for Real<B> {
        type Address = B;
        const SIZE: usize = B::control + <B as registers::Bus<u8>>::PADDED_SIZE;
        unsafe fn new(address: B) -> Self {
            Self { address }
        }
    }
}

// tock_registers::registers! {
//     pub nested_array {
//         0x0 => array: [[u8; 2]; 3] { Read },
//     }
// }

pub mod nested_array {
    use super::*;

    pub trait Interface:
        RegisterArray<Element: RegisterArray<Element: Register<DataType = u8> + Read>>
    {
    }

    pub trait Bus: Copy + sealed::Bus + registers::Bus<u8> {}
    impl Bus for Mmio {}
    mod sealed {
        use crate::*;
        pub trait Bus {}
        impl Bus for Mmio {}
    }

    // For an inline-defined array, we need to generate a separate type for the inner value.
    #[derive(Clone, Copy)]
    pub struct Element<B: Bus>(B);

    impl<B: Bus> Block for Element<B> {
        type Address = B;
        const SIZE: usize = <B as registers::Bus<u8>>::PADDED_SIZE;
        unsafe fn new(address: B) -> Self {
            Self(address)
        }
    }

    impl<B: Bus> Register for Element<B> {
        type DataType = u8;
    }

    Read!(real_impl, Element, u8,);

    pub type Real<B> = RealRegisterArray<RealRegisterArray<Element<B>, 2>, 3>;
}

#[cfg(test)]
mod tests {
    use crate::*;
    use core::{cell::Cell, ptr::NonNull};

    #[test]
    fn fake_foo() {
        use crate::foo::Interface;
        // Implements a fake Foo peripheral. This peripheral has he following behavior:
        // 1. The control register acts like normal memory (it holds whatever value is written into
        //    it).
        // 2. It contains a u8 counter. Reading from `status` returns this counter.
        // 3. Reading from `status_array` increments the counter by `control * (index + 1)`,
        //    wrapping.
        #[derive(Default)]
        struct Fake {
            status: Cell<u8>,
            control: Cell<u8>,
        }
        impl<'f> Interface for &'f Fake {
            type status = FakeStatus<'f>;
            fn status(self) -> FakeStatus<'f> {
                FakeStatus {
                    fake: self,
                    multiplier: 0,
                }
            }
            type status_array = FakeStatusArray<'f>;
            fn status_array(self) -> FakeStatusArray<'f> {
                FakeStatusArray(self)
            }
            type control = FakeControl<'f>;
            fn control(self) -> FakeControl<'f> {
                FakeControl(self)
            }
        }
        #[derive(Clone, Copy)]
        struct FakeStatus<'f> {
            fake: &'f Fake,
            multiplier: u8,
        }
        impl status::Interface for FakeStatus<'_> {}
        impl Read for FakeStatus<'_> {
            fn read(self) -> u8 {
                let value = self
                    .fake
                    .status
                    .get()
                    .wrapping_add(self.fake.control.get() * self.multiplier);
                self.fake.status.set(value);
                value
            }
        }
        impl Register for FakeStatus<'_> {
            type DataType = u8;
        }
        #[derive(Clone, Copy)]
        struct FakeStatusArray<'f>(&'f Fake);
        impl<'f> RegisterArray for FakeStatusArray<'f> {
            type Element = FakeStatus<'f>;
            const LEN: usize = 4;
            fn get(self, index: usize) -> Option<FakeStatus<'f>> {
                if index >= 4 {
                    return None;
                }
                Some(FakeStatus {
                    fake: self.0,
                    multiplier: (index + 1).try_into().unwrap(),
                })
            }
        }
        impl status_array::Interface for FakeStatusArray<'_> {}
        #[derive(Clone, Copy)]
        struct FakeControl<'f>(&'f Fake);
        impl Read for FakeControl<'_> {
            fn read(self) -> u8 {
                self.0.control.get()
            }
        }
        impl Write for FakeControl<'_> {
            fn write(self, value: u8) {
                self.0.control.set(value);
            }
        }
        impl Register for FakeControl<'_> {
            type DataType = u8;
        }
        let foo = Fake::default();
        assert_eq!(foo.control().read(), 0);
        assert_eq!(foo.status().read(), 0);
        assert_eq!(foo.status_array().get(0).unwrap().read(), 0);
        assert_eq!(foo.status_array().get(1).unwrap().read(), 0);
        assert_eq!(foo.status_array().get(2).unwrap().read(), 0);
        assert_eq!(foo.status_array().get(3).unwrap().read(), 0);
        assert!(foo.status_array().get(4).is_none());
        foo.control().write(1);
        assert_eq!(foo.control().read(), 1);
        assert_eq!(foo.status().read(), 0);
        assert_eq!(foo.status_array().get(0).unwrap().read(), 1);
        assert_eq!(foo.status_array().get(1).unwrap().read(), 3);
        assert_eq!(foo.status_array().get(2).unwrap().read(), 6);
        assert_eq!(foo.status_array().get(3).unwrap().read(), 10);
        assert!(foo.status_array().get(4).is_none());
        foo.control().write(2);
        assert_eq!(foo.control().read(), 2);
        assert_eq!(foo.status().read(), 10);
        assert_eq!(foo.status_array().get(0).unwrap().read(), 12);
        assert_eq!(foo.status_array().get(1).unwrap().read(), 16);
        assert_eq!(foo.status_array().get(2).unwrap().read(), 22);
        assert_eq!(foo.status_array().get(3).unwrap().read(), 30);
        assert!(foo.status_array().get(4).is_none());
    }

    #[test]
    fn foo_interface_impl() {
        fn needs_interface<T: foo::Interface>(_: T) {}
        needs_interface(unsafe { foo::Real::new(Mmio(NonNull::dangling())) })
    }

    #[test]
    fn nested_array_read() {
        let mut array = [1u8, 2, 3, 4, 5, 6];
        let real = unsafe {
            <nested_array::Real<Mmio> as Block>::new(Mmio(NonNull::new_unchecked(
                (&raw mut array).cast(),
            )))
        };
        assert_eq!(real.get(0).unwrap().get(0).unwrap().read(), 1);
        assert_eq!(real.get(0).unwrap().get(1).unwrap().read(), 2);
        assert!(real.get(0).unwrap().get(2).is_none());
        assert_eq!(real.get(1).unwrap().get(0).unwrap().read(), 3);
        assert_eq!(real.get(1).unwrap().get(1).unwrap().read(), 4);
        assert!(real.get(1).unwrap().get(2).is_none());
        assert_eq!(real.get(2).unwrap().get(0).unwrap().read(), 5);
        assert_eq!(real.get(2).unwrap().get(1).unwrap().read(), 6);
        assert!(real.get(2).unwrap().get(2).is_none());
        assert!(real.get(3).is_none());
    }

    #[test]
    fn status_array_read() {
        let mut statuses = [1u8, 2, 3, 4];
        let real = unsafe {
            <status_array::Real<Mmio> as Block>::new(Mmio(NonNull::new_unchecked(
                (&raw mut statuses).cast(),
            )))
        };
        assert_eq!(real.get(0).unwrap().read(), 1);
        assert_eq!(real.get(1).unwrap().read(), 2);
        assert_eq!(real.get(2).unwrap().read(), 3);
        assert_eq!(real.get(3).unwrap().read(), 4);
    }
}
