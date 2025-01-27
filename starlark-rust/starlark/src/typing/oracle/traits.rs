/*
 * Copyright 2019 The Starlark in Rust Authors.
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

// This makes for a better API.
#![allow(clippy::result_unit_err)]

use crate::typing::ty::Arg;
use crate::typing::ty::Ty;
use crate::typing::ty::TyName;

/// Callbacks which provide types when typechecking a module.
///
/// The instance for slices/`Vec` allow composing a series of oracles
/// which are tried in order until one succeeds.
#[allow(unused_variables)] // Otherwise the types are bad in completions
pub trait TypingOracle {
    /// Given a type and a `.` attribute, what is its type.
    /// Return [`Err`] to indicate we _know_ this isn't a valid attribute.
    /// Return [`None`] if we aren't sure.
    fn attribute(&self, ty: &Ty, attr: &str) -> Option<Result<Ty, ()>> {
        None
    }

    /// Given a symbol in the global environment, what is its type.
    /// Return [`Err`] if we _know_ this isn't a valid builtin.
    /// Return [`None`] if we aren't sure.
    fn builtin(&self, name: &str) -> Option<Result<Ty, ()>> {
        None
    }

    /// A call to a builtin function. Only used for functions where the return type
    /// is dependent on the arguments in a way that can't be expressed by [`TypingOracle::builtin`].
    /// Return [`None`] to defer to [`TypingOracle::builtin`].
    /// Return [`Err`] with a message if these arguments are type-incorrect.
    fn builtin_call(&self, name: &str, args: &[Arg]) -> Option<Result<Ty, String>> {
        None
    }

    /// If we require the first type, but got the second type, is that OK?
    /// Usually its OK if the requirement is a subtype of the one we got.
    fn subtype(&self, require: &TyName, got: &TyName) -> bool {
        false
    }
}

/// Returns no information for everything.
pub struct OracleNone;

impl TypingOracle for OracleNone {}

/// Declare that there are no builtins, usually used at the end of a [`Vec`].
pub struct OracleNoBuiltins;

impl TypingOracle for OracleNoBuiltins {
    fn builtin(&self, _name: &str) -> Option<Result<Ty, ()>> {
        Some(Err(()))
    }
}

/// Declare that there are no attributes, usually used at the end of a [`Vec`].
pub struct OracleNoAttributes;

impl TypingOracle for OracleNoAttributes {
    fn attribute(&self, _ty: &Ty, _attr: &str) -> Option<Result<Ty, ()>> {
        Some(Err(()))
    }
}

/// Sequence of oracles, first one that returns [`Some`] wins.
pub struct OracleSeq<T>(pub Vec<T>)
where
    T: TypingOracle;

impl<T: TypingOracle> TypingOracle for OracleSeq<T> {
    fn attribute(&self, ty: &Ty, attr: &str) -> Option<Result<Ty, ()>> {
        self.0.iter().find_map(|oracle| oracle.attribute(ty, attr))
    }

    fn builtin(&self, name: &str) -> Option<Result<Ty, ()>> {
        self.0.iter().find_map(|oracle| oracle.builtin(name))
    }

    fn builtin_call(&self, name: &str, args: &[Arg]) -> Option<Result<Ty, String>> {
        self.0
            .iter()
            .find_map(|oracle| oracle.builtin_call(name, args))
    }

    fn subtype(&self, require: &TyName, got: &TyName) -> bool {
        self.0.iter().any(|oracle| oracle.subtype(require, got))
    }
}

// Forwarding traits

impl<'a, T: TypingOracle + ?Sized> TypingOracle for &'a T {
    fn attribute(&self, ty: &Ty, attr: &str) -> Option<Result<Ty, ()>> {
        (*self).attribute(ty, attr)
    }
    fn builtin(&self, name: &str) -> Option<Result<Ty, ()>> {
        (*self).builtin(name)
    }
    fn builtin_call(&self, name: &str, args: &[Arg]) -> Option<Result<Ty, String>> {
        (*self).builtin_call(name, args)
    }
    fn subtype(&self, require: &TyName, got: &TyName) -> bool {
        (*self).subtype(require, got)
    }
}

impl<T: TypingOracle + ?Sized> TypingOracle for Box<T> {
    fn attribute(&self, ty: &Ty, attr: &str) -> Option<Result<Ty, ()>> {
        self.as_ref().attribute(ty, attr)
    }
    fn builtin(&self, name: &str) -> Option<Result<Ty, ()>> {
        self.as_ref().builtin(name)
    }
    fn builtin_call(&self, name: &str, args: &[Arg]) -> Option<Result<Ty, String>> {
        self.as_ref().builtin_call(name, args)
    }
    fn subtype(&self, require: &TyName, got: &TyName) -> bool {
        self.as_ref().subtype(require, got)
    }
}
