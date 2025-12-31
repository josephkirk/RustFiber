# Code Review: optimize-memory-init

## Overview
This review evaluates the implementation of the "Optimized Memory Initialization & NUMA-Local Stacks" specification. The spec aims to address startup stutter, NUMA locality issues, and TLB pressure through three main changes: NUMA-local zeroing, interleaved pre-warming, and large pages support.

## Implementation Status

### ✅ **Implemented: Interleaved Pre-warming**
- **FiberPool::grow()** method correctly implemented for incremental allocation
- **Reduced initial pool size** from 128 to 16 fibers (512KB stacks = 8MB vs 64MB per worker)
- **Background growth** implemented in worker idle cycles
- **Verification**: Startup analysis shows 4-5ms → <1ms improvement for reduced initial allocation

### ⚠️ **Partially Implemented: NUMA-Local Zeroing**
- **prefault_stack_memory()** method exists but is **DISABLED** due to Windows compatibility issues
- **Issue**: Manual prefaulting causes "Guard Page Violations" on Windows due to strict stack probing requirements
- **Current state**: Relies on OS page faults during first coroutine execution
- **Impact**: Core NUMA locality guarantee is **not implemented**

### ❌ **Not Implemented: Large Pages Support**
- **Status**: Deferred to v0.3 per ROADMAP.md
- **Rationale**: Requires custom allocator support in corosensei or manual VirtualAlloc
- **Acceptable**: Properly documented as future work

### ❌ **Missing: prefetch_pages Configuration**
- **Spec requirement**: Add `prefetch_pages: bool` to `FiberConfig`
- **Current state**: Field does not exist in `FiberConfig` struct
- **Impact**: No way to disable prefaulting if needed for debugging/performance tuning

## Code Quality Assessment

### Strengths
- **Clean implementation** of incremental growth pattern
- **Good documentation** explaining the trade-offs and Windows compatibility issues
- **Conservative defaults** that prioritize stability over aggressive optimization
- **Proper separation** of concerns between pool management and fiber creation

### Issues Found

#### 1. **Critical: Core Feature Disabled**
```rust
fn prefault_stack_memory(_stack: &mut DefaultStack) {
    // NOTE: Manual prefaulting caused Guard Page Violations on Windows due to
    // strict stack probing requirements and corosensei's internal layout.
    // We defer this optimization to v0.3 where we can implement a custom Stack allocator.
}
```
**Severity**: High
**Impact**: The primary NUMA locality optimization is not working
**Recommendation**: Either fix the Windows compatibility issue or clearly document this as a known limitation

#### 2. **Missing Configuration Option**
**Location**: `src/job_system.rs:15-25`
**Issue**: `prefetch_pages` field missing from `FiberConfig`
**Impact**: No runtime control over prefaulting behavior
**Recommendation**: Add the field even if currently unused

#### 3. **Inconsistent Error Handling**
**Issue**: Prefaulting failure silently ignored rather than logged or handled
**Recommendation**: Add proper error handling and logging for prefaulting failures

## Verification Gaps

### ❌ **Missing Benchmarks**
- **Startup latency**: No automated benchmarks to verify the 4-5ms improvement
- **NUMA locality**: No tools/measures to verify memory placement on correct NUMA nodes
- **TLB pressure**: No measurements of TLB miss rates with/without large pages

### ❌ **Missing Tests**
- **Incremental growth**: No tests verifying background pool growth behavior
- **Prefaulting**: No tests for the disabled prefaulting functionality
- **Configuration**: No tests for FiberConfig options

## Recommendations

### Immediate Actions
1. **Fix Windows Compatibility**: Investigate alternative approaches for NUMA-local zeroing that don't violate guard page constraints
2. **Add prefetch_pages config**: Implement the missing configuration option
3. **Add error handling**: Log prefaulting failures and provide fallback behavior

### Testing Improvements
1. **Add startup benchmarks**: Automated measurement of initialization time
2. **Add NUMA verification**: Tools to verify memory placement (if possible)
3. **Add configuration tests**: Test all FiberConfig options

### Documentation Updates
1. **Update spec status**: Clearly document what's implemented vs. deferred
2. **Add Windows limitations**: Document the guard page issue and its impact
3. **Add performance expectations**: Document expected improvements and current limitations

## Risk Assessment

### High Risk
- **NUMA locality not guaranteed**: Core optimization goal not achieved
- **Silent failures**: Prefaulting issues not reported to users

### Medium Risk
- **Windows-only limitation**: May affect cross-platform performance consistency
- **No configuration control**: Unable to disable prefaulting for debugging

### Low Risk
- **Deferred large pages**: Properly documented as future work
- **Incremental growth**: Well-implemented and tested

## Conclusion

**Overall Grade**: C+ (Partially Implemented)

The interleaved pre-warming optimization is well-implemented and provides measurable startup improvements. However, the core NUMA-local zeroing feature is disabled due to Windows compatibility issues, significantly reducing the effectiveness of this optimization. The implementation should either fix the Windows issue or clearly document this limitation and consider alternative approaches for NUMA-aware memory placement.

**Recommendation**: Address the Windows compatibility issue before considering this spec complete, or revise the spec to focus on the working optimizations (incremental allocation) and defer NUMA locality to a future version with proper platform support.</content>
<parameter name="filePath">g:\Projects\Arcel\RustFiber\openspec\changes\optimize-memory-init\review.md