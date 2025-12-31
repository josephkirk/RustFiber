use crate::JobSystem;

pub trait ParallelSlice<T> {
    fn fiber_iter<'a>(&'a self, job_system: &'a JobSystem) -> ParallelIter<'a, T>;
}

pub trait ParallelSliceMut<T> {
    fn fiber_iter_mut<'a>(&'a mut self, job_system: &'a JobSystem) -> ParallelIterMut<'a, T>;
}

impl<T: Sync> ParallelSlice<T> for [T] {
    fn fiber_iter<'a>(&'a self, job_system: &'a JobSystem) -> ParallelIter<'a, T> {
        ParallelIter {
            slice: self,
            job_system,
        }
    }
}

impl<T: Send> ParallelSliceMut<T> for [T] {
    fn fiber_iter_mut<'a>(&'a mut self, job_system: &'a JobSystem) -> ParallelIterMut<'a, T> {
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

pub struct ParallelIterMut<'a, T> {
    slice: &'a mut [T],
    job_system: &'a JobSystem,
}

// Trampolines for type erasure
unsafe fn trampoline<T, F>(op_ptr: *const (), slice_ptr: *const (), range: std::ops::Range<usize>)
where
    F: Fn(&T) + Sync + Send + Clone,
{
    unsafe {
        let op = &*(op_ptr as *const F);
        let slice_base = slice_ptr as *const T;
        // Reconstruct slice for better vectorization
        let sub_slice = std::slice::from_raw_parts(slice_base.add(range.start), range.len());
        for item in sub_slice {
            op(item);
        }
    }
}

unsafe fn trampoline_mut<T, F>(
    op_ptr: *const (),
    slice_ptr: *const (),
    range: std::ops::Range<usize>,
) where
    F: Fn(&mut T) + Sync + Send + Clone,
{
    unsafe {
        let op = &*(op_ptr as *const F);
        let slice_base = slice_ptr as *mut T;
        // Reconstruct slice for better vectorization
        let sub_slice = std::slice::from_raw_parts_mut(slice_base.add(range.start), range.len());
        for item in sub_slice {
            op(item);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JobSystem;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicI32, Ordering};

    #[test]
    fn test_par_iter_mut_capture() {
        let job_system = JobSystem::default();
        let mut data = vec![1, 2, 3, 4, 5];
        let factor = 10;

        data.fiber_iter_mut(&job_system).for_each(|x| {
            *x *= factor;
        });

        assert_eq!(data, vec![10, 20, 30, 40, 50]);
    }

    #[test]
    fn test_par_iter_capture() {
        let job_system = JobSystem::default();
        let data = vec![1, 2, 3, 4, 5];
        let sum = Arc::new(AtomicI32::new(0));

        data.fiber_iter(&job_system).for_each(|&x| {
            sum.fetch_add(x, Ordering::Relaxed);
        });

        assert_eq!(sum.load(Ordering::Relaxed), 15);
    }
}

#[derive(Clone, Copy)]
struct CallContext {
    op_addr: usize,
    slice_addr: usize,
    trampoline: unsafe fn(*const (), *const (), std::ops::Range<usize>),
}
unsafe impl Send for CallContext {}
unsafe impl Sync for CallContext {}

impl<'a, T: Sync + 'static> ParallelIter<'a, T> {
    pub fn for_each<F>(self, op: F)
    where
        F: Fn(&T) + Sync + Send + Clone,
    {
        let len = self.slice.len();
        let slice_ptr = self.slice.as_ptr() as *const ();
        let op_ptr = &op as *const F as *const ();

        // Force monomorphization and get function pointer
        let trampoline_fn = trampoline::<T, F>;

        // Context contains only static data (pointers and fn pointer)
        let ctx = CallContext {
            op_addr: op_ptr as usize,
            slice_addr: slice_ptr as usize,
            trampoline: trampoline_fn,
        };

        // parallel_for_chunked_auto requires 'static closure.
        // ctx is Copy/Send/Sync and 'static.
        let counter = self
            .job_system
            .parallel_for_chunked_auto(0..len, move |range| unsafe {
                (ctx.trampoline)(ctx.op_addr as *const (), ctx.slice_addr as *const (), range);
            });
        self.job_system.wait_for_counter(&counter);
    }
}

impl<'a, T: Send + 'static> ParallelIterMut<'a, T> {
    pub fn for_each<F>(self, op: F)
    where
        F: Fn(&mut T) + Sync + Send + Clone,
    {
        let len = self.slice.len();
        let slice_ptr = self.slice.as_mut_ptr() as *const ();
        let op_ptr = &op as *const F as *const ();

        let trampoline_fn = trampoline_mut::<T, F>;

        let ctx = CallContext {
            op_addr: op_ptr as usize,
            slice_addr: slice_ptr as usize,
            trampoline: trampoline_fn,
        };

        let counter = self
            .job_system
            .parallel_for_chunked_auto(0..len, move |range| unsafe {
                (ctx.trampoline)(ctx.op_addr as *const (), ctx.slice_addr as *const (), range);
            });
        self.job_system.wait_for_counter(&counter);
    }
}
