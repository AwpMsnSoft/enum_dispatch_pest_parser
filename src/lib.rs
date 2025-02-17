//! A procedural macro for generating pest-based parsers with integrated `enum_dispatch` support.
//!
//! This crate automates the creation of type-safe parser implementations by combining pest grammar definitions
//! with static dispatch via the `enum_dispatch` crate. It generates both parser rules and associated data structures
//! while establishing a trait-based interface for unified rule handling.
//！
//!
//! ## Features
//! - **Automatic Parser Generation**: Converts pest grammar files into executable parsing logic
//! - **Rule-specific Structs**: Generates zero-sized structs for each grammar rule
//! - **Static Dispatch Integration**: Implements `enum_dispatch`-powered trait unification
//! - **Trait-based Interface**: Creates a unified API for all parsing rules
//!
//! ## Usage
//! 1. Add dependencies to `Cargo.toml`:
//!    ```toml
//!    [dependencies]
//!    pest = "2.5"
//!    pest_derive = "2.5"
//!    enum_dispatch = "0.3"
//!    enum_dispatch_pest_parser = { version = "0.1" }  # This crate
//!    ```
//!
//! 2. Define a trait interface for parser rules
//! 3. Apply the `#[pest_parser]` attribute to a struct
//!
//! ## Example
//! ```rust
//! use anyhow::Result;
//! use enum_dispatch::enum_dispatch;
//! use enum_dispatch_pest_parser::pest_parser;
//! use pest::Parser;
//!
//! // Define parser trait interface
//! #[enum_dispatch]
//! trait ParserInterface {
//!     fn parse_rule(&self, arg: &str) -> Result<()>;
//! }
//!
//! // Generate parser implementation
//! #[pest_parser(grammar = "grammar.pest", interface = "ParserInterface")]
//! pub struct LanguageParser;
//!
//! // Implement trait for individual rules
//! #[derive(Default)]
//! struct Statement;
//! impl ParserInterface for Statement {
//!     fn parse_rule(&self, arg: &str) -> Result<()> {
//!         /* do something here */
//!         Ok(())
//!     }
//! }
//!
//! // Usage example
//! fn main() -> Result<()> {
//!     let content = read_to_string("input.txt")?;
//!     let _ = LanguageParser::parse(Rule::Statement(Statement {}), &content)?;
//!     node.parse_rule("argument")?;
//!     // Dispatches to Statement::parse_rule automatically
//! }
//! ```
//!
//! ## Implementation Notes
//! ### Code Generation Phases
//! 1. **Base Parser Generation**: Uses `pest_generator` to create initial parsing code
//! 2. **Struct Generation**:
//!    - Extracts `enum Rule` definition from generated code
//!    - Creates unit structs for each variant (e.g., `struct Statement;`)
//! 3. **Dispatch Integration**:
//!    - Inserts `#[enum_dispatch]` attribute on `enum Rule`
//!    - Modifies pattern matching to handle struct wrappers
//!    - Adjusts rule instantiation syntax
//!
//! ## Safety & Compatibility
//! 1. **pest Version Locking**:
//!    - Tightly coupled with pest's code generation output
//!    - Tested with pest 2.5.4 - may break with newer versions
//! 2. **Fragile Regex Modifications**:
//!    - Uses regular expressions for code transformation
//!    - May fail with unusual formatting or comments
//! 3. **Trait Implementation**:
//!    - Users MUST manually implement the trait for generated structs
//!    - Structs are public and reside in root module
//!
//! ## Debugging Tips
//! 1. Inspect generated code using:
//!    ```rust
//!    println!("{}", raw_codes);  // Add temporary debug output
//!    ```
//! 2. Verify `enum Rule` extraction boundaries
//! 3. Check regex replacements for rule wrapping
//!
//! ## Limitations
//! - Requires nightly Rust for procedural macros
//! - Rule structs pollute root namespace
//! - Limited error reporting for malformed grammars

extern crate pest_generator;
extern crate proc_macro;
extern crate quote;
extern crate regex;
extern crate syn;

use pest_generator::derive_parser;
use proc_macro::TokenStream;
use quote::quote;
use regex::Regex;
use std::str::FromStr;
use syn::{
    parse_macro_input, parse_str, punctuated::Punctuated, Expr, ItemEnum, ItemStruct, Lit,
    MetaNameValue,
};

fn enum_dispatch_tag_generator(nodes: TokenStream) -> TokenStream {
    let raw_codes = derive_parser(nodes.into(), false).to_string();

    // NOTE: the auto-generated code by `pest` is not stable. if compile error occurs here,
    // check the raw_codes and find the correct position, and modify the `enum_start`'s value
    // and `enum_end`'s value manually.
    //
    // pest 2.5.4 example code
    // ```rust
    // #[allow(dead_code, non_camel_case_types, clippy :: upper_case_acronyms)]
    // #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)] pub enum
    // Rule
    // {
    //     #[doc = "End-of-input"] EOI, r#Script, r#Statement, r#Command,
    //     r#CmdMessage, r#Arguments, r#Argument, r#TrivalArgument,
    //     r#NullableArgument, r#NamedArgument, r#Strings, r#Number, r#Identifier
    // } impl Rule
    // {
    //     pub fn all_rules() -> & 'static [Rule]
    //     {
    //         &
    //         [Rule :: r#Script, Rule :: r#Statement, Rule :: r#Command, Rule ::
    //         r#CmdMessage, Rule :: r#Arguments, Rule :: r#Argument, Rule ::
    //         r#TrivalArgument, Rule :: r#NullableArgument, Rule :: r#NamedArgument,
    //         Rule :: r#Strings, Rule :: r#Number, Rule :: r#Identifier]
    //     }
    // }
    // ```
    let enum_start =
        raw_codes.find("#[allow(dead_code, non_camel_case_types, clippy :: upper_case_acronyms)]");
    let enum_end = raw_codes.find("}");

    if let (Some(enum_start), Some(enum_end)) = (enum_start, enum_end) {
        let raw_enum = String::from(&(&raw_codes)[enum_start..=enum_end]);
        let enums = parse_str::<ItemEnum>(&raw_enum)
            .expect("cannot parse cutted `pest`'s enum definition string into enum.")
            .variants
            .into_iter()
            .map(|ident| ident.ident)
            .map(|ident| quote! { #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)] pub struct #ident; });

        quote! { #(#enums)* }.into()
    } else {
        unreachable!(
            "cannot find `pub enum Rule` in `pest`'s auto-generated code. this error might be a false positive in rust-analyzer,
            so please refer to the compilation results."
        );
    }
}

fn enum_dispatch_generated_enum_hooker(nodes: TokenStream, interface: String) -> TokenStream {
    let mut raw_codes = derive_parser(nodes.into(), true).to_string();

    // find `pub enum Rule`'s derive list.
    // only `enum Rule` has `#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]` in raw_codes.
    // and we wanna insert `#[enum_dispatch]` before it.
    let enum_insert_pos = raw_codes.find("#[derive(").unwrap();
    // TODO: maybe `WgscriptParserCommand` can be a proc_macro argument?
    raw_codes.insert_str(
        enum_insert_pos,
        &format!("#[enum_dispatch({interface})]\r\n"),
    );

    // because of the raw enum is hooked, the `match` statement of raw enum need hooked too.
    // generaly, the `pest` macro will generate statement like follows:
    // ```
    //  match rule
    // {
    //     Rule :: r#script => rules :: r#script(state), Rule ::
    //     r#statement => rules :: r#statement(state), Rule :: EOI =>
    //     rules :: EOI(state)
    // }
    // ```
    // but obviewsly, now `Rule :: r#script` is a unit struct instead of tuple variant, so it should be
    // ```
    // match rule
    // {
    //      Rule::r#script(_)  => rules::r#script(state),
    //      Rule::r#statement(_) => rules::r#statement(state),
    //      Rule::EOI(_) => rules::EOI(state)
    // }
    //```
    // Here is a trick that only this match block has token `=>`, so just insert `(_)` before every `=>`.
    raw_codes = raw_codes.replace("=>", "(_) =>");

    // then, after we changed match statement, we need to hook the `pest`'s inner implemention:
    // the enum it self should be like `Rule::r#Script(crate::r#Script)`, but the `pest`'s auto-generated code is `Rule::r#Script`
    // the function signature of `State::rule` is `pub fn rule<F>(mut self: Box<Self>, rule: R, f: F) -> ParseResult<Box<Self>>`
    // and the function call like `state.rule(Rule :: r#Statement, ...)` should be `state.rule(Rule :: r#Statement(crate::r#Statement{}), ...). Same as `Rule::all_rules()` methods.
    // this time we have no tricks but use regex normally, replace all `r#$n,` to `r#$n(crate::$n {})` can solve it.
    // replace `Rule::r#$n,` to `Rule::r#$n(crate::r#$n {})` first, then `r#$n,` to `r#$n(crate::r#$n)` (for enum definition).
    // NOTE: without `crate::*` it will cause name conflict (`$n` can be both `crate::Rule::$n` or `crate::$n`)
    let regex = Regex::new(
        r"(?x)
            Rule[[:blank:]]*::[[:blank:]]*r[[:blank:]]*\#[[:blank:]]*
            (?P<n>(\w+|r\#\w+)),",
    )
    .unwrap();
    let raw_codes = regex.replace_all(&raw_codes, "Rule::r#$n(crate::r#$n {}), ");
    let regex = Regex::new(
        r"(?x)
            r[[:blank:]]*\#[[:blank:]]*
            (?P<n>(\w+|r\#\w+)),",
    )
    .unwrap();
    let raw_codes = regex.replace_all(&raw_codes, "r#$n(crate::r#$n), ");

    // there are two f**king special cases:
    // 1. `Rule::EOI`. `r#$n` cannot match it.
    // 2. the last `Rule::r#$n` in enum definition. `r#$n,` cannot match it because of the comma.
    //      in enum definition it's r#$n}` and in `pub fn all_rules()` it's `Rule::r#$n]`.
    // TODO: maybe less regex is possible but too lazy to do it. you don't care about compile time, right?
    let regex = Regex::new(
        r"(?x)
            Rule[[:blank:]]*::[[:blank:]]*EOI,",
    )
    .unwrap();
    let raw_codes = regex.replace_all(&raw_codes, "Rule::EOI(crate::EOI {}), ");
    let regex = Regex::new(
        r"(?x)
            EOI[[:blank:]]*\,",
    )
    .unwrap();
    let raw_codes = regex.replace_all(&raw_codes, "EOI(crate::EOI), ");
    let regex = Regex::new(
        r"(?x)
            Rule[[:blank:]]*::[[:blank:]]*r[[:blank:]]*\#[[:blank:]]*
            (?P<n>(\w+|r\#\w+))\s*\]",
    )
    .unwrap();
    let raw_codes = regex.replace_all(&raw_codes, "Rule::r#$n(crate::r#$n {})]");
    let regex = Regex::new(
        r"(?x)
            r[[:blank:]]*\#[[:blank:]]*
            (?P<n>(\w+|r\#\w+))\s*\}",
    )
    .unwrap();
    let raw_codes = regex.replace_all(&raw_codes, "r#$n(crate::r#$n)}");

    TokenStream::from_str(&raw_codes).expect("illegal code format found")
}

fn get_pest_parser_argument(arg: MetaNameValue) -> (String, String) {
    let key = if let Some(ident) = arg.path.get_ident() {
        ident.to_string()
    } else {
        panic!("key of argument must be an identifier");
    };
    let value = if let Expr::Lit(lit) = arg.value {
        if let Lit::Str(lit_str) = lit.lit {
            lit_str.value()
        } else {
            panic!("value of argument must be a string literal");
        }
    } else {
        panic!("value of argument must be a string literal");
    };
    (key, value)
}

/// Generates a pest-based parser with `enum_dispatch` integration for static method dispatch.
///
/// This procedural macro automates the creation of a parser from a pest grammar file while generating
/// zero-sized structs for each grammar rule. These structs implement a specified trait interface through
/// `enum_dispatch`, enabling efficient static dispatch of parsing methods.
#[proc_macro_attribute]
pub fn pest_parser(arg: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    let vis = input.vis;
    let ident = input.ident;

    let args =
        parse_macro_input!(arg with Punctuated::<MetaNameValue, syn::Token![,]>::parse_terminated);

    assert!(
        args.len() == 2,
        "expected 2 arguments, but got {}",
        args.len()
    );

    let (arg0_key, mut arg0_value) = get_pest_parser_argument(args[0].clone());
    let (arg1_key, mut arg1_value) = get_pest_parser_argument(args[1].clone());

    assert!(
        (arg0_key.as_str(), arg1_key.as_str()) == ("grammar", "interface")
            || (arg0_key.as_str(), arg1_key.as_str()) == ("interface", "grammar"),
        "expected arguments are `grammar` and `interface`, but got `{}` and `{}`",
        arg0_key.clone(),
        arg1_key.clone()
    );

    if arg0_key == String::from("interface") {
        std::mem::swap(&mut arg0_value, &mut arg1_value);
    }

    let grammar_file = arg0_value.clone();
    let interface = arg1_value.clone();

    let mut ast_part1: TokenStream = quote! {
        #vis struct #ident;
    }
    .into();

    let ast_part2 = enum_dispatch_tag_generator(
        quote! {
            #[grammar = #grammar_file]
            #vis struct #ident;
        }
        .into(),
    );

    let ast_part3: TokenStream = enum_dispatch_generated_enum_hooker(
        quote! {
            #[derive(Parser)]
            #[grammar = #grammar_file]
            #vis struct #ident;
        }
        .into(),
        interface,
    );

    ast_part1.extend(vec![ast_part2, ast_part3]);
    ast_part1
}
