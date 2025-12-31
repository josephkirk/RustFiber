use crate::JobSystem;


// #[derive(Clone, Copy)]
struct UnsafeSlice<T> {
    slice: *mut [T],
}

unsafe impl<T> Send for UnsafeSlice<T> {}
unsafe impl<T> Sync for UnsafeSlice<T> {}

impl<T> UnsafeSlice<T> {
    fn new(slice: &mut [T]) -> Self {
        Self {
            slice: slice as *mut [T],
        }
    }

    /// Safety: Caller must ensure disjoint access from other threads.
    unsafe fn get_mut<'a>(&self, index: usize) -> &'a mut T {
        // Safety requirement implicitly met by unsafe fn
        unsafe { &mut (*self.slice)[index] }
    }
}

// Manual impl to avoid T: Clone/Copy bound
impl<T> Copy for UnsafeSlice<T> {}
impl<T> Clone for UnsafeSlice<T> {
    fn clone(&self) -> Self {
        *self
    }
}

pub trait ParallelSlice<T> {
    fn par_iter<'a>(&'a self, job_system: &'a JobSystem) -> ParallelIter<'a, T>;
}

pub trait ParallelSliceMut<T> {
    fn par_iter_mut<'a>(&'a mut self, job_system: &'a JobSystem) -> ParallelIterMut<'a, T>;
}

impl<T: Sync> ParallelSlice<T> for [T] {
    fn par_iter<'a>(&'a self, job_system: &'a JobSystem) -> ParallelIter<'a, T> {
        ParallelIter {
            slice: self,
            job_system,
        }
    }
}

impl<T: Send> ParallelSliceMut<T> for [T] {
    fn par_iter_mut<'a>(&'a mut self, job_system: &'a JobSystem) -> ParallelIterMut<'a, T> {
        ParallelIterMut {
            slice: self,
            job_system,
        }
    }
}

pub struct ParallelIter<'a, T> {
    slice: &'a [T],
    job_system: &'a JobSystem,
}

impl<'a, T: Sync + 'static> ParallelIter<'a, T> {
    pub fn for_each<F>(self, op: F)
    where
        F: Fn(&T) + Sync + Send + Clone + 'static,
    {
        let len = self.slice.len();
        // Avoid creating a mutable reference from a shared one directly as it is UB.
        // We go via raw pointers.
        let slice_ptr = self.slice.as_ptr() as *mut T;
        let slice_len = self.slice.len();
        // Reconstruct unsafe slice manually from raw parts
        let unsafe_slice = UnsafeSlice {
            slice: std::ptr::slice_from_raw_parts_mut(slice_ptr, slice_len),
        };
        
        let counter = self.job_system.parallel_for_auto(0..len, move |i| {
            // Safety: Disjoint access guaranteed by index i
            let item = unsafe { &*(unsafe_slice.get_mut(i) as *const T) };
            op(item);
        });
        self.job_system.wait_for_counter(&counter);
    }
}

pub struct ParallelIterMut<'a, T> {
    slice: &'a mut [T],
    job_system: &'a JobSystem,
}

impl<'a, T: Send + 'static> ParallelIterMut<'a, T> {
    pub fn for_each<F>(self, op: F)
    where
        F: Fn(&mut T) + Sync + Send + Clone + 'static,
    {
        let len = self.slice.len();
        let unsafe_slice = UnsafeSlice::new(self.slice);

        let counter = self.job_system.parallel_for_auto(0..len, move |i| {
            // Safety: parallel_for_auto guarantees distinct indices i
            let item = unsafe { unsafe_slice.get_mut(i) };
            op(item);
        });
        self.job_system.wait_for_counter(&counter);
    }
}
