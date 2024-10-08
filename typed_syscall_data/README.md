# Typed Syscall Data

To date, Tock's [syscall
ABI](https://github.com/tock/tock/blob/master/doc/reference/trd104-syscalls.md)
has only been defined for 32-bit non-CHERI platforms. This has allowed Tock to
use `u32`, `usize`, and pointers relatively interchangeably in its syscall ABI.
However, there is now interest in porting Tock to CHERI platforms, both 32-bit
and 64-bit. Suddenly, `u32`, `usize`, and pointers can be different sizes (in
64-bit CHERI, they are three distinct sizes!), so we can no longer treat them as
equivalent.

At a higher level, Tock frequently relies on syscall drivers and userspace
libraries to cast types for transfer across the syscall interface. The
temperature capsule [uses an unsigned upcall argument to send an `i32` to
userspace](https://github.com/tock/tock/blob/772ed33c594cb3fcd7590444a6b45aaca1172b68/capsules/extra/src/temperature.rs#L131),
the buttons capsule [uses SuccessWithU32 to return a boolean
value](https://github.com/tock/tock/blob/772ed33c594cb3fcd7590444a6b45aaca1172b68/doc/syscalls/00003_buttons.md#command-number-3),
the console driver [passes an error code in an integer argument of
upcalls](https://github.com/tock/tock/blob/772ed33c594cb3fcd7590444a6b45aaca1172b68/doc/syscalls/00001_console.md#subscribe-number-2),
and it is easy to find many other examples of manual casts in syscall drivers.

This document explores the possibility of providing a more type-safe syscall API
by proposing new type-safe syscall ABIs.

## What types do we care about?

To start, we can look at the [list of all Rust
types](https://doc.rust-lang.org/reference/types.html) and see several types
that probably make sense in a syscall interface:

* `bool`
* Signed integers: `i8`, `i16`, `i32`, `i64`, `i128`, `isize`
* Unsigned integers: `u8`, `u16`, `u32`, `u64`, `u128`, `usize`
* Floating point: `f32`, `f64`
* Codepoint: `char`
* Upcall function pointer
* Raw pointers: `*const T`, `*mut T`

There are a couple other types that we probably want to consider as well:

* Tock's `ErrorCode`, as it is extremely common.
* A CHERI capability (which is not necessarily a pointer and not necessarily
  tagged 1).
* A Register type that represents *any* possible register value. This is a
  future-compatibility safeguard: if we ever need a type that is not in our
  fixed list, we can call it an ArbitraryRegister and make things work (albeit
  with less type safety). It also allows us to embed ArbitraryData (which will
  be defined later) inside ArbitraryData, which is used by Yield.

### Reducing the number of types

Of course, there is a cost to complexity and in particular supporting a long
list of types. It probably doesn't make sense to provide special support for
*all* of the above types. In particular:

* `i8` and `i16` are not particularly common and can easily be passed as `i32`.
* `u8` and `u16` can similarly be passed as `u32`.
* `i128` and `u128` are extremely rare in Rust, completely absent in standard C,
  and probably exceptionally rare on the small systems Tock targets.
* `char` is rare in Tock, as Tock mostly treats text as byte buffers. When
  needed, it can be passed as `u32`. It's also not a standard part of C (where
  `char` has a different meaning).
* Wide pointers: these aren't a concept in other languages (e.g. C) and
  therefore don't belong in a syscall ABI.
* `*const T` and `*mut T` are redundant: they're distinct types, but only exist
  to communicate mutability information. We already handle this in Tock by using
  different system calls for buffers of different mutability. Therefore we only
  need one pointer type.

### The reduced list

This leaves us with:

* `bool`
* Numeric: `i32`, `i64`, `isize`, `u32`, `u64`, `usize`, `f32`, `f64`
* Upcall function pointer
* Narrow pointer (`*const T` or `*mut T` where `T: Sized`)
* `ErrorCode`
* CHERI capability

## Type descriptors

When a process wants to send data to the kernel, the process needs a way to tell
the kernel the sent data's type (and vice versa when the kernel sends data to
the process). To do this, we need a way to serialize information about a a list
of types. To start, lets assign numbers to each type (DNE means this type
doesn't exist yet). 

| ID       | Rust Type                 | C Type                    |
| -------- | ------------------------- | ------------------------- |
| `0b0001` | `ErrorCode`               | Error code (DNE)          |
| `0b0010` | `u32`                     | `uint32_t`                |
| `0b0011` | `i32`                     | `int32_t`                 |
| `0b0100` | `usize`                   | `size_t`                  |
| `0b0101` | `isize`                   | `ptrdiff_t`               |
| `0b0110` | `u64`                     | `uint64_t`                |
| `0b0111` | `i64`                     | `int64_t`                 |
| `0b1000` | `f32`                     | `float`                   |
| `0b1001` | `f64`                     | `double`                  |
| `0b1010` | `bool`                    | `bool`                    |
| `0b1011` | Upcall fn pointer         | Upcall fn pointer         |
| `0b1100` | `*mut T` where `T: Sized` | `T*`                      |
| `0b1101` | CHERI capability (DNE)    | CHERI capability (DNE)    |
| `0b1110` | *Reserved for future use* | *Reserved for future use* |
| `0b1111` | Register                  | Register (DNE)            |

We can describe a list of N types as a 4N bit integer by embedding the Nth type
ID in the Nth nibble of the integer. So:

* The empty list has ID `0`
* A list of one type has an ID equal to that type.
* A list of two types has the first type in bits 0-3 and the second in bits 4-7.
* A list of three types has the first type in bits 0-3, the second in bits 4-7,
  and the third in bits 8-11.
* and so on

For example, the type `(bool, u32, *mut T)` would be described by
`0b110000101010`, expanded here:

| Bits | Value  | Type           |
| ---- | ------ | -------------- |
| 0-3  | 0b1010 | `bool`         |
| 4-7  | 0b0010 | `u32`          |
| 8-11 | 0b1100 | Narrow pointer |

Note that if this type descriptor were stored in a larger type (such as a
`u32`), you can determine that it is a list of three types because `0b0000` is
not a valid type ID.

## Putting multiple differently-typed values into registers

This table indicates how many registers are needed for each type:

| Type              | 32 bit non-CHERI | 64 bit non-CHERI | 32 bit CHERI | 64 bit CHERI |
| ----------------- | ---------------- | ---------------- | ------------ | ------------ |
| `u64`             | 2                | 1                | 1? 2?        | 1            |
| `i64`             | 2                | 1                | 1? 2?        | 1            |
| `f64`             | 2                | 1                | 1? 2?        | 1            |
| CHERI capability  | N/A              | N/A              | 1            | 1            |
| *Everything else* | 1                | 1                | 1            | 1            |

Shorthand: we'll use `regcount<T>` to denote the number of registers needed for
type `T` on a particular platform.

If we have a list of typed values `(v1: T1, v2: T2, v3: T3, ...)` and an ordered
list of registers, we can store `v1` in the first `regcount<T1>` registers, `v2`
in the next `regcount<T2>` registers, `v3` in the next `regcount<T3>` registers,
and so on. Values that span multiple registers store their least significant
bits in the first register and their most significant bits in the second
register. To make this concrete, if we have values `(v1: bool, v2: u64, v3: *mut
())` we would pack them as follows on a 32 bit non-CHERI system:

1. `v1`
2. Least-significant 32 bits of `v2`
3. Most-significant 32 bits of `v2`
4. `v3`

## ArbitraryData

`ArbitraryData` is a data structure designed to fit in registers and carry data
of a variety of types. The value of the first register is the type descriptor
for the list of data types. The remaining pieces of data are stored in the rest
of the registers in order (as described in the previous heading).

For example, on a non-CHERI 32-bit system, `ArbitraryData` would store the value
`(true, 0x0123456789ABCDEFu64, 3i32)` as:

| Register | Value            | Meaning                                |
| -------- | ---------------- | -------------------------------------- |
| 0        | `0b001101101010` | Type descriptor for `(bool, u64, i32)` |
| 1        | `1`              | `true`                                 |
| 2        | `0x89ABCDEF`     | Lower 32 bits of `0x0123456789ABCDEF`  |
| 3        | `0x01234567`     | Upper 32 bits of `0x0123456789ABCDEF`  |
| 4        | `3`              | `3`                                    |

We also use `ArbitraryData(N)` (where N >= 1) to denote an `ArbitraryData` that
must fit into N registers. If the `ArbitraryData` only uses the first K
registers to store data, then the last N-K registers are left unspecified and
should not be read.

## Register names

We'll use the names `a1`, `a2`, `a3`, etc. to refer to registers used to pass
syscall arguments to the kernel, and the names `r1`, `r2`, `r3`, etc. to refer
to registers used to return syscall results to userspace. These names map to
ARM and RISC-V registers via the following tables:

|        |  a1 |  a2 |  a3 |  a4 |  a5 |  a6 |  a7 |  a8 |  a9 | a10 | a11 | a12 | a13 | a14 | a15 | a16 | a17 | a18 | a19 | a20 | a21 | a22 | a23 | a24 | a25 |
| ------ | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ARM    |  a1 |  a2 |  a3 |  a4 |  v1 |  v2 |  v3 |  v4 |  v5 |  v7 |  v8 |     |     |     |     |     |     |     |     |     |     |     |     |     |     |
| RISC-V | x11 | x12 | x13 | x14 | x15 | x16 | x17 |  x5 |  x6 |  x7 | x28 | x29 | x30 | x31 |  x9 | x18 | x19 | x20 | x21 | x22 | x23 | x24 | x25 | x26 | x27 |

|        |  r1 |  r2 |  r3 |  r4 |  r5 |  r6 |  r7 |  r8 |  r9 | r10 | r11 | r12 | r13 | r14 | r15 | r16 | r17 | r18 | r19 | r20 | r21 | r22 | r23 | r24 | r25 | r26 |
| ------ | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ARM    |  a1 |  a2 |  a3 |  a4 |  v1 |  v2 |  v3 |  v4 |  v5 |  v7 |  v8 |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |
| RISC-V | x10 | x11 | x12 | x13 | x14 | x15 | x16 | x17 |  x5 |  x6 |  x7 | x28 | x29 | x30 | x31 |  x9 | x18 | x19 | x20 | x21 | x22 | x23 | x24 | x25 | x26 | x27 |

An ARM, the system call being invoked is passed via the `svc` instruction; on
RISC-V it is passed via `x10` (this is why `x10` is a return register but not
an argument register).

## Syscall ABI A

This section proposes a system call ABI for Tock. This system call ABI is based
on [TRD
104](https://github.com/tock/tock/blob/master/doc/reference/trd104-syscalls.md)
and omits many details that are either the same as TRD 104 or which are
irrelevant to exploring the concept of typed system calls.

### Return Values

All system calls return an `ArbitraryData` that starts at `r1` and spans across
all the return registers.

If userspace tries to invoke a system call that the kernel does not recognize or
does not support, or a system call on a system call driver that does not exist,
the kernel will return type `(ErrorCode)`.

### Upcall arguments

Upcall arguments are `ArbitraryData(4)`. This allows syscall drivers to pass up
to three values to each invoked upcall (depending on types and the platform).
Upcall functions must have the following signature (specified in both Rust and
C):

```C
void(void*, Register, Register, Register, Register)
```

```Rust
unsafe extern "C" fn(*mut (), Register, Register, Register, Register)
```

The first argument is the userdata pointer, the rest are the `ArbitraryData(4)`.

### Yield

Arguments:

| Yield type | a1  | a2            | a3               |
| ---------- | --- | ------------- | ---------------- |
| no-wait    | `0` | Unused        | Unused           |
| wait       | `1` | Unused        | Unused           |
| wait-for   | `2` | Driver number | Subscribe number |

If there is no pending upcall, yield-no-wait will return `()`.

If there is an upcall to invoke, yield-wait and yield-no-wait return `(upcall fn
pointer, userdata: *mut (), Register, Register, Register, Register)`. The
userspace Yield function should then invoke the provided function pointer and
pass it the userdata and register values.

If there is an upcall, yield-wait-for returns `(Register, Register, Register,
Register)`. The four `Register` values are the upcall's `ArbitraryData(4)`
parameters.
