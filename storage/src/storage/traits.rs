// Copyright (C) 2019-2022 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use super::{DataID, DataMap};

use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::{borrow::Borrow, path::Path};

/// A trait applicable to all access modes of database operations.
pub trait StorageAccess: Send + Sync + 'static {}
/// A marker trait for storage functionalities require write access.
pub trait StorageReadWrite: StorageAccess {}

/// A marker type for objects with read-only storage capabilities.
#[derive(Clone, Copy)]
pub struct ReadOnly;
/// A marker type for objects with read-write storage capabilities.
#[derive(Clone, Copy)]
pub struct ReadWrite;

// Both `ReadOnly` and `ReadWrite` are storage access modes...
impl StorageAccess for ReadOnly {}
impl StorageAccess for ReadWrite {}

// But only `ReadWrite` implements `StorageReadWrite`
impl StorageReadWrite for ReadWrite {}
