// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

#![allow(clippy::as_conversions)]
#![allow(clippy::new_without_default)]

#[cfg(windows)]
mod intel;

#[cfg(windows)]
pub mod pe;

pub mod block;
