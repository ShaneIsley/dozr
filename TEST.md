# Dozr Testing Strategy

This document outlines the testing strategy for the `dozr` project, providing a clear guide for developers on how to write and run tests.

## Testing Philosophy

We believe in a comprehensive testing approach that combines the speed and precision of unit tests with the real-world validation of integration tests. Our goal is to ensure that every part of the application is well-tested, from the core logic to the command-line interface.

## Types of Tests

We use two primary types of tests in this project:

1.  **Unit Tests**: These tests focus on small, isolated pieces of code, such as individual functions or methods. They are fast, easy to write, and provide immediate feedback during development.

2.  **Integration Tests**: These tests verify that the different components of the application work together correctly. They are essential for ensuring that the application as a whole behaves as expected.

## Unit Tests

### Location

Unit tests are located in the same file as the code they are testing, inside a `#[cfg(test)]` module. This is a standard practice in Rust and allows the tests to access private functions and internal state.

### Coverage

Our unit tests cover the following areas:

*   **`src/conditions.rs`**: The core logic for calculating wait durations for each distribution is thoroughly tested.
*   **`src/cli.rs`**: The time parsing logic for the `at` command is tested in isolation.
*   **`src/lib.rs`**: The main application logic is tested using mock objects to ensure that the correct `WaitCondition` is created for each wait type.

## Integration Tests

### Location

Integration tests are located in the `tests` directory at the root of the project. Each file in this directory is treated as a separate crate that depends on the main `dozr` library.

### Coverage

Our integration tests cover the following areas:

*   **`tests/cli.rs`**: The command-line interface is extensively tested to ensure that all subcommands and arguments are parsed correctly, that error conditions are handled gracefully, and that the application produces the expected output.

## Running the Tests

To run all the tests in the project, use the following command:

```bash
cargo test
```

This command will compile and run all the unit and integration tests, providing a comprehensive overview of the project's health.

## Benchmarks

Performance benchmarks are located in the `benches/` directory and use the [Criterion](https://github.com/bheisler/criterion.rs) framework. These benchmarks measure the computational overhead of calculating wait durations for each distribution type.

To run the benchmarks:

```bash
cargo bench
```

Benchmark results are saved to `target/criterion/` with HTML reports for visualization.

## Runnable Examples

The `examples/` directory contains runnable examples demonstrating library usage:

```bash
# Basic duration waits with jitter
cargo run --example basic_wait

# Statistical distribution sampling
cargo run --example distributions

# Verbose progress output demonstrations
cargo run --example verbose_progress
```
