//! SMPTE ST 336 KLV (Key-Length-Value) primitives.
//!
//! Provides BER length decoding, [`KlvStream`] for sequential triplet reading,
//! [`LocalSet`] for local-tag-keyed sets, and [`LocalTagRegister`].

pub mod ber;
pub mod error;
pub mod local_set;
pub mod stream;
pub mod triplet;

pub use error::KlvError;
pub use local_set::{LocalSet, LocalTagRegister};
pub use stream::KlvStream;
pub use triplet::MemoryTriplet;
