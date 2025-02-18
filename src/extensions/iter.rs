// SPDX-FileCopyrightText: 2024 Nils Jochem
// SPDX-License-Identifier: MPL-2.0

/// extentions for all Iterators
pub trait IteratorExt: Iterator + Sized {
    /// creates an [`ExactSizeIterator`] from `self` with `size`
    fn with_size(self, size: usize) -> ExactSizeWrapper<Self>;
    /// zips `other` to the left og `self`
    fn lzip<I: Iterator>(self, other: I) -> std::iter::Zip<I, Self>;
    /// checks if `self` is ordered
    #[allow(clippy::wrong_self_convention)]
    fn is_sorted(self) -> bool
    where
        Self::Item: Ord;
    /// checks if `self` is ordered by `ord`
    #[allow(clippy::wrong_self_convention)]
    fn is_sorted_by(self, ord: impl FnMut(&Self::Item, &Self::Item) -> std::cmp::Ordering) -> bool;

    /// collects element one by one into an accumulator, but returns instantly when `transform` returns an error
    /// # Errors
    /// returns any error that `transform` returns
    fn reduce_early_return<ACC, E>(
        self,
        initial: ACC,
        transform: impl FnMut(ACC, Self::Item) -> Result<ACC, E>,
    ) -> Result<ACC, E>;
}
impl<Iter: Iterator> IteratorExt for Iter {
    fn with_size(self, size: usize) -> ExactSizeWrapper<Self> {
        ExactSizeWrapper::new(self, size)
    }
    #[inline]
    fn lzip<I: Iterator>(self, other: I) -> std::iter::Zip<I, Self> {
        other.zip(self)
    }
    fn is_sorted(self) -> bool
    where
        Self::Item: Ord,
    {
        IteratorExt::is_sorted_by(self, Self::Item::cmp)
    }
    fn is_sorted_by(
        self,
        mut ord: impl FnMut(&Self::Item, &Self::Item) -> std::cmp::Ordering,
    ) -> bool {
        let mut last = None;
        for item in self {
            if last.is_some_and(|last| ord(&last, &item).is_gt()) {
                return false;
            }
            last = Some(item);
        }
        true
    }
    fn reduce_early_return<ACC, E>(
        self,
        initial: ACC,
        mut reduce: impl FnMut(ACC, Self::Item) -> Result<ACC, E>,
    ) -> Result<ACC, E> {
        let mut acc = initial;
        for next in self {
            acc = match reduce(acc, next) {
                Result::Ok(value) => value,
                Result::Err(e) => return Err(e),
            }
        }
        Ok(acc)
    }
}

/// extentions for all Iterators over [futures](core::future::Future)
#[cfg(feature = "fut_iter")]
pub trait FutIterExt: IntoIterator + Sized
where
    Self::Item: core::future::Future,
{
    /// joins all futures in `self`
    fn join_all(self) -> futures::future::JoinAll<<Self as IntoIterator>::Item>;
}
#[cfg(feature = "fut_iter")]
impl<Iter: IntoIterator + Sized> FutIterExt for Iter
where
    Iter::Item: core::future::Future,
{
    fn join_all(self) -> futures::future::JoinAll<<Self as IntoIterator>::Item> {
        futures::future::join_all(self)
    }
}

/// extentions for all Iterators over clonable Elements
pub trait CloneIteratorExt: Iterator + Sized {
    /// cuts up the iterator in chunks of size `window_size`. The next Chunk starts `hop_lenght` after the last one started
    fn chunked(self, window_size: usize, hop_length: usize) -> ChunkedIterator<Self>;
    /// filters elements with respect to thier neighbors
    fn filter_surrounding<F>(self, predicate: F) -> SurroundingFilterIterator<Self, F>
    where
        F: FnMut(&Option<Self::Item>, &Self::Item, &Option<Self::Item>) -> bool;
    /// iterates over all pairs of Elements, with special cases for first and last
    fn open_border_pairs(self) -> OpenBorderWindowIterator<Self>;
}
impl<Iter> CloneIteratorExt for Iter
where
    Iter: Iterator,
    Iter::Item: Clone,
{
    fn chunked(self, window_size: usize, hop_length: usize) -> ChunkedIterator<Self> {
        ChunkedIterator::new(self, window_size, hop_length)
    }
    fn filter_surrounding<F>(self, predicate: F) -> SurroundingFilterIterator<Self, F>
    where
        F: FnMut(&Option<Self::Item>, &Self::Item, &Option<Self::Item>) -> bool,
    {
        SurroundingFilterIterator::new(self, predicate)
    }
    fn open_border_pairs(self) -> OpenBorderWindowIterator<Self> {
        OpenBorderWindowIterator::new(self)
    }
}
#[allow(missing_docs)]
pub struct ChunkedIterator<Iter: Iterator> {
    iter: Iter,
    window_size: usize,
    hop_length: usize,
    buffer: Vec<Iter::Item>,
}
impl<Iter> ChunkedIterator<Iter>
where
    Iter: Iterator,
    Iter::Item: Clone,
{
    fn new(iter: Iter, window_size: usize, hop_length: usize) -> Self {
        Self {
            iter,
            window_size,
            hop_length,
            buffer: Vec::with_capacity(hop_length),
        }
    }
}
impl<Iter> Iterator for ChunkedIterator<Iter>
where
    Iter: Iterator,
    Iter::Item: Clone,
{
    type Item = Vec<Iter::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.buffer.len() < self.window_size {
            match self.iter.next() {
                Some(e) => self.buffer.push(e),
                None => break,
            }
        }
        if self.buffer.is_empty() {
            return None;
        }
        let ret = self.buffer.clone();
        self.buffer.drain(..self.hop_length.min(self.buffer.len()));

        Some(ret)
    }
}
impl<Iter> ExactSizeIterator for ChunkedIterator<Iter>
where
    Iter: ExactSizeIterator,
    Iter::Item: Clone,
{
    fn len(&self) -> usize {
        (self.iter.len() as f64 / self.hop_length as f64).ceil() as usize
    }
}

#[allow(missing_docs)]
pub struct SurroundingFilterIterator<
    Iter: Iterator,
    F: FnMut(&Option<Iter::Item>, &Iter::Item, &Option<Iter::Item>) -> bool,
> {
    iter: Iter,
    predicate: F,
    last: Option<Iter::Item>,
    element: Option<Iter::Item>,
    next: Option<Iter::Item>,
}
impl<Iter, F> SurroundingFilterIterator<Iter, F>
where
    Iter: Iterator,
    Iter::Item: Clone,
    F: FnMut(&Option<Iter::Item>, &Iter::Item, &Option<Iter::Item>) -> bool,
{
    fn new(mut iter: Iter, predicate: F) -> Self {
        Self {
            predicate,
            last: None,
            element: iter.next(),
            next: iter.next(),
            iter,
        }
    }
}
impl<Iter, F> Iterator for SurroundingFilterIterator<Iter, F>
where
    Iter: Iterator,
    Iter::Item: Clone,
    F: FnMut(&Option<Iter::Item>, &Iter::Item, &Option<Iter::Item>) -> bool,
{
    type Item = Iter::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let include = (self.predicate)(&self.last, self.element.as_ref()?, &self.next);
        let element = std::mem::replace(&mut self.next, self.iter.next()); // get next element
        self.last = std::mem::replace(&mut self.element, element);
        if include {
            Some(self.last.clone().unwrap()) // return clone of self.last==old.element
        } else {
            self.next() // skip this element
        }
    }
}

#[allow(missing_docs)]
pub struct ExactSizeWrapper<Iter: Iterator> {
    iter: Iter,
    consumed: usize,
    size: usize,
}
impl<Iter: Iterator> ExactSizeWrapper<Iter> {
    const fn new(iter: Iter, size: usize) -> Self {
        Self {
            iter,
            consumed: 0,
            size,
        }
    }
}
impl<Iter: Iterator> Iterator for ExactSizeWrapper<Iter> {
    type Item = Iter::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let ret = self.iter.next();
        self.consumed += ret.is_some() as usize;
        ret
    }
}
impl<Iter: Iterator> ExactSizeIterator for ExactSizeWrapper<Iter> {
    fn len(&self) -> usize {
        self.size - self.consumed
    }
}
impl<Iter: Iterator + DoubleEndedIterator> DoubleEndedIterator for ExactSizeWrapper<Iter> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let ret = self.iter.next_back();
        self.consumed += ret.is_some() as usize;
        ret
    }
}

/// represents a Pair of items, or the border elements
#[derive(Debug, PartialEq, Eq)]
pub enum State<T> {
    /// the first element
    Start(T),
    /// a Pair of elements
    Middle(T, T),
    /// the last element
    End(T),
}
impl<T> State<T> {
    #[allow(clippy::missing_const_for_fn)]
    fn new(a: Option<T>, b: Option<T>) -> Option<Self> {
        match (a, b) {
            (None, None) => None,
            (None, Some(b)) => Some(Self::Start(b)),
            (Some(a), Some(b)) => Some(Self::Middle(a, b)),
            (Some(a), None) => Some(Self::End(a)),
        }
    }
}
#[allow(missing_docs)]
pub struct OpenBorderWindowIterator<Iter: Iterator> {
    iter: Iter,
    next: Option<Iter::Item>,
}
impl<Iter> OpenBorderWindowIterator<Iter>
where
    Iter: Iterator,
    Iter::Item: Clone,
{
    const fn new(iter: Iter) -> Self {
        Self { iter, next: None }
    }
}
impl<Iter> Iterator for OpenBorderWindowIterator<Iter>
where
    Iter: Iterator,
    Iter::Item: Clone,
{
    type Item = State<Iter::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        let last = std::mem::replace(&mut self.next, self.iter.next());
        State::new(last, self.next.clone())
    }
}
impl<Iter> ExactSizeIterator for OpenBorderWindowIterator<Iter>
where
    Iter: ExactSizeIterator,
    Iter::Item: Clone,
{
    fn len(&self) -> usize {
        self.iter.len() + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;

    #[test]
    fn chunked_test() {
        let expected = vec![0..6, 4..10, 8..14, 12..15]
            .into_iter()
            .map(itertools::Itertools::collect_vec)
            .collect_vec();
        let is = (0..15).chunked(6, 4);
        assert_eq!(expected.len(), is.len());

        let is = is.collect_vec();
        assert!(&is.eq(&expected), "expected {expected:?} but was {is:?}");
    }

    #[test]
    fn surrounding_filter_test() {
        let is = (0..4)
            .filter_surrounding(|l, _e, a| {
                !(l.is_some_and(|it| it == 2) || a.is_some_and(|it| it == 2))
            })
            .collect_vec();
        let expected = vec![0, 2];
        assert!(&is.eq(&expected), "expected {expected:?} but got {is:?}");
    }
    #[test]
    fn open_border_iter() {
        let iter = [1, 2, 3].into_iter().open_border_pairs();
        assert_eq!(iter.len(), 4);
        assert!(iter.eq([
            State::Start(1),
            State::Middle(1, 2),
            State::Middle(2, 3),
            State::End(3)
        ]
        .into_iter()));
    }

    #[test]
    fn exact_size() {
        let mut iter = (0..10).with_size(10);

        assert_eq!(iter.len(), 10);

        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.len(), 9);
        assert_eq!(iter.next_back(), Some(9));
        assert_eq!(iter.len(), 8);

        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.len(), 7);
        assert_eq!(iter.next_back(), Some(8));
        assert_eq!(iter.len(), 6);

        assert_eq!(iter.collect_vec(), (2..8).collect_vec());
    }

    #[test]
    fn reduce_early_return() {
        assert_eq!(
            vec![1, 2, 3, 0, 4, 5]
                .into_iter()
                .inspect(|it| assert!(*it < 4, "didn't early return"))
                .reduce_early_return(1, |acc, it| {
                    if it != 0 {
                        Ok(acc * it)
                    } else {
                        Err(0)
                    }
                }),
            Err(0)
        );
        assert_eq!(
            vec![1, 2, 3, 4, 5]
                .into_iter()
                .reduce_early_return(1, |acc, it| {
                    if it != 0 {
                        Ok(acc * it)
                    } else {
                        Err(0)
                    }
                }),
            Ok(120)
        );
    }
}
