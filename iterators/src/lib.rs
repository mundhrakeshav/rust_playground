pub struct FlattenedIterator<T>
where
    T: Iterator,
    T::Item: IntoIterator,
{
    // outer = [[1, 2, 3], [1, 3, 6]]
    // inner_iterator(s) = [1, 2, 3]/ [1, 3, 6]
    inner_iterator: Option<<T::Item as IntoIterator>::IntoIter>,
    outer: T,
}

impl<T> FlattenedIterator<T>
where
    T: Iterator,
    T::Item: IntoIterator,
{
    fn new(outer: T) -> Self {
        FlattenedIterator {
            outer,
            inner_iterator: None,
        }
    }
}

impl<T> Iterator for FlattenedIterator<T>
where
    T: Iterator,
    T::Item: IntoIterator,
{
    type Item = <T::Item as IntoIterator>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(ref mut inner_iter) = self.inner_iterator {
                if let Some(i) = inner_iter.next() {
                    return Some(i);
                }
                self.inner_iterator = None;
            }

            self.inner_iterator = Some(self.outer.next()?.into_iter());
        }
    }
}

pub fn flatten<T>(iter: T) -> FlattenedIterator<T::IntoIter>
where
    T: IntoIterator,
    T::Item: IntoIterator,
{
    FlattenedIterator::new(iter.into_iter())
}


#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn empty() {
        assert_eq!(flatten(std::iter::empty::<Vec<()>>()).count(), 0);
    }

    #[test]
    fn two() {
        assert_eq!(flatten(std::iter::once(vec!["a"])).count(), 1);
        assert_eq!(flatten(vec![vec!["a"], vec!["1"]]).count(), 2);
    }
}
