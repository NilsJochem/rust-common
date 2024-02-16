// SPDX-FileCopyrightText: 2024 Nils Jochem
// SPDX-License-Identifier: MPL-2.0

//! some common functionalitys
pub mod boo;
/// a collection for extionsion functions
pub mod extensions {
    ///extention functions for [`std::borrow::Cow`]
    pub mod cow;
    ///extention functions for [`std::time::Duration`]
    pub mod duration;
    /// extention function for Iterators
    pub mod iter;
    ///extention functions for [`Option`]
    pub mod option;
    ///extention functions for [`Vec`]
    pub mod vec;
}
pub mod io;
pub mod rc;
/// common string utils
pub mod str {
    /// A module for converting the case of strings
    pub mod convert;
    /// A module for searching strings
    pub mod filter;

    #[allow(missing_docs)]
    pub const fn compare_char(a: char, b: char, ignore_case: bool) -> bool {
        (ignore_case && a.eq_ignore_ascii_case(&b)) || a == b
    }
}

/// collections
pub mod collections {
    /// a wrapper to packed bits
    pub mod bit_set;

    enum DoubleArrayIndex {
        First(usize),
        Second(usize),
    }
    /// Array of size N + M
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(C)]
    pub struct ArrayNPM<const N: usize, const M: usize, T> {
        a: [T; N],
        b: [T; M],
    }
    impl<const N: usize, const M: usize, T> ArrayNPM<N, M, T> {
        /// populates this array with values produced by `cb`
        pub fn from_fn(mut cb: impl FnMut(usize) -> T) -> Self {
            Self {
                a: std::array::from_fn(&mut cb),
                b: std::array::from_fn(|it| cb(it + N)),
            }
        }

        fn to_index(idx: usize) -> DoubleArrayIndex {
            if idx < N {
                DoubleArrayIndex::First(idx)
            } else if idx < N + M {
                DoubleArrayIndex::Second(idx - N)
            } else {
                panic!("{idx} is out of bounds {}", N + M)
            }
        }
        fn assert_slice() {
            let size_t = std::mem::size_of::<T>();
            assert_ne!(0, size_t, "can't work with zero sized Types");

            assert_ne!(
                None,
                N.checked_add(M)
                    .and_then(|len| len.checked_mul(size_t))
                    .and_then(|len| isize::try_from(len).ok()),
                "length would overflow pointer"
            );
        }
        /// returns a slice representing `self`
        pub fn as_slice(&mut self) -> &[T] {
            Self::assert_slice();
            // SAFETY: assert_slice checks for Zero Size of T and overflows of (N+M)*size_t
            unsafe { std::slice::from_raw_parts(self.a.as_ptr(), N + M) }
        }
        /// returns a mutable slice representing `self`
        pub fn as_mut_slice(&mut self) -> &mut [T] {
            Self::assert_slice();
            // SAFETY: assert_slice checks for Zero Size of T and overflows of (N+M)*size_t
            unsafe { std::slice::from_raw_parts_mut(self.a.as_mut_ptr(), N + M) }
        }
    }

    impl<const N: usize, const M: usize, T> std::ops::Index<usize> for ArrayNPM<N, M, T> {
        type Output = T;

        fn index(&self, index: usize) -> &Self::Output {
            match Self::to_index(index) {
                DoubleArrayIndex::First(idx) => &self.a[idx],
                DoubleArrayIndex::Second(idx) => &self.b[idx],
            }
        }
    }
    impl<const N: usize, const M: usize, T> std::ops::IndexMut<usize> for ArrayNPM<N, M, T> {
        fn index_mut(&mut self, index: usize) -> &mut Self::Output {
            match Self::to_index(index) {
                DoubleArrayIndex::First(idx) => &mut self.a[idx],
                DoubleArrayIndex::Second(idx) => &mut self.b[idx],
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::extensions::iter::IteratorExt;

        use super::*;

        #[test]
        fn slice() {
            let mut data = ArrayNPM::<3, 3, u8>::from_fn(|it| it as u8);

            for (i, ele) in data.as_mut_slice().iter_mut().lzip(0..) {
                assert_eq!(i, *ele);
                *ele += 10;
            }
            assert_eq!([10, 11, 12], data.a, "failed to write to a");
            assert_eq!([13, 14, 15], data.b, "failed to write to b");
        }
    }
}

/// used as drop in replacement for assert, when an Error needs to be returned
/// the Error will be lazily constructed
///
/// #[usage]
/// require!{
///     <condition>,
///     <Error>
/// }
#[macro_export]
macro_rules! require {
    ($cond:expr, $err:expr) => {
        if !$cond {
            return Err($err);
        }
    };
}

/// common utilitys for argparsing
pub mod args {
    /// common utilitys for input managing
    pub mod input {
        use clap::Args;

        #[derive(Args, Debug, Clone, Copy)]
        #[group(required = false, multiple = false)]
        /// clap struct for input config
        pub struct Inputs {
            /// always answer yes
            #[clap(short)]
            pub yes: bool,
            /// always answer no
            #[clap(short)]
            pub no: bool,
            /// number of retrys
            #[clap(long, default_value_t = 3)]
            pub trys: u8,
        }
        impl Inputs {
            /// creates a new Inputs struct
            ///
            /// when bools is None, no default answer is set
            pub fn new(bools: impl Into<Option<bool>>, trys: impl Into<Option<u8>>) -> Self {
                let bools: Option<_> = bools.into();
                Self {
                    yes: bools.is_some(),
                    no: bools.is_some_and(|it| !it),
                    trys: trys.into().unwrap_or(3),
                }
            }

            #[inline]
            #[allow(clippy::needless_pass_by_value)]
            fn inner_read<T>(
                msg: impl AsRef<str>,
                default: impl Into<Option<T>>,
                retry_msg: Option<impl AsRef<str>>,
                mut map: impl FnMut(String) -> Option<T>,
                trys: impl IntoIterator<Item = u8>,
            ) -> Option<T> {
                let msg = msg.as_ref();
                let retry_msg = retry_msg.as_ref().map(std::convert::AsRef::as_ref);
                let default = default.into();

                print!("{msg}");
                for _ in trys {
                    let rin: String = text_io::read!("{}\n");
                    if default.is_some() && rin.is_empty() {
                        return default;
                    }
                    match (map(rin), retry_msg) {
                        (Some(t), _) => return Some(t),
                        (None, Some(retry_msg)) => println!("{retry_msg}"),
                        (None, None) => print!("{msg}"),
                    }
                }
                None
            }

            const DEFAULT_RETRY_MSG: &'static str = "couldn't parse that, please try again: ";
            /// read userinput as a String
            pub fn read(msg: impl AsRef<str>, default: Option<String>) -> String {
                Self::inner_read(
                    msg,
                    default,
                    Some(Self::DEFAULT_RETRY_MSG),
                    Some,
                    std::iter::once(1),
                )
                .unwrap_or_else(|| unreachable!())
            }
            /// read userinput and map it. Retrys to read until `map` returns `Some`
            pub fn map_read<T>(
                msg: impl AsRef<str>,
                default: impl Into<Option<T>>,
                retry_msg: Option<impl AsRef<str>>,
                map: impl FnMut(String) -> Option<T>,
            ) -> T {
                Self::inner_read(msg, default, retry_msg, map, 1..)
                    .unwrap_or_else(|| unreachable!())
            }
            // TODO remove trys from Self
            /// read userinput and map it. Retrys to read until `map` returns `Some` or until self.trys
            pub fn try_read<T>(
                &self,
                msg: impl AsRef<str>,
                default: Option<T>,
                map: impl FnMut(String) -> Option<T>,
            ) -> Option<T> {
                Self::inner_read(
                    msg,
                    default,
                    Some(Self::DEFAULT_RETRY_MSG),
                    map,
                    1..self.trys,
                )
            }

            #[must_use]
            #[momo::momo]
            /// asks user for consent if no default is set
            pub fn ask_consent(self, msg: impl AsRef<str>) -> bool {
                if self.yes || self.no {
                    return self.yes;
                }
                self.try_read(format!("{msg} [y/n]: "), None, |it| {
                    if ["y", "yes", "j", "ja"].contains(&it.as_str()) {
                        Some(true)
                    } else if ["n", "no", "nein"].contains(&it.as_str()) {
                        Some(false)
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| {
                    log::info!("probably not");
                    false
                })
            }

            #[must_use]
            /// read userinput as a String.
            /// Starts with `initial`
            /// Uses `suggestor` for suggestions
            ///
            /// # Panics
            /// unwraps undocumented Result of [`inquire::prompts::text::Text::prompt`]
            pub fn read_with_suggestion(
                msg: impl AsRef<str>,
                initial: Option<&str>,
                mut suggestor: impl autocompleter::Autocomplete,
            ) -> String {
                let mut text = inquire::Text::new(msg.as_ref());
                text.initial_value = initial;
                // SAFTY: the reference to suggestor must be kept alive until ac is dropped. black-box should do this.
                let ac = unsafe { autocompleter::BorrowCompleter::new(&mut suggestor) };
                let res = text.with_autocomplete(ac).prompt().unwrap();
                drop(std::hint::black_box(suggestor));
                res
            }
        }

        #[allow(missing_docs)]
        pub mod autocompleter {
            use std::fmt::Debug;

            use itertools::Itertools;

            use crate::str::filter::StrMetric;

            pub type Error = inquire::CustomUserError;
            pub type Replacement = inquire::autocompletion::Replacement;
            /// a wrapper around inquires Autocomplete, that doesn't have the Clone + 'static requirement
            pub trait Autocomplete: Debug {
                /// List of input suggestions to be displayed to the user upon typing the
                /// text input.
                ///
                /// If the user presses the autocompletion hotkey (`tab` as default) with
                /// a suggestion highlighted, the user's text input will be replaced by the
                /// content of the suggestion string.
                #[allow(clippy::missing_errors_doc)]
                fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, Error>;

                /// Standalone autocompletion that can be implemented based solely on the user's
                /// input.
                ///
                /// If the user presses the autocompletion hotkey (`tab` as default) and
                /// there are no suggestions highlighted (1), this function will be called in an
                /// attempt to autocomplete the user's input.
                ///
                /// If the returned value is of the `Some` variant, the text input will be replaced
                /// by the content of the string.
                ///
                /// (1) This applies where either there are no suggestions at all, or there are
                /// some displayed but the user hasn't highlighted any.
                #[allow(clippy::missing_errors_doc)]
                fn get_completion(
                    &mut self,
                    input: &str,
                    highlighted_suggestion: Option<String>,
                ) -> Result<Replacement, Error>;
            }
            impl<AC: Autocomplete> Autocomplete for &mut AC {
                #[inline]
                fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, Error> {
                    (**self).get_suggestions(input)
                }

                #[inline]
                fn get_completion(
                    &mut self,
                    input: &str,
                    highlighted_suggestion: Option<String>,
                ) -> Result<Replacement, Error> {
                    (**self).get_completion(input, highlighted_suggestion)
                }
            }
            #[derive(Debug)]
            /// adapter to match `MyAutocompleter` to `inqure::Autocomplete`
            pub(super) struct BorrowCompleter {
                inner: &'static mut dyn Autocomplete,
            }
            impl BorrowCompleter {
                pub(super) unsafe fn new<'a>(other: &'a mut dyn Autocomplete) -> Self {
                    // SAFTY: transmute to upgrade lifetime to static, so one can uphold Autocompletes Clone + 'static needs
                    Self {
                        inner: unsafe {
                            std::mem::transmute::<
                                &'a mut dyn Autocomplete,
                                &'static mut dyn Autocomplete,
                            >(other)
                        },
                    }
                }
            }
            // fake being clone, it's (probably) only needed, when the holding inquire::Text ist cloned
            impl Clone for BorrowCompleter {
                fn clone(&self) -> Self {
                    panic!("cloned Autocompleter {self:?}");
                    // Self { inner: self.inner }
                }
            }
            impl inquire::Autocomplete for BorrowCompleter {
                fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, Error> {
                    self.inner.get_suggestions(input)
                }

                fn get_completion(
                    &mut self,
                    input: &str,
                    highlighted_suggestion: Option<String>,
                ) -> Result<Replacement, Error> {
                    self.inner.get_completion(input, highlighted_suggestion)
                }
            }

            #[derive(Debug)]
            /// takes a list of Strings for suggestions
            pub struct VecCompleter {
                data: Vec<String>,
                metric: Box<dyn StrMetric + Send>,
            }
            impl VecCompleter {
                #[must_use]
                #[allow(missing_docs)]
                pub fn new(data: Vec<String>, metric: impl StrMetric + Send + 'static) -> Self {
                    Self {
                        data,
                        metric: Box::new(metric),
                    }
                }
                #[allow(missing_docs)]
                pub fn from_iter<Iter>(iter: Iter, metric: impl StrMetric + Send + 'static) -> Self
                where
                    Iter: IntoIterator,
                    Iter::Item: ToString,
                {
                    Self::new(
                        iter.into_iter().map(|it| it.to_string()).collect_vec(),
                        metric,
                    )
                }
            }
            impl Autocomplete for VecCompleter {
                fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, Error> {
                    Ok(crate::str::filter::sort_with(
                        self.metric.as_ref(),
                        self.data.iter(),
                        input,
                        |it| it,
                    )
                    .cloned()
                    .collect_vec())
                }

                fn get_completion(
                    &mut self,
                    _input: &str,
                    highlighted_suggestion: Option<String>,
                ) -> Result<Replacement, Error> {
                    Ok(highlighted_suggestion)
                }
            }
        }
    }

    /// common debug utils
    pub mod debug {
        use clap::Args;

        #[derive(Args, Debug, Clone, Copy)]
        #[group(required = false, multiple = false)]
        #[allow(clippy::struct_excessive_bools)]
        #[allow(missing_docs)]
        pub struct OutputLevel {
            #[clap(short, long, help = "print maximum info")]
            pub(crate) debug: bool,
            #[clap(short, long, help = "print more info")]
            pub(crate) verbose: bool,
            #[clap(short, long, help = "print sligtly more info")]
            pub(crate) warn: bool,
            #[clap(short, long, help = "print almost no info")]
            pub(crate) silent: bool,
        }

        impl OutputLevel {
            #[allow(missing_docs)]
            pub fn init_logger(&self) {
                let level = log::Level::from(*self);
                Self::init_logger_with(level);
            }
            #[allow(missing_docs)]
            pub fn init_logger_with(level: log::Level) {
                let env = env_logger::Env::default();
                let env = env.default_filter_or(level.as_str());

                let mut builder = env_logger::Builder::from_env(env);

                builder.format_timestamp(None);
                builder.format_target(false);
                builder.format_level(level < log::Level::Info);

                builder.init();
            }
        }

        impl From<OutputLevel> for log::Level {
            fn from(val: OutputLevel) -> Self {
                if val.silent {
                    Self::Error
                } else if val.verbose {
                    Self::Trace
                } else if val.debug {
                    Self::Debug
                } else if val.warn {
                    Self::Warn
                } else {
                    Self::Info
                }
            }
        }
    }
}
