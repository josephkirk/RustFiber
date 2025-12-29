//! Fiber management and execution context.
//!
//! This module provides lightweight execution contexts (coroutines) for jobs,
//! allowing them to yield execution without blocking the OS thread.

use crate::job::Job;
use corosensei::{Coroutine, CoroutineResult, Yielder};

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicPtr, AtomicU32};

// Re-export FiberHandle as a wrapper for raw pointer to allow Send/Sync
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FiberHandle(pub *mut Fiber);

unsafe impl Send for FiberHandle {}
unsafe impl Sync for FiberHandle {}

impl FiberHandle {
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }

    pub fn null() -> Self {
        FiberHandle(std::ptr::null_mut())
    }
}

pub const NODE_STATE_RUNNING: u32 = 0;
pub const NODE_STATE_WAITING: u32 = 1;
pub const NODE_STATE_SIGNALED: u32 = 2;
pub const NODE_STATE_SPINNING: u32 = 3;

#[repr(transparent)]
pub struct AllocatorPtr(pub *mut crate::allocator::paged::PagedFrameAllocator);
unsafe impl Send for AllocatorPtr {}
unsafe impl Sync for AllocatorPtr {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueuePtr(pub *const crossbeam::deque::Worker<Job>);
unsafe impl Send for QueuePtr {}

pub enum FiberInput {
    Start(
        Job,
        *const dyn crate::counter::JobScheduler,
        *mut Fiber,
        Option<AllocatorPtr>,
        Option<QueuePtr>,
    ),
    Resume,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YieldType {
    /// Reschedule the fiber immediately (cooperative yield).
    Normal,
    /// Suspend the fiber and wait for an external signal (intrusive wait).
    Wait,
    /// Job completed, fiber ready for reuse (Trampoline Pattern).
    Complete,
}

/// The node embedded in every Fiber for intrusive lists.
/// Must be 'repr(C)' for stable pointer offsets.
#[repr(C, align(64))]
pub struct WaitNode {
    /// Atomic pointer to the next node in the list.
    pub next: AtomicPtr<WaitNode>,
    /// The handle to the fiber to be resumed.
    pub fiber_handle: AtomicPtr<Fiber>,
    /// State of the node for race-free signaling.
    pub state: AtomicU32,
}

impl Default for WaitNode {
    fn default() -> Self {
        Self {
            next: AtomicPtr::new(std::ptr::null_mut()),
            fiber_handle: AtomicPtr::new(std::ptr::null_mut()),
            state: AtomicU32::new(NODE_STATE_RUNNING),
        }
    }
}

use corosensei::stack::DefaultStack;

/// Represents a fiber - a lightweight stackful execution context.
///
/// Uses `corosensei` for context switching.
pub struct Fiber {
    /// The underlying coroutine.
    /// 'static lifetime is a lie to make it self-referential safe in this context,
    /// as we promise not to drop stack while coroutine exists.
    /// Drop order: coroutine drops first, then stack.
    coroutine: Option<Coroutine<FiberInput, YieldType, (), &'static mut DefaultStack>>,

    /// The stack backing the coroutine.
    /// Owned here to ensure proper drop order: coroutine first, then stack.
    #[allow(dead_code)]
    stack: Box<DefaultStack>,

    /// The intrusive node used when the fiber is waiting.
    /// Wrapped in UnsafeCell because it's modified by other threads while suspended.
    pub wait_node: UnsafeCell<WaitNode>,

    /// The yielder for this fiber, set when the fiber starts.
    /// Valid only when the fiber is running.
    yielder: *const Yielder<FiberInput, YieldType>,

    /// Flag to prevent double-resume hazards.
    /// Set to true when suspended, false when running.
    pub is_suspended: std::sync::atomic::AtomicBool,

    /// Whether the stack has been prefaulted for NUMA locality.
    /// Prefaulting is done lazily on first resume to avoid Windows stack probing issues.
    stack_prefaulted: bool,

    /// Whether to prefetch stack pages for NUMA locality.
    prefetch_pages: bool,
}

pub enum FiberState {
    Yielded(YieldType),
    Complete,
    Panic(Box<dyn std::any::Any + Send>),
}

// Thread-local yielder accessible to Context
thread_local! {
    static CURRENT_FIBER: std::cell::Cell<Option<FiberHandle>> = const { std::cell::Cell::new(None) };
}

impl Fiber {
    /// Creates a new fiber with the given stack size.
    /// Uses a Trampoline Pattern: the coroutine loops forever, yielding Complete after each job.
    pub fn new(stack_size: usize, prefetch_pages: bool) -> Self {
        let mut stack = Box::new(
            DefaultStack::new(stack_size)
                .unwrap_or_else(|_| DefaultStack::new(1024 * 1024).unwrap()),
        );

        // Extend lifetime of stack to 'static to satisfy Coroutine type.
        // SAFETY: We ensure `coroutine` is dropped before `stack`.
        let stack_ref = unsafe {
            std::mem::transmute::<&mut DefaultStack, &'static mut DefaultStack>(stack.as_mut())
        };

        // Trampoline Pattern: Three-phase loop per the ECS fiber guide
        // Phase A: Execution (with panic handling)
        // Phase B: Synchronization (counter decrement via RAII guard)
        // Phase C: Recycling (yield Complete, loop for next job)
        let coroutine = Coroutine::with_stack(stack_ref, move |yielder, mut input: FiberInput| {
            use std::panic::{AssertUnwindSafe, catch_unwind};

            loop {
                if let FiberInput::Start(
                    job,
                    scheduler_ptr,
                    fiber_ptr,
                    allocator_wrapper,
                    queue_wrapper,
                ) = input
                {
                    // === PHASE A: Execution with Panic Handling ===
                    // Panics must not escape the fiber (UB across FFI/asm boundary)
                    unsafe {
                        (*fiber_ptr).yielder = yielder as *const _;
                    }

                    let scheduler = unsafe { &*scheduler_ptr };
                    let allocator = allocator_wrapper.map(|w| w.0);
                    let queue = queue_wrapper.map(|w| w.0);

                    let result = catch_unwind(AssertUnwindSafe(|| {
                        // Job::execute handles counter decrement via CounterGuard (Phase B)
                        job.execute(scheduler, allocator, queue);
                    }));

                    if let Err(panic_payload) = result {
                        // Log panic but do not propagate - fiber continues to recycle
                        let msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                            *s
                        } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                            s.as_str()
                        } else {
                            "Unknown panic"
                        };
                        eprintln!("[Fiber] Job panicked: {}", msg);
                        // Note: CounterGuard already decremented in Job::execute
                    }
                }

                // === PHASE C: Recycling ===
                // Yield Complete to return fiber to pool, await next job
                input = yielder.suspend(YieldType::Complete);
            }
        });

        Fiber {
            coroutine: Some(coroutine),
            stack,
            wait_node: UnsafeCell::new(WaitNode::default()),
            yielder: std::ptr::null(),
            is_suspended: std::sync::atomic::AtomicBool::new(false),
            stack_prefaulted: false,
            prefetch_pages,
        }
    }

    /// Pre-faults the stack memory to ensure NUMA locality.
    /// This is a placeholder for future implementation.
    /// Currently disabled due to Windows stack probing compatibility issues.
    #[inline(never)]
    fn prefault_stack_memory(&mut self) {
        if self.stack_prefaulted || !self.prefetch_pages {
            return; // Already prefaulted or disabled
        }

        // TODO: Implement platform-specific prefaulting that respects Windows stack semantics
        // For now, we rely on OS page faults during first use
        self.stack_prefaulted = true; // Mark as done to avoid repeated attempts
    }

    /// Resumes the fiber with a new job or continues execution.
    pub fn resume(&mut self, input: FiberInput) -> FiberState {
        // Lazy prefaulting: ensure stack pages are allocated on current NUMA node
        // This is done here (after coroutine creation) to avoid Windows stack probing issues
        self.prefault_stack_memory();

        let self_ptr = self as *mut _;
        if let Some(coroutine) = self.coroutine.as_mut() {
            // Set current fiber for introspection/scheduling logic
            CURRENT_FIBER.set(Some(FiberHandle(self_ptr)));

            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| coroutine.resume(input)));

            CURRENT_FIBER.set(None);

            // Trampoline Pattern yield semantics:
            // - Yield(Complete) = job completed, fiber ready for reuse → FiberState::Complete
            // - Yield(Normal/Wait) = mid-job suspend → Yielded(...)
            // - Return = should never happen (infinite loop) → Complete
            match result {
                Ok(CoroutineResult::Yield(YieldType::Complete)) => FiberState::Complete,
                Ok(CoroutineResult::Yield(y_type)) => FiberState::Yielded(y_type),
                Ok(CoroutineResult::Return(_)) => FiberState::Complete,
                Err(payload) => FiberState::Panic(payload),
            }
        } else {
            FiberState::Complete
        }
    }

    /// Resets the fiber state for reuse.
    /// With Trampoline Pattern, the coroutine is reused - no recreation needed.
    pub fn reset(&mut self, _stack_size: usize, prefetch_pages: bool) {
        // Update prefetch_pages setting
        self.prefetch_pages = prefetch_pages;

        // Reset prefaulting flag for lazy prefaulting on next use
        self.stack_prefaulted = false;

        // Reset suspension state
        self.is_suspended
            .store(false, std::sync::atomic::Ordering::Relaxed);

        // NOTE: With Trampoline Pattern, coroutine stays alive and loops.
        // No recreation needed - this is the key optimization!
    }

    /// Yields execution of the current fiber.
    pub fn yield_now(reason: YieldType) {
        if let Some(handle) = CURRENT_FIBER.get() {
            // We are running inside a fiber.
            // Access the yielder stored in the fiber.
            // SAFETY: Handle is valid if CURRENT_FIBER is set.
            unsafe {
                let fiber = &*handle.0;
                if !fiber.yielder.is_null() {
                    let yielder = &*fiber.yielder;
                    yielder.suspend(reason);
                } else {
                    // Should not happen if initialized correctly
                    panic!("Fiber yielded without initialized yielder!");
                }
            }
        } else {
            // Fallback to thread yield if not in a fiber
            std::thread::yield_now();
        }
    }

    /// Helper to get the current fiber's handle (pointer).
    pub fn current() -> Option<FiberHandle> {
        CURRENT_FIBER.get()
    }
}

unsafe impl Send for Fiber {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wait_node_alignment() {
        assert!(
            std::mem::align_of::<WaitNode>() >= 64,
            "WaitNode not aligned to 64"
        );
    }
}
