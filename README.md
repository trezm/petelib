# Petelib

Petelib is a collection of Rust libraries providing various utilities and macros for web development and general programming tasks. This workspace contains multiple crates that can be used independently or together.

## Project Structure

The project consists of three main crates:

### thruster-auth
Authentication utilities for the Thruster web framework.

### thruster-macros
Procedural macros for the Thruster web framework, including Prisma integration.

### usual-macros
General-purpose macros for common programming tasks.

## Installation

Add the desired crate to your `Cargo.toml`:

```toml
[dependencies]
thruster-auth = { git = "https://github.com/yourusername/petelib" }
thruster-macros = { git = "https://github.com/yourusername/petelib" }
usual-macros = { git = "https://github.com/yourusername/petelib" }
```

## Development

### Prerequisites

- Rust (latest stable version)
- Cargo

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.