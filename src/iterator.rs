use bucket::Bucket;

use std::iter::Iterator;
use std::slice;

/// An iterator over the pairs stored in a BiMap.
pub struct Iter<'a, L, R, B>
where
    L: 'a,
    R: 'a,
    B: 'a,
{
    left_data: slice::Iter<'a, Bucket<L, usize, B>>,
    right_data: &'a [Bucket<R, usize, B>],
}

impl<'a, L, R, B> Iter<'a, L, R, B> {
    pub fn new(
        left_data: slice::Iter<'a, Bucket<L, usize, B>>,
        right_data: &'a [Bucket<R, usize, B>],
    ) -> Self {
        Iter {
            left_data,
            right_data,
        }
    }
}

impl<'a, L, R, B> Iterator for Iter<'a, L, R, B>
where
    L: 'a,
    R: 'a,
{
    type Item = (&'a L, &'a R);

    fn next(&mut self) -> Option<Self::Item> {
        let &mut Iter {
            ref mut left_data,
            right_data,
        } = self;
        left_data
            .filter_map(|bucket| bucket.data.as_ref())
            .map(|&(ref key, value, _)| (key, &right_data[value].data.as_ref().unwrap().0))
            .next()
    }
}

/// An owning iterator over the pairs stored in a BiMap.
pub struct IntoIter<L, R, B> {
    left_data: Box<[Bucket<L, usize, B>]>,
    right_data: Box<[Bucket<R, usize, B>]>,
    index: usize,
}

impl<L, R, B> IntoIter<L, R, B> {
    pub(crate) fn new(
        left_data: Box<[Bucket<L, usize, B>]>,
        right_data: Box<[Bucket<R, usize, B>]>,
    ) -> Self {
        IntoIter {
            left_data,
            right_data,
            index: 0,
        }
    }
}

impl<L, R, B> Iterator for IntoIter<L, R, B> {
    type Item = (L, R);

    fn next(&mut self) -> Option<Self::Item> {
        let &mut IntoIter {
            ref mut left_data,
            ref mut right_data,
            ref mut index,
        } = self;

        loop {
            if *index >= left_data.len() {
                break None;
            }
            if left_data[*index].data.is_some() {
                let (left, right_index, ..) = left_data[*index].data.take().unwrap();
                let (right, ..) = right_data[right_index].data.take().unwrap();
                *index += 1;
                break Some((left, right));
            }
            *index += 1;
        }
    }
}
