//! Library code for the SampleGraph backend API.
#![deny(
    missing_docs,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

pub mod routes;
pub use routes::*;

pub mod structs;
pub use structs::*;
