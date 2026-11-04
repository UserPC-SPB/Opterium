//! toolfmt — universal tool-output formatter and MCP gateway.
//!
//! The agent calls a tool; the output reaches it already in the format the
//! agent chose once, per tool, in a standard (`standards/<server>.<tool>.yml`).
//! No standard: the output passes through untouched. See SPEC.md.

// Nested `if let` reads like the rule it implements; same style as the parser.
#![allow(clippy::collapsible_if)]

pub mod capture;
pub mod catalog;
pub mod config;
pub mod demo;
pub mod detect;
pub mod gateway;
pub mod http;
pub mod image;
pub mod json;
pub mod meta;
pub mod ops;
pub mod pipeline;
pub mod render;
pub mod replay;
pub mod rpc;
pub mod standard;
pub mod tpl;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
