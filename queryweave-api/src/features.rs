//! Which of this crate's features are enabled, for the code generated for a
//! mirror to check.
//!
//! An integration naming this crate's types (e.g. `serde`, whose mirror's
//! sort details hold a `SortDirectionDto`) needs the feature of the same
//! name, which derives its traits on them. Each feature has a trait here,
//! implemented for [`SortDirectionDto`](crate::SortDirectionDto) only when
//! the feature is enabled; a mirror opting into the integration requires it,
//! so a disabled feature is reported at the mirror's flag, naming the feature
//! to enable, rather than only as traits missing on `SortDirectionDto`.

/// Implemented when the `serde` feature is enabled.
#[diagnostic::on_unimplemented(
    message = "the `serde` integration needs `queryweave-api`'s `serde` feature",
    label = "`queryweave-api` is built without its `serde` feature",
    note = "enable it: `queryweave-api = {{ version = \"..\", features = [\"serde\"] }}`"
)]
pub trait Serde {}

#[cfg(feature = "serde")]
impl Serde for crate::SortDirectionDto {}

/// Implemented when the `utoipa` feature is enabled.
#[diagnostic::on_unimplemented(
    message = "the `utoipa` integration needs `queryweave-api`'s `utoipa` feature",
    label = "`queryweave-api` is built without its `utoipa` feature",
    note = "enable it: `queryweave-api = {{ version = \"..\", features = [\"utoipa\"] }}`"
)]
pub trait Utoipa {}

#[cfg(feature = "utoipa")]
impl Utoipa for crate::SortDirectionDto {}

/// Implemented when the `schemars` feature is enabled.
#[diagnostic::on_unimplemented(
    message = "the `schemars` integration needs `queryweave-api`'s `schemars` feature",
    label = "`queryweave-api` is built without its `schemars` feature",
    note = "enable it: `queryweave-api = {{ version = \"..\", features = [\"schemars\"] }}`"
)]
pub trait Schemars {}

#[cfg(feature = "schemars")]
impl Schemars for crate::SortDirectionDto {}
