//! A design based around the `allow::Ref` type, which is a type that can point
//! to both 'static and stack-based buffers. `allow::Ref` itself is type (and
//! lifetime)-erased to avoid monomorphization bloat in APIs. However, it is
//! limited to only RO Allows or only RW Allows (because only allow::Ref<Ro> can
//! be created from a `&'static [u8]`).

pub type ErrorCode = u32;

pub mod allow {
    use crate::*;
    use core::ptr::null_mut;

    /// A reference to something that can be shared with the kernel via the
    /// Read-Only Allow system call (and if P is Rw, read-write Allow).
    pub struct Ref<P: Permissions, T>
    where
        [T]: Allowable,
    {
        // The lifetime is erased to 'static here. There is no correct lifetime to
        // write when an AllowRef is embedded inside a Buffer.
        buffer: P::BufRef<'static, [T]>,
        // APIs need to be able to retrieve mutable references to the buffer
        // (for RW Allow), which requires that they have an exclusive reference
        // to the Ref. However, giving an &mut reference to the Ref would allow
        // the buffer to replace the Ref and forget it, which would prevent the
        // unallow from occurring before the buffer is dropped. To prevent that,
        // we pin the Ref as well.
        _pinned: PhantomPinned,
        share_info: Option<ShareInfo>,
    }

    impl<P: Permissions, T> Ref<P, T>
    where
        [T]: Allowable,
    {
        fn unshare_if_shared(&mut self) {
            let Some(ref info) = self.share_info else {
                return;
            };
            unsafe {
                static_allow::<P::Static>(info.driver_num, info.buffer_num, null_mut(), 0);
            }
            self.share_info = None;
        }
    }

    impl<P: Permissions, T> Ref<P, T>
    where
        [T]: Allowable,
    {
        pub fn allow(
            self: Pin<&mut Ref<P, T>>,
            driver_num: u32,
            buffer_num: u32,
        ) -> Result<(), ErrorCode> {
            if self.share_info.is_some() {
                return Err(3);
            }
            let this = unsafe { Pin::into_inner_unchecked(self) };
            let (variant, r1, _, _) = unsafe {
                static_allow::<P::Static>(
                    driver_num,
                    buffer_num,
                    &mut this.buffer as *mut _ as *mut u8,
                    size_of_val(&this.buffer),
                )
            };
            if variant == 2 {
                return Err(r1.addr() as u32);
            }
            this.share_info = Some(ShareInfo {
                driver_num,
                buffer_num,
            });
            Ok(())
        }

        pub fn unallow(self: Pin<&mut Ref<P, T>>) {
            let this = unsafe { Pin::into_inner_unchecked(self) };
            this.unshare_if_shared();
        }
    }

    impl<T> From<&'static [T]> for Ref<Ro, T>
    where
        [T]: Allowable,
    {
        fn from(value: &'static [T]) -> Self {
            Self {
                buffer: value,
                _pinned: Default::default(),
                share_info: None,
            }
        }
    }

    impl<T> From<&'static mut [T]> for Ref<Rw, T>
    where
        [T]: Allowable,
    {
        fn from(value: &'static mut [T]) -> Self {
            Self {
                buffer: value,
                _pinned: Default::default(),
                share_info: None,
            }
        }
    }

    impl<T, P: Permissions> Drop for Ref<P, T>
    where
        [T]: Allowable,
    {
        fn drop(&mut self) {
            self.unshare_if_shared();
        }
    }

    struct ShareInfo {
        driver_num: u32,
        buffer_num: u32,
    }

    /// Trait representing an allowable type.
    pub trait Allowable: FromBytes + IntoBytes + 'static {}
    impl<T: FromBytes + IntoBytes + ?Sized + 'static> Allowable for T {}

    /// A type that represents whether a `Ref` has write access to a buffer.
    pub trait Permissions: sealed::Sealed {
        type BufRef<'b, T: Allowable + ?Sized>;
        // Only exists within the context of this design exploration; would not
        // exist in libtock-rs (because Permissions replaces StaticType
        // entirely).
        type Static: StaticType;
    }

    // Permissions implementations.
    pub enum Ro {}
    impl sealed::Sealed for Ro {}
    impl Permissions for Ro {
        type BufRef<'b, B: Allowable + ?Sized> = &'b B;
        type Static = StaticRo;
    }
    pub enum Rw {}
    impl sealed::Sealed for Rw {}
    impl Permissions for Rw {
        type BufRef<'b, B: Allowable + ?Sized> = &'b mut B;
        type Static = StaticRw;
    }

    mod sealed {
        pub trait Sealed {}
    }

    pub struct Buffer<P: Permissions, T, const N: usize>
    where
        [T]: Allowable,
    {
        allow_ref: Ref<P, T>,
        buffer: [T; N],
    }

    impl<T, const N: usize> From<[T; N]> for Buffer<Ro, T, N>
    where
        [T]: Allowable,
    {
        fn from(value: [T; N]) -> Self {
            let reference = unsafe { core::mem::transmute(&value as &[T]) };

            Self {
                buffer: value,
                allow_ref: Ref {
                    buffer: reference,
                    _pinned: Default::default(),
                    share_info: None,
                },
            }
        }
    }

    impl<T, const N: usize> From<[T; N]> for Buffer<Rw, T, N>
    where
        [T]: Allowable,
    {
        fn from(mut value: [T; N]) -> Self {
            let reference = unsafe { core::mem::transmute(&mut value as &mut [T]) };

            Self {
                buffer: value,
                allow_ref: Ref {
                    buffer: reference,
                    _pinned: Default::default(),
                    share_info: None,
                },
            }
        }
    }

    impl<T, P: Permissions, const N: usize> Buffer<P, T, N>
    where
        [T]: Allowable,
    {
        pub fn buffer(self: Pin<&Buffer<P, T, N>>) -> Option<&[T; N]> {
            if self.allow_ref.share_info.is_some() {
                return None;
            }

            Some(&self.get_ref().buffer)
        }

        pub fn buffer_mut(self: Pin<&mut Buffer<P, T, N>>) -> Option<&mut [T; N]> {
            if self.allow_ref.share_info.is_some() {
                return None;
            }

            Some(&mut unsafe { Pin::into_inner_unchecked(self) }.buffer)
        }

        pub fn unallow(self: Pin<&mut Buffer<P, T, N>>) -> &mut [T; N] {
            let this = unsafe { Pin::into_inner_unchecked(self) };
            this.allow_ref.unshare_if_shared();
            &mut this.buffer
        }

        pub fn allow_ref(self: Pin<&mut Buffer<P, T, N>>) -> Pin<&mut Ref<P, T>> {
            unsafe { self.map_unchecked_mut(|buffer| &mut buffer.allow_ref) }
        }

        pub fn allow(
            self: Pin<&mut Buffer<P, T, N>>,
            driver_num: u32,
            buffer_num: u32,
        ) -> Result<(), ErrorCode> {
            self.allow_ref().allow(driver_num, buffer_num)
        }
    }
}
