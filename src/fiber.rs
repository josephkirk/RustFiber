//! Fiber management and execution context.
//!
//! This module provides lightweight execution contexts (coroutines) for jobs,
//! allowing them to yield execution without blocking the OS thread.

use crate::job::Job;
use corosensei::{Coroutine, CoroutineResult, Yielder};
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicPtr, AtomicU32};
use crossbeam::deque::Injector;

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

pub enum FiberInput {
    Start(Job, *const Injector<Job>, *mut Fiber),
    Resume,
}

/// The node embedded in every Fiber for intrusive lists.
/// Must be 'repr(C)' for stable pointer offsets.
#[repr(C)]
pub struct WaitNode {
    /// Atomic pointer to the next node in the list.
    pub next: AtomicPtr<WaitNode>,
    /// The handle to the fiber to be resumed.
    pub fiber_handle: FiberHandle,
    /// State of the node for race-free signaling.
    pub state: AtomicU32,
}

impl Default for WaitNode {
    fn default() -> Self {
        Self {
            next: AtomicPtr::new(std::ptr::null_mut()),
            fiber_handle: FiberHandle::null(),
            state: AtomicU32::new(NODE_STATE_RUNNING),
        }
    }
}

/// Represents a fiber - a lightweight stackful execution context.
///
/// Uses `corosensei` for context switching.
pub struct Fiber {
    /// The underlying coroutine.
    coroutine: Option<Coroutine<FiberInput, (), ()>>,
    
    /// The intrusive node used when the fiber is waiting.
    /// Wrapped in UnsafeCell because it's modified by other threads while suspended.
    pub wait_node: UnsafeCell<WaitNode>,

    /// The yielder for this fiber, set when the fiber starts.
    /// Valid only when the fiber is running.
    yielder: *const Yielder<FiberInput, ()>,
}

pub enum FiberState {
    Yielded,
    Complete,
    Panic(Box<dyn std::any::Any + Send>),
}

// Thread-local yielder accessible to Context
thread_local! {
    static CURRENT_FIBER: std::cell::Cell<Option<FiberHandle>> = const { std::cell::Cell::new(None) };
}

impl Fiber {
    /// Creates a new fiber with the given stack size.
    pub fn new(_stack_size: usize) -> Self {
        let coroutine = Coroutine::new(move |yielder, input: FiberInput| {
            if let FiberInput::Start(job, injector_ptr, fiber_ptr) = input {
                // Initialize yielder pointer in the Fiber struct.
                // SAFETY: fiber_ptr is valid and pinned (Boxed in pool).
                unsafe {
                    (*fiber_ptr).yielder = yielder as *const _;
                }

                job.execute(injector_ptr);
            } else {
                // Logic error: effectively a no-op or panic
            }
        });

        Fiber {
            coroutine: Some(coroutine),
            wait_node: UnsafeCell::new(WaitNode::default()),
            yielder: std::ptr::null(),
        }
    }

    /// Resumes the fiber with a new job or continues execution.
    pub fn resume(&mut self, input: FiberInput) -> FiberState {
        let self_ptr = self as *mut _;
        if let Some(coroutine) = self.coroutine.as_mut() {
            // Set current fiber for introspection/scheduling logic
            CURRENT_FIBER.set(Some(FiberHandle(self_ptr)));
            
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                coroutine.resume(input)
            }));
            
            CURRENT_FIBER.set(None);
            
            match result {
                Ok(CoroutineResult::Yield(_)) => FiberState::Yielded,
                Ok(CoroutineResult::Return(_)) => FiberState::Complete,
                Err(payload) => FiberState::Panic(payload),
            }
        } else {
            FiberState::Complete
        }
    }
    
    /// Resets the fiber state for reuse.
    /// Recreates the coroutine to reset the stack.
    pub fn reset(&mut self, stack_size: usize) {
        *self = Fiber::new(stack_size);
    }
    
    /// Yields execution of the current fiber.
    pub fn yield_now() {
        if let Some(handle) = CURRENT_FIBER.get() {
            // We are running inside a fiber.
            // Access the yielder stored in the fiber.
            // SAFETY: Handle is valid if CURRENT_FIBER is set.
            unsafe {
                let fiber = &*handle.0;
                if !fiber.yielder.is_null() {
                    let yielder = &*fiber.yielder;
                    yielder.suspend(());
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
