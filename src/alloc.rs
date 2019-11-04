// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(unstable_name_collisions)]
#![allow(dead_code)]

//! Memory allocation APIs

use core::fmt;
use core::mem;
use core::ptr::NonNull;
use core::usize;

pub use core::alloc::{Layout, LayoutErr};

fn new_layout_err() -> LayoutErr {
    Layout::from_size_align(1, 3).unwrap_err()
}

pub fn handle_alloc_error(layout: Layout) -> ! {
    panic!("encountered allocation error: {:?}", layout)
}

pub trait UnstableLayoutMethods {
    fn padding_needed_for(&self, align: usize) -> usize;
    fn repeat(&self, n: usize) -> Result<(Layout, usize), LayoutErr>;
    fn array<T>(n: usize) -> Result<Layout, LayoutErr>;
}

impl UnstableLayoutMethods for Layout {
    fn padding_needed_for(&self, align: usize) -> usize {
        let len = self.size();

        // Rounded up value is:
        //   len_rounded_up = (len + align - 1) & !(align - 1);
        // and then we return the padding difference: `len_rounded_up - len`.
        //
        // We use modular arithmetic throughout:
        //
        // 1. align is guaranteed to be > 0, so align - 1 is always
        //    valid.
        //
        // 2. `len + align - 1` can overflow by at most `align - 1`,
        //    so the &-mask wth `!(align - 1)` will ensure that in the
        //    case of overflow, `len_rounded_up` will itself be 0.
        //    Thus the returned padding, when added to `len`, yields 0,
        //    which trivially satisfies the alignment `align`.
        //
        // (Of course, attempts to allocate blocks of memory whose
        // size and padding overflow in the above manner should cause
        // the allocator to yield an error anyway.)

        let len_rounded_up = len.wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
        len_rounded_up.wrapping_sub(len)
    }

    fn repeat(&self, n: usize) -> Result<(Layout, usize), LayoutErr> {
        let padded_size = self
            .size()
            .checked_add(self.padding_needed_for(self.align()))
            .ok_or_else(new_layout_err)?;
        let alloc_size = padded_size.checked_mul(n).ok_or_else(new_layout_err)?;

        unsafe {
            // self.align is already known to be valid and alloc_size has been
            // padded already.
            Ok((
                Layout::from_size_align_unchecked(alloc_size, self.align()),
                padded_size,
            ))
        }
    }

    fn array<T>(n: usize) -> Result<Layout, LayoutErr> {
        Layout::new::<T>().repeat(n).map(|(k, offs)| {
            debug_assert!(offs == mem::size_of::<T>());
            k
        })
    }
}

/// Represents the combination of a starting address and
/// a total capacity of the returned block.
// #[unstable(feature = "allocator_api", issue = "32838")]
#[derive(Debug)]
pub struct Excess(pub NonNull<u8>, pub usize);

fn size_align<T>() -> (usize, usize) {
    (mem::size_of::<T>(), mem::align_of::<T>())
}

/// The `AllocErr` error indicates an allocation failure
/// that may be due to resource exhaustion or to
/// something wrong when combining the given input arguments with this
/// allocator.
// #[unstable(feature = "allocator_api", issue = "32838")]
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AllocErr;

// (we need this for downstream impl of trait Error)
// #[unstable(feature = "allocator_api", issue = "32838")]
impl fmt::Display for AllocErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("memory allocation failed")
    }
}

/// The `CannotReallocInPlace` error is used when `grow_in_place` or
/// `shrink_in_place` were unable to reuse the given memory block for
/// a requested layout.
// #[unstable(feature = "allocator_api", issue = "32838")]
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CannotReallocInPlace;

// #[unstable(feature = "allocator_api", issue = "32838")]
impl CannotReallocInPlace {
    pub fn description(&self) -> &str {
        "cannot reallocate allocator's memory in place"
    }
}

// (we need this for downstream impl of trait Error)
// #[unstable(feature = "allocator_api", issue = "32838")]
impl fmt::Display for CannotReallocInPlace {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}
