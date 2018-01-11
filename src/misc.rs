
pub trait ClonableIterator<'a>: Iterator {
    fn clone_box(&self) -> Box<ClonableIterator<'a, Item = Self::Item> + 'a>;
}

impl<'a, T: Clone + Iterator + 'a> ClonableIterator<'a> for T {
    fn clone_box(&self) -> Box<ClonableIterator<'a, Item = Self::Item> + 'a> {
        Box::new(self.clone())
    }
}