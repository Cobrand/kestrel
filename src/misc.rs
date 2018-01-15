
pub trait ClonableIterator<'a>: Iterator {
    fn clone_box(&self) -> Box<ClonableIterator<'a, Item = Self::Item> + 'a>;
}

impl<'a, T: Clone + Iterator + 'a> ClonableIterator<'a> for T {
    fn clone_box(&self) -> Box<ClonableIterator<'a, Item = Self::Item> + 'a> {
        Box::new(self.clone())
    }
}

/// Represents a Box where the first `strip` elements
/// are actually "forgotten" and not returned by 
/// the AsRef<[T]> trait or the `as_slice` call.
pub (crate) struct StrippedBoxedSlice<T> {
    data: Box<[T]>,
    strip: usize,
}

impl<T> ::std::fmt::Debug for StrippedBoxedSlice<T> where T: ::std::fmt::Debug {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        // format StrippedBoxedSlice exactly as a slice starting from strip.
        (&self.data[self.strip..]).fmt(f)
    }
}

impl<T> StrippedBoxedSlice<T> {
    /// Creates a Stripped version of a Box, omitting
    /// the `strip` first elements from AsRef<[u8]> returns.
    ///
    /// This function panics if strip is higher than the length
    /// of the data slice
    pub fn new(data: Box<[T]>, strip: usize) -> StrippedBoxedSlice<T> {
        if strip > data.len() {
            panic!("StrippedBoxedSlice: cannot strip a higher amount than the length of original boxed slice: {} > {}", strip, data.len());
        }
        StrippedBoxedSlice { data, strip }
    }

    pub fn as_slice(&self) -> &[T] {
        &self.data[self.strip..]
    }

    pub fn into_box(self) -> (Box<[T]>, usize) {
        (self.data, self.strip)
    }
}

impl<T> AsRef<[T]> for StrippedBoxedSlice<T> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}