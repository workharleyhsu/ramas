//! `ramas`: interactive tree-style disk usage analyzer.
//!
//! The library holds all logic so it can be tested without a terminal;
//! `main.rs` only wires the CLI to it.

pub mod actions;
pub mod cli;
pub mod config;
pub mod export;
pub mod scan;
pub mod tree;
pub mod ui;
