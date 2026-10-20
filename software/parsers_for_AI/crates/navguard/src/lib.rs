//! navguard: project navigation for AI agents. Scans a project, keeps a
//! symbol index with a Merkle-checked cache and renders navigator maps.
//!
//! * [`api`]: data-returning functions for tools (map, symbols, symbol
//!   search, line ranges, text search, TODOs, changes since the last scan).
//! * [`cli`]: the `navguard` command line (`any_txt_parser nav ...`).
//! * [`parser`]: the heuristic symbol indexer used by both.

pub mod api;
pub mod build_rs;
pub mod cache;
pub mod cli;
pub mod config;
pub mod navigator;
pub mod parser;
pub mod paths;
pub mod responder;
pub mod scanner;
pub mod step;
pub mod text;
pub mod triangle;
pub mod validate;
pub mod watch;
pub mod write;
