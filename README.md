# An enum_dispatch based pest parser

[![crates.io](https://img.shields.io/crates/v/enum_dispatch_pest_parser)](https://crates.io/crates/enum_dispatch_pest_parser)
[![docs.rs](https://docs.rs/enum_dispatch_pest_parser/badge.svg)](https://docs.rs/enum_dispatch_pest_parser)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE)

A procedural macro for generating pest-based parsers with integrated `enum_dispatch` support, enabling type-safe static dispatch of parsing rules.

## Features

- 🚀 **Automatic Parser Generation** - Convert pest grammar files into executable parsers
- 🧩 **Rule-specific Structs** - Generate zero-sized types for each grammar rule
- ⚡ **Static Dispatch** - Leverage `enum_dispatch` for efficient method resolution
- 🔧 **Trait-based Interface** - Unified API across all parsing rules

## Usage
1. Add dependencies to `Cargo.toml`:
    ```toml
    [dependencies]
    pest = { version = "2.5", features = ["derive"] }
    enum_dispatch = "0.3"
    enum_dispatch_pest_parser = { version = "0.1" }  # This crate
    ```

2. Define a trait interface for parser rules
3. Apply the `#[pest_parser]` attribute to a struct

## Example
```rust
use anyhow::Result;
use enum_dispatch::enum_dispatch;
use enum_dispatch_pest_parser::pest_parser;
use pest::Parser;

// Define parser trait interface
#[enum_dispatch]
trait ParserInterface {
    fn parse_rule(&self, arg: &str) -> Result<()>;
}

// Generate parser implementation
#[pest_parser(grammar = "grammar.pest", interface = "ParserInterface")]
pub struct LanguageParser;

// Implement trait for individual rules
#[derive(Default)]
struct Statement;
impl ParserInterface for Statement {
    fn parse_rule(&self, arg: &str) -> Result<()> {
        // do something here...
        Ok(())
    }
}

// Usage example
fn main() -> Result<()> {
    let content = read_to_string("input.txt")?;
    let node = LanguageParser::parse(Rule::Statement(Statement {}), &content)?;
    node.parse_rule("argument")?;
    // Dispatches to Statement::parse_rule automatically
}
```

## Implementation Notes
### Code Generation Phases
1. **Base Parser Generation**: Uses `pest_generator` to create initial parsing code
2. **Struct Generation**:
   - Extracts `enum Rule` definition from generated code
   - Creates unit structs for each variant (e.g., `struct Statement;`)
3. **Dispatch Integration**:
   - Inserts `#[enum_dispatch]` attribute on `enum Rule`
   - Modifies pattern matching to handle struct wrappers
   - Adjusts rule instantiation syntax

## Safety & Compatibility
1. **pest Version Locking**:
   - Tightly coupled with pest's code generation output
   - Tested with pest 2.5.7 - may break with newer versions
2. **Fragile Regex Modifications**:
   - Uses regular expressions for code transformation
   - May fail with unusual formatting or comments
3. **Trait Implementation**:
   - Users MUST manually implement the trait for generated structs
   - Structs are public and reside in root module

## Debugging Tips
1. Inspect generated code in `pest_parser` using:
   ```rust
   println!("{}", raw_codes);  // Add temporary debug output
   ```
2. Verify `enum Rule` extraction boundaries
3. Check regex replacements for rule wrapping
