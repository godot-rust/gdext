# TODO: Default Parameters Implementation (PR #1213)

**Status**: Stalled - Needs architectural redesign of parameter handling system

**Goal**: Implement default parameter functionality for Rust functions exposed to Godot (issue #381)

## What's Been Done

- ✅ Added `#[opt(default = value)]` attribute parsing in macros
- ✅ Basic codegen infrastructure for default parameters
- ✅ Test case demonstrating desired API:
  ```rust
  #[func]
  fn function_with_default_params(
      required: i32,
      #[opt(default = "test")] string: GString,
      #[opt(default = 123)] integer: i32,
  ) -> VariantArray
  ```

## Critical Blockers

### 1. ParamTuple System Redesign
**Files**: `godot-core/src/meta/param_tuple/impls.rs:60-70`

The current `ParamTuple` trait assumes exact parameter count matching. With default parameters, we need to support variable argument counts.

**Problem**: 
- `unsafe { *args_ptr.offset($n) }` assumes `args_ptr` has exactly `Self::LEN` elements
- With defaults, we may receive fewer arguments than the function signature defines

**Solution needed**: Modify `ParamTuple` to:
- Check actual argument count vs expected count
- Fill missing arguments with default values
- Maintain type safety guarantees

### 2. Signature Processing
**Files**: `godot-core/src/meta/signature.rs:81`

The `from_varcall_args` function needs to handle variable argument counts.

**Problem**:
```rust
// Currently assumes exact match
let args = unsafe { Params::from_varcall_args(args_ptr, call_ctx)? };
```

**Solution needed**:
- Pass actual argument count to `from_varcall_args`
- Implement default value substitution logic
- Update error handling for argument count mismatches

### 3. Method Registration
**Files**: `godot-core/src/registry/method.rs`

Method metadata needs to communicate minimum required arguments vs total parameters.

**Solution needed**:
- Extend method registration to include:
  - Minimum required argument count
  - Default value metadata
  - Parameter optionality information

## Implementation Strategy

### Phase 1: Core Architecture Changes
1. **Extend ParamTuple trait**:
   - Add `MIN_ARGS: usize` constant for required parameters
   - Modify `from_varcall_args` signature to accept actual argument count
   - Implement default value injection logic

2. **Update signature handling**:
   - Pass argument count through call chain
   - Add validation for minimum required arguments
   - Preserve existing error messages for better UX

### Phase 2: Macro Integration
1. **Generate default value code**:
   - Already partially implemented in `godot-macros/src/class/data_models/func.rs:200`
   - Need to integrate with new ParamTuple system

2. **Method metadata**:
   - Register minimum vs maximum argument counts
   - Include default values in registration

### Phase 3: Testing & Validation
1. **Integration tests**:
   - Currently in `itest/rust/src/register_tests/default_parameters_test.rs`
   - Expand test coverage for edge cases

2. **Performance validation**:
   - Ensure default parameter handling doesn't impact non-default function calls
   - Benchmark argument processing overhead

## Technical Considerations

### Type Safety
- Must maintain compile-time type checking
- Default values should be validated at macro expansion time
- Runtime argument count validation should provide clear error messages

### Performance
- Zero-cost abstraction for functions without default parameters
- Minimal overhead for default parameter substitution
- Consider codegen strategies to optimize common cases

### Backward Compatibility
- Existing functions without defaults must continue working unchanged
- No breaking changes to public API
- Preserve existing error handling behavior

## Files to Focus On

**Critical**:
- `godot-core/src/meta/param_tuple/impls.rs` - Core parameter handling
- `godot-core/src/meta/signature.rs` - Function signature processing
- `godot-macros/src/class/data_models/func.rs` - Macro parsing and codegen

**Supporting**:
- `godot-core/src/registry/method.rs` - Method registration
- `godot-codegen/src/generator/default_parameters.rs` - Code generation
- `itest/rust/src/register_tests/default_parameters_test.rs` - Test validation

## Next Steps

1. **Design Review**: The parameter handling system redesign needs careful architecture planning
2. **Prototype**: Implement minimal viable changes to ParamTuple to support one optional parameter
3. **Iterate**: Expand to multiple optional parameters once core mechanism works
4. **Integration**: Connect macro-generated defaults with runtime parameter processing

## Notes from Original Author

From commit comments:
- Author (astrale) was working on this but got blocked by the complexity
- Key insight: Current safety assumptions break with variable argument counts
- Need to rethink fundamental parameter handling architecture

The implementation requires deep understanding of godot-rust's type system and memory safety guarantees.