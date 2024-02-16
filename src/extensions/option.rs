// SPDX-FileCopyrightText: 2024 Nils Jochem
// SPDX-License-Identifier: MPL-2.0

use std::future::Future;

/// extentions for Options<Type>
pub trait Ext {
    /// The type of value of the Option.
    type Type;

    /// Returns `true` if the option is a [`None`] or the value inside of it matches a predicate.
    #[allow(clippy::wrong_self_convention)]
    fn is_none_or(self, f: impl FnOnce(Self::Type) -> bool) -> bool;
}

impl<T> Ext for Option<T> {
    type Type = T;

    #[inline]
    fn is_none_or(self, f: impl FnOnce(T) -> bool) -> bool {
        match self {
            None => true,
            Some(x) => f(x),
        }
    }
}

#[test]
fn is_none_or() {
    assert!(Some(2).is_none_or(|x| x > 1));
    assert!(!Some(0).is_none_or(|x| x > 1));
    assert!(None::<usize>.is_none_or(|x| x > 1));
}

/// extentions for Options<Future<_>>
#[async_trait::async_trait]
pub trait FutureExt {
    /// The type of value produced on completion.
    type Type;
    /// returns the hold value or computes `f` and inserts it
    async fn get_or_try_insert_future<F: Future<Output = Option<Self::Type>> + Send>(
        &mut self,
        f: F,
    ) -> Option<&mut Self::Type>;
    #[allow(missing_docs)]
    async fn get_or_insert_future<F: Future<Output = Self::Type> + Send>(
        &mut self,
        f: F,
    ) -> &mut Self::Type {
        self.get_or_try_insert_future(async { Some(f.await) })
            .await
            .unwrap()
    }
    #[allow(missing_docs)]
    async fn insert_future_if_none<F: Future<Output = Self::Type> + Send>(&mut self, f: F) {
        let _ = self.get_or_insert_future(f).await;
    }
    #[allow(missing_docs)]
    async fn try_inser_futuret_if_none<F: Future<Output = Option<Self::Type>> + Send>(
        &mut self,
        f: F,
    ) {
        let _ = self.get_or_try_insert_future(f).await;
    }
}
#[async_trait::async_trait]
impl<T: Send> FutureExt for Option<T> {
    type Type = T;
    async fn get_or_try_insert_future<F: Future<Output = Self> + Send>(
        &mut self,
        f: F,
    ) -> Option<&mut Self::Type> {
        if self.is_none() {
            f.await.map(|t| self.insert(t))
        } else {
            self.as_mut()
        }
    }
}
