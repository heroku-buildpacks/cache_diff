//! Generate clean, human readable diffs between two cache structs
//!
//! ## What
//!
//! The `CacheDiff` trait provides a way to compare two structs and generate a list of differences between them.
//! When it returns an empty list, the two structs are identical.
//!
//! You can manually implement the trait, or you can use the `#[derive(CacheDiff)]` macro to automatically generate the implementation.
//!
//! Attributes are:
//!
//!   - `cache_diff(rename = "<new name>")` Specify custom name for the field
//!   - `cache_diff(ignore)` Ignores the given field
//!   - `cache_diff(display = <function>)` Specify a function to call to display the field
//!
//! ## Why
//!
//! Cloud Native Buildpacks (CNBs) written in Rust using [libcnb.rs](https://github.com/heroku/libcnb.rs) use
//! a serializable struct to represent the state of a cache. When that data changes, we need to invalidate the
//! cache, but also report back to the user what changed.
//!
//! Due to the CNB layer implementation, this struct is often called "metadata".
//!
//! ## Install
//!
//! ```shell
//! $ cargo add cache_diff
//! ```
//!
//! For ANSI colored output, add the [`bullet_stream`](https://github.com/heroku-buildpacks/bullet_stream) feature:
//!
//! ```shell
//! $ cargo add cache_diff --features bullet_stream
//! ```
//!
//! ## Derive usage
//!
//! By default a `#[derive(CacheDiff)]` will generate a `diff` function that compares each field in the struct.
//! You can disable this dependency by specifying `features = []`.
//!
//! ```rust
//! use cache_diff::CacheDiff;
//!
//! #[derive(CacheDiff)]
//! struct Metadata {
//!     version: String,
//! }
//! let diff = Metadata { version: "3.4.0".to_string() }
//!     .diff(&Metadata { version: "3.3.0".to_string() });
//!
//! assert_eq!(diff.join(" "), "version (`3.3.0` to `3.4.0`)");
//! ```
//!
//! Struct fields must implement [`PartialEq`](std::cmp::PartialEq) and [`Display`](std::fmt::Display). Also note that [`PartialEq`](std::cmp::PartialEq) on the top level
//! cache struct is not  used or required. If you want to customize equality logic, you can implement
//! the `CacheDiff` trait manually:
//!
//! ```rust
//! use cache_diff::CacheDiff;
//!
//! #[derive(Debug)]
//! struct Metadata {
//!     version: String,
//! }
//!
//! // Implement the trait manually
//! impl CacheDiff for Metadata {
//!    fn diff(&self, old: &Self) -> Vec<String> {
//!         let mut diff = vec![];
//!         // This evaluation logic differs from the derive macro
//!         if !self.custom_compare_eq(old) {
//!             diff.push(format!("Cache is different ({old:?} to {self:?})"));
//!         }
//!
//!         diff
//!    }
//! }
//!
//! impl Metadata {
//!   fn custom_compare_eq(&self, old: &Self) -> bool {
//!       todo!()
//!   }
//! }
//! ```
//!
//! ## Ordering
//!
//! The order of output will match the struct field definition from top to bottom:
//!
//! ```rust
//! use cache_diff::CacheDiff;
//!
//! #[derive(CacheDiff)]
//! struct Metadata {
//!     version: String,
//!     distro: String,
//! }
//! let now = Metadata { version: "3.4.0".to_string(), distro: "Ubuntu".to_string() };
//! let diff = now.diff(&Metadata { version: "3.3.0".to_string(), distro: "Alpine".to_string() });
//!
//! assert_eq!(diff.join(", "), "version (`3.3.0` to `3.4.0`), distro (`Alpine` to `Ubuntu`)");
//! ```
//!
//! ## Rename attributes
//!
//! If your field name is not descriptive enough, you can rename it:
//!
//! ```rust
//! use cache_diff::CacheDiff;
//!
//! #[derive(CacheDiff)]
//! struct Metadata {
//!     #[cache_diff(rename="Ruby version")]
//!     version: String,
//! }
//! let now = Metadata { version: "3.4.0".to_string() };
//! let diff = now.diff(&Metadata { version: "3.3.0".to_string() });
//!
//! assert_eq!(diff.join(" "), "Ruby version (`3.3.0` to `3.4.0`)");
//! ```
//!
//! ## Ignore attributes
//!
//! If the struct contains fields that should not be included in the diff comparison, you can ignore them:
//!
//! ```rust
//! use cache_diff::CacheDiff;
//!
//! #[derive(CacheDiff)]
//! struct Metadata {
//!     version: String,
//!
//!     #[cache_diff(ignore)]
//!     changed_by: String
//! }
//! let now = Metadata { version: "3.4.0".to_string(), changed_by: "Alice".to_string() };
//! let diff = now.diff(&Metadata { version: now.version.clone(), changed_by: "Bob".to_string() });
//!
//! assert!(diff.is_empty());
//! ```
//!
//! ## Handle structs missing display
//!
//! Not all structs implement the [`Display`](std::fmt::Display) trait, for example [`std::path::PathBuf`](std::path::PathBuf) requires that you call `display()` on it.
//!
//! The `#[derive(CacheDiff)]` macro will automatically handle the following conversions for you:
//!
//! - `std::path::PathBuf` (via [`std::path::Path::display`](std::path::Path::display))
//!
//! However, if you have a custom struct that does not implement [`Display`](std::fmt::Display), you can specify a function to call instead:
//!
//! ```rust
//! use cache_diff::CacheDiff;
//!
//! #[derive(CacheDiff)]
//! struct Metadata {
//!     #[cache_diff(display = my_function)]
//!     version: NoDisplay,
//! }
//!
//! #[derive(PartialEq)]
//! struct NoDisplay(String);
//! fn my_function(s: &NoDisplay) -> String {
//!     format!("custom {}", s.0)
//! }
//!
//! let now = Metadata { version: NoDisplay("3.4.0".to_string())};
//! let diff = now.diff(&Metadata { version: NoDisplay("3.3.0".to_string())});
//!
//! assert_eq!(diff.join(" "), "version (`custom 3.3.0` to `custom 3.4.0`)");
//! ```

/// Centralized cache invalidation logic with human readable differences
///
/// When a struct is used to represent values in a cache, this trait can be implemented to
/// to determine whether or not that cache needs to be invalidated.
pub trait CacheDiff {
    /// Given another cache object, returns a list of differences between the two.
    ///
    /// If no differences, return an empty list. An empty list should indicate that the
    /// cache should be retained (not invalidated). One or more items would indicate that
    /// the cached value should be invalidated.
    fn diff(&self, old: &Self) -> Vec<String>;

    #[cfg(feature = "bullet_stream")]
    fn fmt_value<T: std::fmt::Display>(&self, value: &T) -> String {
        bullet_stream::style::value(value.to_string())
    }

    /// How values are displayed in the diff output, the default is to wrap them in backticks
    ///
    /// Enable ANSI colors with `features = ["bullet_stream"]`
    #[cfg(not(feature = "bullet_stream"))]
    fn fmt_value<T: std::fmt::Display>(&self, value: &T) -> String {
        format!("`{}`", value)
    }
}
pub use cache_diff_derive::CacheDiff;
