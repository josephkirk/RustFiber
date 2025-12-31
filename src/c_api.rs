use crate::JobSystem;
use std::sync::Arc;

/// C-API Handle for JobSystem (Opaque pointer to Arc<JobSystem>)
///
/// We use `*mut JobSystem` in function signatures for C readability,
/// but the runtime value is actually a pointer to `Box<Arc<JobSystem>>`.
/// This wrapper ensures we handle the `Arc` reference counting correctly
/// across the FFI boundary.
#[unsafe(no_mangle)]
/// Creates a new JobSystem handle for C API.
///
/// # Safety
/// The caller is responsible for eventually calling `JobSystem_Destroy` to leak memory properly
/// (via `Box::from_raw`) or valid usage. Actually destruction should properly drop.
pub unsafe extern "C" fn JobSystem_Create(num_threads: usize) -> *mut JobSystem {
    let js = if num_threads == 0 {
        JobSystem::default()
    } else {
        JobSystem::new(num_threads)
    };
    let arc = Arc::new(js);
    Box::into_raw(Box::new(arc)) as *mut JobSystem
}

#[unsafe(no_mangle)]
/// Destroys the JobSystem handle.
///
/// # Safety
/// `handle` must be a valid pointer returned by `JobSystem_Create`.
/// It must not be used after this call.
pub unsafe extern "C" fn JobSystem_Destroy(handle: *mut JobSystem) {
    if handle.is_null() {
        return;
    }
    let arc_ptr = handle as *mut Arc<JobSystem>;
    // Dropping the Box<Arc> decrements the strong count.
    unsafe { drop(Box::from_raw(arc_ptr)) };
}

/// Helper for Rust FFI consumers to reconstruct Arc from the opaque C handle.
///
/// # Safety
/// The handle must be a valid pointer returned by `JobSystem_Create`.
/// This function CLONES the Arc, incrementing the reference count.
/// The caller is responsible for eventually dropping the returned Arc.
/// The original handle remains valid until `JobSystem_Destroy` is called.
pub unsafe fn job_system_from_handle(handle: *mut JobSystem) -> Option<Arc<JobSystem>> {
    if handle.is_null() {
        return None;
    }
    let arc_ptr = handle as *mut Arc<JobSystem>;
    // We strictly use (*ptr).clone() to get a new ArcOwned reference
    // We DO NOT take ownership of the Box.
    Some(unsafe { (*arc_ptr).clone() })
}
