# Expose Simplified Config and API

## Summary
Improve library usability by exposing configuration options and simplifying the API for common use cases, while maintaining zero performance overhead for advanced users.

## Motivation
Current API requires users to understand low-level details like fiber stack sizes, pool sizes, and pinning strategies. This creates a barrier for new users and leads to suboptimal defaults being used. By providing simplified constructors, presets, and better documentation of best use cases, we can make the library more accessible without compromising performance.

## Goals
- Provide high-level presets for common scenarios (e.g., gaming, data processing)
- Add a builder pattern for gradual configuration
- Document best practices and use cases
- Maintain zero-cost abstractions for advanced users

## Non-Goals
- Change internal architecture
- Add runtime configuration (init-time only)
- Break existing API

## Impact
- Improves developer experience
- Reduces configuration errors
- Maintains performance characteristics