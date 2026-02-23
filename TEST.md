# Dozr Testing Strategy

This document outlines the testing strategy for the `dozr` project.

## Testing Philosophy

Our approach combines unit tests for speed and precision with integration tests for real-world validation. We test every layer of the application, from core logic to the command-line interface.

## Types of Tests

The project uses two test types:

1.  **Unit Tests**: These test small, isolated pieces of code—individual functions or methods. They run fast and give immediate feedback during development.

2.  **Integration Tests**: These verify that application components work together correctly. They ensure the full application behaves as expected.

## Unit Tests

### Location

Unit tests live in the same file as the code they test, inside a `#[cfg(test)]` module. This follows standard Rust practice and lets tests access private functions and internal state.

### Coverage

Unit tests cover:

*   **`src/conditions.rs`**: Core logic for calculating wait durations for each distribution.
*   **`src/cli.rs`**: Time parsing logic for the `at` command, tested in isolation.
*   **`src/lib.rs`**: Main application logic, tested with mock objects to verify the correct `WaitCondition` is created for each wait type.

## Integration Tests

### Location

Integration tests live in the `tests` directory at the project root. Each file is a separate crate that depends on the main `dozr` library.

### Coverage

Integration tests cover:

*   **`tests/cli.rs`**: All subcommands and arguments, error handling, and expected output.

## Running the Tests

Run all tests:

```bash
cargo test
```

This compiles and runs all unit and integration tests.

## Benchmarks

Performance benchmarks are in the `benches/` directory and use the [Criterion](https://github.com/bheisler/criterion.rs) framework. They measure the computational overhead of calculating wait durations for each distribution type.

To run benchmarks:

```bash
cargo bench
```

Benchmark results are saved to `target/criterion/` with HTML reports for visualization.

## Runnable Examples

The `examples/` directory contains runnable examples that demonstrate library usage:

```bash
# Basic duration waits with jitter
cargo run --example basic_wait

# Statistical distribution sampling
cargo run --example distributions

# Verbose progress output demonstrations
cargo run --example verbose_progress
```
