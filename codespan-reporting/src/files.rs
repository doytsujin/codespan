//! Source file support for diagnostic reporting.
//!
//! The main trait defined in this module is the [`Files`] trait, which provides
//! provides the minimum amount of functionality required for printing [`Diagnostics`]
//! with the [`term::emit`] function.
//!
//! Simple implementations of this trait are implemented:
//!
//! - [`SimpleFile`]: For single-file use-cases
//! - [`SimpleFiles`]: For single-file use-cases
//!
//! These data structures provide a pretty minimal API, however,
//! so end-users are encouraged to create their own implementations for their
//! own specific use-cases, such as an implementation that accesses the file
//! system directly (and caches the line start locations), or an implementation
//! using an incremental compilation library like [`salsa`].
//!
//! [`term::emit`]: crate::term::emit
//! [`Diagnostics`]: crate::diagnostic::Diagnostic
//! [`Files`]: Files
//! [`SimpleFile`]: SimpleFile
//! [`SimpleFiles`]: SimpleFiles
//!
//! [`salsa`]: https://crates.io/crates/salsa

use std::ops::Range;

/// A minimal interface for accessing source files when rendering diagnostics.
///
/// A lifetime parameter `'a` is provided to allow any of the returned values to returned by reference.
/// This is to workaround the lack of higher kinded lifetime parameters.
/// This can be ignored if this is not needed, however.
pub trait Files<'a> {
    /// A unique identifier for files in the file provider. This will be used
    /// for rendering `diagnostic::Label`s in the corresponding source files.
    type FileId: 'a + Copy + PartialEq;
    /// The user-facing name of a file, to be displayed in diagnostics.
    type Name: 'a + std::fmt::Display;
    /// The source code of a file.
    type Source: 'a + AsRef<str>;

    /// The user-facing name of a file.
    fn name(&'a self, id: Self::FileId) -> Option<Self::Name>;

    /// The source code of a file.
    fn source(&'a self, id: Self::FileId) -> Option<Self::Source>;

    /// The index of the line at the given byte index.
    fn line_index(&'a self, id: Self::FileId, byte_index: usize) -> Option<usize>;

    /// The user-facing line number at the given line index.
    ///
    /// This can be useful for implementing something like the
    /// [C preprocessor's `#line` macro][line-macro],
    /// but is usually 1-indexed from the beginning of the file.
    ///
    /// [line-macro]: https://en.cppreference.com/w/c/preprocessor/line
    #[allow(unused_variables)]
    fn line_number(&'a self, id: Self::FileId, line_index: usize) -> Option<usize> {
        Some(line_index + 1)
    }

    /// The index of the line at the given byte index.
    fn line_range(&'a self, id: Self::FileId, line_index: usize) -> Option<Range<usize>>;
}

/// The column index at the given byte index in the source file.
/// This is the number of characters to the given byte index.
///
/// If the byte index is smaller than the start of the line, then `0` is returned.
/// If the byte index is past the end of the line, the column index of the last
/// character `+ 1` is returned.
///
/// # Example
///
/// ```rust
/// use codespan_reporting::files;
///
/// let source = "\n\n🗻∈🌏\n\n";
///
/// assert_eq!(files::column_index(source, 0..1, 0), 0);
/// assert_eq!(files::column_index(source, 2..13, 0), 0);
/// assert_eq!(files::column_index(source, 2..13, 2 + 0), 0);
/// assert_eq!(files::column_index(source, 2..13, 2 + 1), 0);
/// assert_eq!(files::column_index(source, 2..13, 2 + 4), 1);
/// assert_eq!(files::column_index(source, 2..13, 2 + 8), 2);
/// assert_eq!(files::column_index(source, 2..13, 2 + 10), 2);
/// assert_eq!(files::column_index(source, 2..13, 2 + 11), 3);
/// assert_eq!(files::column_index(source, 2..13, 2 + 12), 3);
/// ```
pub fn column_index(source: &str, line_range: Range<usize>, byte_index: usize) -> usize {
    let end_index = std::cmp::min(byte_index, std::cmp::min(line_range.end, source.len()));

    (line_range.start..end_index)
        .filter(|byte_index| source.is_char_boundary(byte_index + 1))
        .count()
}

/// The 1-indexed column number at the given byte index.
///
/// # Example
///
/// ```rust
/// use codespan_reporting::files;
///
/// let source = "\n\n🗻∈🌏";
/// let line_range = 2..13;
///
/// assert_eq!(files::column_number(source, 0..1, 0), 1);
/// assert_eq!(files::column_number(source, 2..13, 0), 1);
/// assert_eq!(files::column_number(source, 2..13, 2 + 0), 1);
/// assert_eq!(files::column_number(source, 2..13, 2 + 1), 1);
/// assert_eq!(files::column_number(source, 2..13, 2 + 4), 2);
/// assert_eq!(files::column_number(source, 2..13, 2 + 8), 3);
/// assert_eq!(files::column_number(source, 2..13, 2 + 10), 3);
/// assert_eq!(files::column_number(source, 2..13, 2 + 11), 4);
/// assert_eq!(files::column_number(source, 2..13, 2 + 12), 4);
/// ```
pub fn column_number(source: &str, line_range: Range<usize>, byte_index: usize) -> usize {
    column_index(source, line_range, byte_index) + 1
}

/// Return the starting byte index of each line in the source string.
///
/// This can make it easier to implement new [`Files`] implementations.
///
/// [`Files`]: Files
///
/// # Example
///
/// ```rust
/// use codespan_reporting::files;
///
/// let source = "foo\nbar\r\n\nbaz";
/// let line_starts: Vec<_> = files::line_starts(source).collect();
///
/// assert_eq!(
///     line_starts,
///     [
///         0,  // "foo\n"
///         4,  // "bar\r\n"
///         9,  // ""
///         10, // "baz"
///     ],
/// );
///
/// fn line_index(line_starts: &[usize], byte_index: usize) -> Option<usize> {
///     match line_starts.binary_search(&byte_index) {
///         Ok(line) => Some(line),
///         Err(next_line) => Some(next_line - 1),
///     }
/// }
///
/// assert_eq!(line_index(&line_starts, 5), Some(1));
/// ```
pub fn line_starts<'source>(source: &'source str) -> impl 'source + Iterator<Item = usize> {
    std::iter::once(0).chain(source.match_indices('\n').map(|(i, _)| i + 1))
}

/// A file database that contains a single source file.
///
/// Because there is only single file in this database we use `()` as a [`FileId`].
///
/// This is useful for simple language tests, but it might be worth creating a
/// custom implementation when a language scales beyond a certain size.
///
/// [`FileId`]: Files::FileId
#[derive(Debug, Clone)]
pub struct SimpleFile<Name, Source> {
    /// The name of the file.
    name: Name,
    /// The source code of the file.
    source: Source,
    /// The starting byte indices in the source code.
    line_starts: Vec<usize>,
}

impl<Name, Source> SimpleFile<Name, Source>
where
    Name: std::fmt::Display,
    Source: AsRef<str>,
{
    /// Create a new source file.
    pub fn new(name: Name, source: Source) -> SimpleFile<Name, Source> {
        SimpleFile {
            name,
            line_starts: line_starts(source.as_ref()).collect(),
            source,
        }
    }

    /// Return the name of the file.
    pub fn name(&self) -> &Name {
        &self.name
    }

    /// Return the source of the file.
    pub fn source(&self) -> &Source {
        &self.source
    }

    fn line_start(&self, line_index: usize) -> Option<usize> {
        use std::cmp::Ordering;

        match line_index.cmp(&self.line_starts.len()) {
            Ordering::Less => self.line_starts.get(line_index).cloned(),
            Ordering::Equal => Some(self.source.as_ref().len()),
            Ordering::Greater => None,
        }
    }
}

impl<'a, Name, Source> Files<'a> for SimpleFile<Name, Source>
where
    Name: 'a + std::fmt::Display + Clone,
    Source: 'a + AsRef<str>,
{
    type FileId = ();
    type Name = Name;
    type Source = &'a str;

    fn name(&self, (): ()) -> Option<Name> {
        Some(self.name.clone())
    }

    fn source(&self, (): ()) -> Option<&str> {
        Some(self.source.as_ref())
    }

    fn line_index(&self, (): (), byte_index: usize) -> Option<usize> {
        match self.line_starts.binary_search(&byte_index) {
            Ok(line) => Some(line),
            Err(next_line) => Some(next_line - 1),
        }
    }

    fn line_range(&self, (): (), line_index: usize) -> Option<Range<usize>> {
        let line_start = self.line_start(line_index)?;
        let next_line_start = self.line_start(line_index + 1)?;

        Some(line_start..next_line_start)
    }
}

/// A file database that can store multiple source files.
///
/// This is useful for simple language tests, but it might be worth creating a
/// custom implementation when a language scales beyond a certain size.
#[derive(Debug, Clone)]
pub struct SimpleFiles<Name, Source> {
    files: Vec<SimpleFile<Name, Source>>,
}

impl<Name, Source> SimpleFiles<Name, Source>
where
    Name: std::fmt::Display,
    Source: AsRef<str>,
{
    /// Create a new files database.
    pub fn new() -> SimpleFiles<Name, Source> {
        SimpleFiles { files: Vec::new() }
    }

    /// Add a file to the database, returning the handle that can be used to
    /// refer to it again.
    pub fn add(&mut self, name: Name, source: Source) -> usize {
        let file_id = self.files.len();
        self.files.push(SimpleFile::new(name, source));
        file_id
    }

    /// Get the file corresponding to the given id.
    pub fn get(&self, file_id: usize) -> Option<&SimpleFile<Name, Source>> {
        self.files.get(file_id)
    }
}

impl<'a, Name, Source> Files<'a> for SimpleFiles<Name, Source>
where
    Name: 'a + std::fmt::Display + Clone,
    Source: 'a + AsRef<str>,
{
    type FileId = usize;
    type Name = Name;
    type Source = &'a str;

    fn name(&self, file_id: usize) -> Option<Name> {
        Some(self.get(file_id)?.name().clone())
    }

    fn source(&self, file_id: usize) -> Option<&str> {
        Some(self.get(file_id)?.source().as_ref())
    }

    fn line_index(&self, file_id: usize, byte_index: usize) -> Option<usize> {
        self.get(file_id)?.line_index((), byte_index)
    }

    fn line_range(&self, file_id: usize, line_index: usize) -> Option<Range<usize>> {
        self.get(file_id)?.line_range((), line_index)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const TEST_SOURCE: &str = "foo\nbar\r\n\nbaz";

    #[test]
    fn line_starts() {
        let file = SimpleFile::new("test", TEST_SOURCE);

        assert_eq!(
            file.line_starts,
            [
                0,  // "foo\n"
                4,  // "bar\r\n"
                9,  // ""
                10, // "baz"
            ],
        );
    }

    #[test]
    fn line_span_sources() {
        let file = SimpleFile::new("test", TEST_SOURCE);

        let line_sources = (0..4)
            .map(|line| {
                let line_range = file.line_range((), line).unwrap();
                &file.source[line_range]
            })
            .collect::<Vec<_>>();

        assert_eq!(line_sources, ["foo\n", "bar\r\n", "\n", "baz"]);
    }
}
