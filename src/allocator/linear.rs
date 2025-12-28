use std::alloc::{Layout, alloc, dealloc};
use std::ptr::NonNull;

/// A simple linear (bump) allocator for per-frame allocations.
///
/// This allocator manages a fixed-size memory block and allocates linearly
/// by advancing a cursor. It is designed to be extremely fast and lock-free
/// for single-threaded usage (e.g., owned by a Worker).
pub struct FrameAllocator {
    base_ptr: NonNull<u8>,
    capacity: usize,
    cursor: usize,
}

impl FrameAllocator {
    /// Creates a new FrameAllocator with the specified capacity in bytes.
    ///
    /// # Panics
    ///
    /// Panics if memory allocation fails.
    pub fn new(capacity: usize) -> Self {
        // Use a default alignment of 16 bytes for the base block to cover most SIMD needs
        let layout =
            Layout::from_size_align(capacity, 16).expect("Invalid layout for FrameAllocator");

        let ptr = unsafe { alloc(layout) };

        if ptr.is_null() {
            panic!("Failed to allocate backing memory for FrameAllocator");
        }

        Self {
            base_ptr: NonNull::new(ptr).expect("Allocated pointer was null"),
            capacity,
            cursor: 0,
        }
    }

    /// Allocates memory with the given layout.
    ///
    /// Returns a pointer to the allocated memory, or None if there is insufficient space.
    pub fn alloc(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        let current_base = unsafe { self.base_ptr.as_ptr().add(self.cursor) };
        let align_offset = current_base.align_offset(layout.align());

        let new_cursor = self
            .cursor
            .checked_add(align_offset)?
            .checked_add(layout.size())?;

        if new_cursor <= self.capacity {
            let res_ptr = unsafe { self.base_ptr.as_ptr().add(self.cursor + align_offset) };
            self.cursor = new_cursor;
            NonNull::new(res_ptr)
        } else {
            None
        }
    }

    /// Allocates memory for a value of type T and writes the value into it.
    ///
    /// Returns a pointer to the value, or None if there is insufficient space.
    pub fn alloc_val<T>(&mut self, value: T) -> Option<NonNull<T>> {
        let layout = Layout::new::<T>();
        let ptr = self.alloc(layout)?.cast::<T>();
        unsafe {
            ptr.as_ptr().write(value);
        }
        Some(ptr)
    }

    /// Resets the allocator, reclaiming all memory.
    ///
    /// # Safety
    ///
    /// This invalidates all pointers previously allocated from this allocator.
    /// The caller must ensure that no references to allocated data exist.
    pub unsafe fn reset(&mut self) {
        self.cursor = 0;
    }

    /// Returns the number of bytes currently allocated.
    pub fn used_bytes(&self) -> usize {
        self.cursor
    }

    /// Returns the total capacity in bytes.
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl Drop for FrameAllocator {
    fn drop(&mut self) {
        let layout = Layout::from_size_align(self.capacity, 16).unwrap();
        unsafe {
            dealloc(self.base_ptr.as_ptr(), layout);
        }
    }
}

// FrameAllocator owns its memory and can be moved between threads (e.g. at creation)
unsafe impl Send for FrameAllocator {}
// It is NOT Sync. Only the owning thread can allocate.
