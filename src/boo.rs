// SPDX-FileCopyrightText: 2024 Nils Jochem
// SPDX-License-Identifier: MPL-2.0

#![cfg(feature = "boo")]
//! A module for Boo (Borrow or Owned)
use std::{
    borrow::Borrow,
    ops::{Deref, DerefMut},
};

/// A Borrow or Owned Smart pointer
/// The Boo type is a smaller Variant of Cow, which doesn't need special types for Borrowed and Owned Types
pub enum Boo<'b, T> {
    /// Borrowed data
    Borrowed(&'b T),
    /// Owned data
    Owned(T),
}

// into Boo
impl<'b, T> TryFrom<Moo<'b, T>> for Boo<'b, T> {
    type Error = &'b mut T;
    fn try_from(val: Moo<'b, T>) -> Result<Self, Self::Error> {
        Boo::try_from(Mob::from(val))
    }
}
impl<'b, T> TryFrom<Mob<'b, T>> for Boo<'b, T> {
    type Error = &'b mut T;

    fn try_from(value: Mob<'b, T>) -> Result<Self, Self::Error> {
        match value {
            Mob::BorrowedMut(t) => Err(t),
            Mob::Borrowed(t) => Ok(Self::Borrowed(t)),
            Mob::Owned(t) => Ok(Self::Owned(t)),
        }
    }
}

impl<'b, T> AsRef<T> for Boo<'b, T> {
    fn as_ref(&self) -> &T {
        match self {
            Boo::Borrowed(t) => t,
            Boo::Owned(t) => t,
        }
    }
}
impl<'b, T> Borrow<T> for Boo<'b, T> {
    fn borrow(&self) -> &T {
        match self {
            Boo::Borrowed(t) => t,
            Boo::Owned(t) => t,
        }
    }
}
impl<'b, T> Deref for Boo<'b, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        match self {
            Boo::Borrowed(t) => t,
            Boo::Owned(t) => t,
        }
    }
}

impl<'b, T> Boo<'b, T> {
    /// creates a new instance while coercing mutable references to normal refs
    pub fn from_coerce_ref(value: impl TryInto<Self, Error = &'b mut T>) -> Self {
        value.try_into().unwrap_or_else(|err| Self::Borrowed(err))
    }
    /// gives an owned instance of `T` by using `deref` on the held reference
    pub fn into_owned(self, deref: impl FnOnce(&'b T) -> T) -> T {
        match self {
            Self::Owned(t) => t,
            Self::Borrowed(t) => deref(t),
        }
    }
    /// gives an owned instance of `T` by cloning the held reference
    pub fn cloned(self) -> T
    where
        T: Clone,
    {
        self.into_owned(T::clone)
    }
    /// gives an owned instance of `T` by cloning the held reference
    pub fn copied(self) -> T
    where
        T: Copy,
    {
        self.into_owned(|it| *it)
    }
}

/// A Mutable, Owned or Borrowed Smart Pointer
/// usefull for implementing Mathoperations while capturing all possible combinations of ownership
#[derive(Debug, PartialEq, Eq, derive_more::From)]
pub enum Mob<'b, T> {
    /// Owned data
    Owned(T),
    /// Borrowed data
    Borrowed(&'b T),
    /// Mutalble borrowed data
    BorrowedMut(&'b mut T),
}

// Into Mob
impl<'b, T> From<Moo<'b, T>> for Mob<'b, T> {
    fn from(value: Moo<'b, T>) -> Self {
        match value {
            Moo::Owned(owned) => Mob::Owned(owned),
            Moo::BorrowedMut(borrow) => Mob::BorrowedMut(borrow),
        }
    }
}
impl<'b, T> From<Boo<'b, T>> for Mob<'b, T> {
    fn from(value: Boo<'b, T>) -> Self {
        match value {
            Boo::Owned(owned) => Mob::Owned(owned),
            Boo::Borrowed(borrow) => Mob::Borrowed(borrow),
        }
    }
}

// From Mob
impl<'b, T> From<Mob<'b, T>> for Option<&'b mut T> {
    fn from(val: Mob<'b, T>) -> Self {
        val.try_into_mut()
    }
}

impl<'b, T> AsRef<T> for Mob<'b, T> {
    fn as_ref(&self) -> &T {
        match self {
            Mob::Owned(t) => t,
            Mob::Borrowed(t) => t,
            Mob::BorrowedMut(t) => t,
        }
    }
}
impl<'b, T> Borrow<T> for Mob<'b, T> {
    fn borrow(&self) -> &T {
        match self {
            Mob::Owned(t) => t,
            Mob::Borrowed(t) => t,
            Mob::BorrowedMut(t) => t,
        }
    }
}
impl<'b, T> Deref for Mob<'b, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Mob::Owned(t) => t,
            Mob::Borrowed(t) => t,
            Mob::BorrowedMut(t) => t,
        }
    }
}

impl<'b, T> Mob<'b, T> {
    /// tries to give a mut ref to the held data where possible (owned, borrow mut) and None else (borrowed)
    pub fn try_as_mut(&mut self) -> Option<&mut T> {
        match self {
            Self::Owned(t) => Some(t),
            Self::BorrowedMut(t) => Some(t),
            Self::Borrowed(_) => None,
        }
    }
    /// returns the mutably borrowed value when existing
    pub fn try_into_mut(self) -> Option<&'b mut T> {
        match self {
            Self::BorrowedMut(t) => Some(t),
            Self::Borrowed(_) | Mob::Owned(_) => None,
        }
    }

    /// gives an owned instance of `T` by using `deref` on the held reference
    pub fn into_owned(self, deref: impl FnOnce(&'b T) -> T) -> T {
        match self {
            Self::Owned(t) => t,
            Self::Borrowed(t) => deref(t),
            Self::BorrowedMut(t) => deref(t),
        }
    }
    /// gives an owned instance of `T` by cloning the held reference
    pub fn cloned(self) -> T
    where
        T: Clone,
    {
        self.into_owned(T::clone)
    }
    /// gives an owned instance of `T` by cloning the held reference
    pub fn copied(self) -> T
    where
        T: Copy,
    {
        self.into_owned(|it| *it)
    }

    /// returns the held value and returning the mut ref when existing
    /// the mutable reference will be left with the `T::default()`
    pub fn take_keep_ref(self) -> (T, Option<&'b mut T>)
    where
        T: Default + Clone,
    {
        match self {
            Self::BorrowedMut(mut_ref) => (std::mem::take(mut_ref), Some(mut_ref)),
            _ => (self.cloned(), None),
        }
    }
}

/// A Mutable or Owned Smart Pointer
/// usefull for return values of functions that take `Mob`
///
/// this type implements derefmut
#[derive(Debug, PartialEq, Eq, derive_more::From)]
pub enum Moo<'b, T> {
    /// Owned data
    Owned(T),
    /// Mutable borrowed data
    BorrowedMut(&'b mut T),
}

impl<'b, T> TryFrom<Boo<'b, T>> for Moo<'b, T> {
    type Error = &'b T;
    fn try_from(val: Boo<'b, T>) -> Result<Self, Self::Error> {
        Moo::try_from(Mob::from(val))
    }
}
impl<'b, T> TryFrom<Mob<'b, T>> for Moo<'b, T> {
    type Error = &'b T;

    fn try_from(value: Mob<'b, T>) -> Result<Self, Self::Error> {
        match value {
            Mob::BorrowedMut(t) => Ok(Self::BorrowedMut(t)),
            Mob::Borrowed(t) => Err(t),
            Mob::Owned(t) => Ok(Self::Owned(t)),
        }
    }
}

impl<'b, T> From<Moo<'b, T>> for Option<&'b mut T> {
    fn from(val: Moo<'b, T>) -> Self {
        val.try_into_mut()
    }
}

impl<'b, T> AsRef<T> for Moo<'b, T> {
    fn as_ref(&self) -> &T {
        match self {
            Moo::Owned(it) => it,
            Moo::BorrowedMut(it) => it,
        }
    }
}
impl<'b, T> Borrow<T> for Moo<'b, T> {
    fn borrow(&self) -> &T {
        match self {
            Moo::Owned(it) => it,
            Moo::BorrowedMut(it) => it,
        }
    }
}
impl<'b, T> Deref for Moo<'b, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Moo::Owned(it) => it,
            Moo::BorrowedMut(it) => it,
        }
    }
}
impl<'b, T> DerefMut for Moo<'b, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Moo::Owned(it) => it,
            Moo::BorrowedMut(it) => it,
        }
    }
}

impl<'b, T> Moo<'b, T> {
    /// creates a new instance either at `maybe_ref` or owned, but always with `value`
    pub fn from_with_value(maybe_ref: impl Into<Option<&'b mut T>>, value: T) -> Self {
        match maybe_ref.into() {
            Some(mut_ref) => {
                *mut_ref = value;
                Self::BorrowedMut(mut_ref)
            }
            None => Self::Owned(value),
        }
    }
    /// creates a new insatance from `value` with the potetionally borrowed value cloned
    pub fn from_mob_cloned(value: Mob<'b, T>) -> Self
    where
        T: Clone,
    {
        match value {
            Mob::BorrowedMut(value) => Moo::BorrowedMut(value),
            value => Moo::Owned(value.cloned()),
        }
    }

    /// expects `self` to be owned or panic with `msg`
    ///
    /// # Panics
    /// will panic when `self` is borrowed
    pub fn expect_owned(self, msg: impl AsRef<str>) -> T {
        #[allow(clippy::expect_fun_call)]
        self.try_get_owned().expect(msg.as_ref())
    }
    /// returns the owned value when possible
    pub fn try_get_owned(self) -> Option<T> {
        match self {
            Self::Owned(it) => Some(it),
            Self::BorrowedMut(_) => None,
        }
    }
    /// expects `self` to be a mutable borrow or panic with `msg`
    ///
    /// # Panics
    /// will panic when `self` is owned
    pub fn expect_mut(self, msg: impl AsRef<str>) -> &'b mut T {
        #[allow(clippy::expect_fun_call)]
        self.try_into_mut().expect(msg.as_ref())
    }
    /// returns the borrowed value when possible
    pub fn try_into_mut(self) -> Option<&'b mut T> {
        Option::from(Mob::from(self))
    }

    /// gives an owned instance of `T` by using `deref` on the held reference
    pub fn into_owned(self, deref: impl FnOnce(&'b T) -> T) -> T {
        match self {
            Self::Owned(t) => t,
            Self::BorrowedMut(t) => deref(t),
        }
    }
    /// gives an owned instance of `T` by cloning the held reference
    pub fn cloned(self) -> T
    where
        T: Clone,
    {
        self.into_owned(T::clone)
    }
    /// gives an owned instance of `T` by cloning the held reference
    pub fn copied(self) -> T
    where
        T: Copy,
    {
        self.into_owned(|it| *it)
    }
}
