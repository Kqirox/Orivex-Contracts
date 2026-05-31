#![no_std]
use soroban_sdk::contract;

pub mod types;

pub use types::{DataKey, Proposal};

#[contract]
pub struct Governance;
