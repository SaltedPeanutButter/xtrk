//! Collection of peculiar yet useful (sometimes) utilities written in Rust.
//!
//! `xtrk` is a hobby-ist project and is not intended for serious usage.
//! Use at your own risk!
//!
//! # Features
//!
//! All features are, by default, enabled through feature flags. Feel free to `--no-default-features`
//! and enable only the features you need.
//!
//! - [sten] for steganography
//! - [crypt] for crytography
//!
//! Do refer to individual module's documentation for more information.
#![deny(clippy::all)]

/// Collection of cryptographic utilities.
///
/// `xtrk` allows simple, and probably not very secure, symmetrically or asymmetrically,
/// cryptographic operations.
///
/// How amazing would it be if you do not have to install a bunch of crates just to
/// protect some teeny tiny text, pfft?
///
/// You should probably refrain from using this those. It is designed for convenience, not performance
/// nor security. Look at crates like `ring` or `rust-crypto` for serious cryptographic operations.
#[cfg(feature = "crypt")]
pub mod crypt;

/// Collection of steganography utilities.
///
/// Stenography, to my inept understanding, is basically hiding stuff in plain sight. It allows you to
/// encode and decode messages stored in images, so that you can send secret messages to your friends
/// without anyone else knowing.
#[cfg(feature = "sten")]
pub mod sten;
