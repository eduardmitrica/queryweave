//! Tests for the output of the [`queryweave_derive::Filters`] proc macro, on
//! root models, on the structs mirroring them, on chains of mirrors, and on
//! each integration of a mirror on its own.

#[path = "filters/chain.rs"]
mod chain;
#[path = "filters/integrations.rs"]
mod integrations;
#[path = "filters/mirror.rs"]
mod mirror;
#[path = "filters/renames.rs"]
mod renames;
#[path = "filters/root.rs"]
mod root;
