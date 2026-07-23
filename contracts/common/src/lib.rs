#![no_std]

//! # Contracts Common
//!
//! Shared types, errors, constants, and auth helpers for all Orivex contracts.
//!
//! This crate provides common functionality to reduce code duplication
//! across the workspace contracts.

pub mod auth;
pub mod constants;
pub mod errors;
pub mod two_step;
pub mod types;

// Re-export soroban-sdk for convenience
pub use soroban_sdk;
