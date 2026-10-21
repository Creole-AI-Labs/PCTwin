//! An enum and the names of all its variants, declared together.
//!
//! A list of an enum's values kept by hand (an `ALL` constant, a test's walk, a fixture list) can
//! leave a new variant out, and an exhaustive `match` only forces an arm to exist, not a place in
//! the list. [`listed_enum!`] declares the enum and makes `VARIANTS` (every variant's name) and
//! `variant()` (a value's variant name) from the same declaration, so a check like "the examples
//! cover every variant" is complete by construction: a new variant is in `VARIANTS` the moment it
//! exists.

/// Declares an enum (attributes, derives and doc comments as usual; unit, tuple and struct
/// variants) and adds `VARIANTS` and `variant()`. The tests below show each kind of variant. (Not a
/// doc test: on Windows a doc test links against the app's build outputs when the whole workspace
/// is tested together, and fails to link.)
///
/// ```text
/// pctwin_types::listed_enum! {
///     #[derive(Debug, Clone, PartialEq)]
///     pub enum Shape {
///         /// A point.
///         Dot,
///         Line(u8),
///         Square { side: u8 },
///     }
/// }
/// assert_eq!(Shape::VARIANTS, ["Dot", "Line", "Square"]);
/// assert_eq!(Shape::Square { side: 2 }.variant(), "Square");
/// ```
#[macro_export]
macro_rules! listed_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $(
                $(#[$vmeta:meta])*
                $variant:ident $( ( $($tuple:tt)* ) )? $( { $($fields:tt)* } )?
            ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis enum $name {
            $(
                $(#[$vmeta])*
                $variant $( ( $($tuple)* ) )? $( { $($fields)* } )?
            ),+
        }

        impl $name {
            /// Every variant's name, in the order declared (made with the enum by `listed_enum!`,
            /// so it cannot leave one out).
            pub const VARIANTS: &'static [&'static str] = &[$(stringify!($variant)),+];

            /// This value's variant name, as it appears in `VARIANTS`.
            pub fn variant(&self) -> &'static str {
                match self {
                    $(
                        $crate::__listed_pattern!(
                            $name $variant $( ( $($tuple)* ) )? $( { $($fields)* } )?
                        ) => stringify!($variant),
                    )+
                }
            }
        }
    };
}

/// The pattern that matches any value of one variant (used by [`listed_enum!`]).
#[doc(hidden)]
#[macro_export]
macro_rules! __listed_pattern {
    ($name:ident $variant:ident) => {
        $name::$variant
    };
    ($name:ident $variant:ident ( $($tuple:tt)* )) => {
        $name::$variant(..)
    };
    ($name:ident $variant:ident { $($fields:tt)* }) => {
        $name::$variant { .. }
    };
}

#[cfg(test)]
mod tests {
    crate::listed_enum! {
        /// Every kind of variant, with the attributes real enums carry.
        #[allow(dead_code)]
        #[derive(Debug, Clone, PartialEq, serde::Serialize)]
        #[serde(tag = "kind", rename_all = "kebab-case")]
        enum Sample {
            /// A unit variant.
            First,
            Second {
                #[serde(rename = "secondsLeft")]
                seconds_left: u64,
                name: String,
            },
            Third,
        }
    }

    crate::listed_enum! {
        #[allow(dead_code)]
        #[derive(Debug)]
        enum WithTuple {
            One(Vec<String>),
            Two(u8, u8),
            Three,
        }
    }

    #[test]
    fn every_variant_is_listed_in_order() {
        assert_eq!(Sample::VARIANTS, ["First", "Second", "Third"]);
        assert_eq!(WithTuple::VARIANTS, ["One", "Two", "Three"]);
    }

    #[test]
    fn each_value_names_its_variant() {
        assert_eq!(Sample::First.variant(), "First");
        let second = Sample::Second {
            seconds_left: 3,
            name: "x".into(),
        };
        assert_eq!(second.variant(), "Second");
        assert_eq!(Sample::Third.variant(), "Third");
        assert_eq!(WithTuple::One(Vec::new()).variant(), "One");
        assert_eq!(WithTuple::Two(1, 2).variant(), "Two");
        assert_eq!(WithTuple::Three.variant(), "Three");
    }

    #[test]
    fn the_enum_keeps_its_attributes() {
        let second = Sample::Second {
            seconds_left: 3,
            name: "x".into(),
        };
        assert_eq!(
            serde_json::to_value(&second).unwrap(),
            serde_json::json!({ "kind": "second", "secondsLeft": 3, "name": "x" })
        );
        assert_eq!(
            serde_json::to_value(Sample::First).unwrap(),
            serde_json::json!({ "kind": "first" })
        );
    }
}
