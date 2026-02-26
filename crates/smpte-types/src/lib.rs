//! SMPTE identifier and primitive types.
//!
//! Provides [`Ul`], [`Auid`], [`Uuid`], [`Umid`], and [`HalfFloat`]
//! for use across the regxmllib-rs workspace.

pub mod auid;
pub mod ul;
pub mod umid;
pub mod uuid;

pub use auid::Auid;
pub use ul::Ul;
pub use umid::Umid;
pub use uuid::Uuid;
pub use half::f16 as HalfFloat;
