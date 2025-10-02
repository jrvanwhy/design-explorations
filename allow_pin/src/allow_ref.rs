//! A design based around the `allow::Ref` type, which is a type that can point
//! to both 'static and stack-based buffers.

pub type ErrorCode = u32;

pub mod allow {
    use crate::*;
    use core::cell::Cell;
    use core::ptr::null_mut;

    pub struct Buffer<T: Allowable + ?Sized> {
        share_info: Cell<Option<ShareInfo>>,
        buffer: T,
    }

    impl<T: Allowable> From<T> for Buffer<T> {
        fn from(buffer: T) -> Buffer<T> {
            Buffer {
                share_info: Cell::new(None),
                buffer,
            }
        }
    }

    impl<T: Allowable + ?Sized> Drop for Buffer<T> {
        fn drop(&mut self) {
            if let Some(info) = self.share_info.take() {
                unsafe {
                    dynamic_allow(
                        info.driver_num,
                        info.buffer_num,
                        null_mut(),
                        0,
                        info.allow_type,
                    );
                }
            }
        }
    }

    impl<T: Allowable + ?Sized> Buffer<T> {
        pub fn allow_ref<'s>(self: Pin<&'s Buffer<T>>) -> AllowRef<&'s T> {
            let this = unsafe { Pin::into_inner_unchecked(self) };
            AllowRef {
                buffer: &this.buffer,
                share_info: Some(&this.share_info),
            }
        }
    }

    /// A reference to a buffer that may be shared with the kernel. P should be either a `&T` or a
    /// `&mut T`.
    // TODO: Is there a way to remove the explicit 'b from here and instead get it through the Ptr
    // trait? I don't want AllowRef<'b, &'b u8> showing up everywhere.
    pub struct AllowRef<'b, P: Ptr<'b>> {
        // Safety invariant: if share_info is Some(...), then the buffer referred to by share_info
        // will be unallowed before `buffer`'s pointee is dropped. If `share_info` is `None`, then
        // `buffer` is 'static and does not need to be unshared.
        buffer: P,
        share_info: Option<P::LifetimeRef<Cell<Option<ShareInfo>>>>,
    }

    impl<T: Allowable + ?Sized> AllowRef<&'static T> {
        pub fn new_static(buffer: &'static T) -> AllowRef<&'static T> {
            todo!()
        }
    }

    impl<T: Allowable + ?Sized> AllowRef<&'static mut T> {
        pub fn new_static(buffer: &'static mut T) -> AllowRef<&'static mut T> {
            todo!()
        }
    }

    impl<'b, T: Allowable + ?Sized> AllowRef<&'b T> {}

    #[derive(Clone, Copy)]
    pub struct ShareInfo {
        allow_type: DynamicType,
        driver_num: u32,
        buffer_num: u32,
    }

    /// Trait representing an allowable type.
    pub trait Allowable: FromBytes + IntoBytes + 'static {}
    impl<T: FromBytes + IntoBytes + ?Sized + 'static> Allowable for T {}

    /// Reference type that an AllowRef can be.
    pub trait Ptr<'b>: sealed::Sealed {
        // For internal use
        type LifetimeRef<T: 'b>;
    }

    impl<'a, B: Allowable + ?Sized> Ptr for &'a B {
        type LifetimeRef<T: 'a> = &'a T;
    }
    impl<'a, B: Allowable + ?Sized> Ptr for &'a mut B {
        type LifetimeRef<T: 'a> = &'a T;
    }

    mod sealed {
        pub trait Sealed {}

        impl<'a, T: super::Allowable + ?Sized> Sealed for &'a T {}
        impl<'a, T: super::Allowable + ?Sized> Sealed for &'a mut T {}
    }
}
