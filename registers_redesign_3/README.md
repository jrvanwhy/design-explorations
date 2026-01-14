# Tock-registers redesign idea

This is a design concept for tock-registers that:

1. Fixes the reference-to-register soundness issue
1. Makes drivers unit-testable
1. Retains the ability for crates external to tock-registers to define new
   register types (e.g. RISC-V CSRs).
1. Supports LiteX registers (via the same mechanism as new register types).
1. Allows for conditionally-compiled registers.
1. Allows larger register blocks to be built out of smaller register blocks.

## Declaration syntax

First, an example of the declaration syntax, showing what this design should be
able to do:

```rust
use tock_registers::registers;

registers! {
    // An individual register, which can be re-used later.
    // Roughly equivalent to (from tock registers v1):
    //     type Status = ReadOnly<u8>;
    status: u8 { Read },

    // A register array, which can be re-used later.
    // Roughly equivalent to (from tock registers v1):
    //     type Buttons = [ReadOnly<u8>; 4];
    buttons: [u8; 4] { Read },

    // Arrays can be created by referring to other registers too.
    // Roughly equivalent to (from tock registers v1):
    //     type StatusArray = [Status; 4];
    status_array: [status; 4],

    // A register block. This is the equivalent to register_structs!
    simple_foo {
        // This defines a field called simple_status, whose type is defined
        // above as `status` (i.e. this is a read-only u8).
        0x0 => simple_status: status,

        // This defines a field called simple_buttons, whose type is the
        // register array `buttons`.
        0x1 => simple_buttons: buttons,

        // This defines a register field inline. Most registers will be defined
        // this way.
        0x5 => control: u16 { Read, Write },
    },

    // A larger register block. This register block contains an instance of a
    // simple_foo inside of itself.
    complex_foo {
        // The nested simple_foo
        0x0 => nested_foo: simple_foo,

        // An array of status registers.
        0x7 => status_array: [status; 2],
    },

    // Arrays can be created out of register blocks as well.
    many_simple_foos: [simple_foo; 8],

    // All of the above are MMIO-only (and do not support LiteX). Types that
    // want to support other uses need to explicitly declare what use cases they
    // support:
    #[bus_adapters(litex_registers::C8B32, litex_registers::C32B32)]
    litex_foo {
        0x0 => control: u16 { Read, Write },
        // Different offsets for different bus adapters.
        [0x8, 0x4] => status: u8 { Read },
    },

    #[bus_adapters(riscv_csrs::CSR)]
    csr_foo {
        // Different operation names are used here, because CSRs support
        // different operations than MMIO registers. For example, they have
        // read_and_set_bits and read_and_clear_bits operations. These operation
        // names are traits, so the CSR traits are separate from the MMIO
        // traits.
        0x0 => example_csr: u32 { CsrRead, CsrWrite },
    },

    // Oh, and to keep the example's size limited, I don't show this, but:
    // types should be able to refer to types defined in other crates/modules.
    // I.e. simple_foo could be defined in one crate, which the crate that
    // defines complex_foo depends on.
}
```

## Module interface

As shown above, register arrays and blocks should be able to nest arbitrarily.
To support that, they all need to have the same interface (I use "interface"
because it doesn't have an existing meaning in Rust). However, that interface
needs to contain both traits and types, so the interface is a standardized
module layout. That is why the register and peripheral blocks are in snake_case:
they turn into module names rather than struct names in the generated code.

The standardized module layout is:

```rust
// The module name matches the register/array/block name from the definition.
mod foo {
    // Names from the surrounding module are brought in by wildcard, so that
    // callers can refer to types and modules by relative path.
    use super::*;

    // Trait with the public interface for the register/array/block. This
    // interface can be implemented by fakes to support unit tests.
    pub trait Interface: /* ... */ { /* ... */ }

    // A trait defining which buses this register/array/blocks supports (MMIO is
    // a bus, each LiteX layout is a bus, CSRs is a bus, etc). This trait is
    // sealed; it is ONLY defined for the type(s) from `#[bus_adapters]`.
    pub trait Bus: /* ... */ { /* ... */ }

    // The type that implements Interface by calling out to real hardware (via
    // MMIO or otherwise).
    #[derive(Clone, Copy)]
    pub struct GenericReal<B> {
        bus: B,
        pointer: NonNull<()>,
    }

    // There are a lot of other trait impls involved, but this is the key one
    // for callers:
    impl<B: Bus> Interface for GenericReal<B> where /* ... */ { /* ... */ }

    // Convenience alias. If no `#[bus_adapters]` was specified, this will not
    // have a generic argument. If `#[bus_adapters]` was specified, this is just
    // an alias for GenericReal
    pub type Real = GenericReal<tock_registers::Mmio>;
}
```

## Basic register definition

The simplest example we have is a single register defined on its own.

```rust
tock_registers::registers! {
    // An individual register, which can be re-used later.
    // Roughly equivalent to (from tock registers v1):
    //     type Status = ReadOnly<u8>;
    pub status: u8 { Read },
}
```

expands to:

```rust
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
    pub struct GenericReal<B> {
        bus: B,
        pointer: core::ptr::NonNull<()>,
    }

    impl<B: Bus> Interface for GenericReal<B> where Self: Register<DataType=u8> + Read {}

    // RealBlock is a trait used to embed this register inside a larger register
    // array or register blocks. It's an internal trait of tock_registers; users
    // shouldn't have to interact with it directly.
    impl<B: Bus> RealBlock for GenericReal<B> {
        type Bus = B;
        const ADDRESS_SIZE: usize = <B as registers::Bus<u8>>::ADDRESS_SIZE;
        unsafe fn with_bus(pointer: core::ptr::NonNull<()>, bus: B) -> Self {
            Self { bus, pointer }
        }
    }

    impl<B: Bus> Register for GenericReal<B> {
        type DataType = u8;
    }

    // Each operation trait has a macro with the same name and path. The
    // generated code calls into that macro to have the operation implemented.
    Read!(real_impl, u8,);

    pub type Real = GenericReal<Mmio>;
}
```

## Array definition

Next, lets take the `static` register from the previous definition and embed it
into an array (by reference).

```rust
tock_registers::registers! {
    // Arrays can be created by referring to other registers too.
    // Roughly equivalent to (from tock registers v1):
    //     type StatusArray = [Status; 4];
    pub status_array: [status; 4],
}
```

expands to:

```rust
pub mod status_array {
    use super::*;

    // ArrayRegister is a trait defined in tock_registers.
    pub trait Interface: ArrayRegister<Element: status::Interface> {}

    pub trait Bus: Copy + sealed::Bus + status::Bus {}
    impl Bus for Mmio {}
    mod sealed {
        use crate::*;
        pub trait Bus {}
        impl Bus for Mmio {}
    }

    // Array registers do not require emitting a new struct, as they can be implemented as a
    // generic type in tock_registers.
    pub type GenericReal<B> = RealArrayRegister<status::GenericReal<B>, 4>;

    impl<B: Bus> Interface for GenericReal<B> where Self: ArrayRegister<Element: status::Interface> {}

    pub type Real = GenericReal<Mmio>;
}
```

## Register block

Stepping up the complexity a bit, lets create a register block that combines
both of the above registers and adds a new register with an inline definition
(note that inline definitions will be the most common case in practice).

```rust
tock_registers::registers! {
    pub foo {
        // status and status array refer to the previous two register types.
        0x0 => status: status,
        0x1 => status_array: status_array,

        // An inline register definition.
        0x5 => control: u8 { Read + Write },
    }
}
```

expands to:

```Rust
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
            assert!(Self::status + status::GenericReal::<Self>::ADDRESS_SIZE == 0x1);
            0x1
        };
        const control: usize = const {
            assert!(Self::status_array + status_array::GenericReal::<Self>::ADDRESS_SIZE == 0x5);
            0x5
        };
    }
    mod sealed {
        use crate::*;
        pub trait Bus {}
        impl Bus for Mmio {}
    }

    #[derive(Clone, Copy)]
    pub struct GenericReal<B> {
        bus: B,
        pointer: core::ptr::NonNull<()>,
    }

    // To avoid name conflicts, the real implementations of inline-defined registers have real_
    // prepended to them.
    #[derive(Clone, Copy)]
    pub struct real_control<B> {
        bus: B,
        pointer: core::ptr::NonNull<()>,
    }

    impl<B: Bus> RealBlock for real_control<B> {
        type Bus = B;
        const ADDRESS_SIZE: usize = <B as registers::Bus<u8>>::ADDRESS_SIZE;
        unsafe fn with_bus(pointer: core::ptr::NonNull<()>, bus: B) -> Self {
            Self { bus, pointer }
        }
    }

    impl<B: Bus> Register for real_control<B> {
        type DataType = u8;
    }

    Read!(real_impl, real_control, u8,);
    Write!(real_impl, real_control, u8,);

    impl<B: Bus> Interface for GenericReal<B>
    where
        status::GenericReal<B>: status::Interface,
        status_array::GenericReal<B>: status_array::Interface,
        real_control<B>: Register<DataType = u8> + Read + Write,
    {
        type status = status::GenericReal<B>;
        fn status(self) -> Self::status {
            unsafe { Self::status::with_bus(self.pointer.byte_add(B::status), self.bus) }
        }
        type status_array = status_array::GenericReal<B>;
        fn status_array(self) -> Self::status_array {
            unsafe {
                Self::status_array::with_bus(self.pointer.byte_add(B::status_array), self.bus)
            }
        }
        type control = real_control<B>;
        fn control(self) -> Self::control {
            unsafe { Self::control::with_bus(self.pointer.byte_add(B::control), self.bus) }
        }
    }

    impl<B: Bus> RealBlock for GenericReal<B> {
        type Bus = B;
        const ADDRESS_SIZE: usize = B::control + <B as registers::Bus<u8>>::ADDRESS_SIZE;
        unsafe fn with_bus(pointer: core::ptr::NonNull<()>, bus: B) -> Self {
            Self { bus, pointer }
        }
    }

    pub type Real = GenericReal<Mmio>;
}
```

## Inline nested array

Inline registers can have nested arrays:

```rust
tock_registers::registers! {
    pub nested_array {
        0x0 => array: [[u8; 2]; 3] { Read },
    }
}
```

expands to:

```rust
pub mod nested_array {
    use super::*;

    pub trait Interface:
        ArrayRegister<Element: ArrayRegister<Element: Register<DataType = u8> + Read>>
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
    pub struct Element<B: Bus> {
        bus: B,
        pointer: core::ptr::NonNull<()>,
    }

    impl<B: Bus> RealBlock for Element<B> {
        type Bus = B;
        const ADDRESS_SIZE: usize = <B as registers::Bus<u8>>::ADDRESS_SIZE;
        unsafe fn with_bus(pointer: core::ptr::NonNull<()>, bus: B) -> Self {
            Self { bus, pointer }
        }
    }

    impl<B: Bus> Register for Element<B> {
        type DataType = u8;
    }

    Read!(real_impl, Bus, Element, u8,);

    pub type GenericReal<B> = RealArrayRegister<RealArrayRegister<Element<B>, 2>, 3>;

    pub type Real = GenericReal<Mmio>;
}
```
