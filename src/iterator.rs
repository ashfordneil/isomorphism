use ::bucket::Bucket;

use std::iter::Iterator;
use std::mem;
use std::ptr;
use std::slice;

unsafe fn duplicate<T>(input: &T) -> T {
    let mut output = mem::uninitialized();
    ptr::copy_nonoverlapping(input, &mut output, mem::size_of::<T>());
    output
}

pub struct BiMapRefIterator<'a, L, R, B> where L: 'a, R: 'a, B: 'a {
    left_data: slice::Iter<'a, Bucket<L, usize, B>>,
    right_data: &'a [Bucket<R, usize, B>],
}

impl<'a, L, R, B> BiMapRefIterator<'a, L, R, B> {
    pub fn new(left_data: slice::Iter<'a, Bucket<L, usize, B>>, right_data: &'a [Bucket<R, usize, B>]) -> Self {
        BiMapRefIterator { left_data, right_data }
    }
}

impl<'a, L, R, B> Iterator for BiMapRefIterator<'a, L, R, B> where L: 'a, R: 'a {
    type Item = (&'a L, &'a R);

    fn next(&mut self) -> Option<Self::Item> {
        let &mut BiMapRefIterator { ref mut left_data, ref right_data } = self;
        left_data
            .filter_map(|bucket| bucket.data.as_ref())
            .map(|&(ref key, value)| (key, &right_data[value].data.as_ref().unwrap().0))
            .next()
    }
}

pub struct BiMapIterator<L, R, B> {
    left_data: Box<[Bucket<L, usize, B>]>,
    right_data: Box<[Bucket<R, usize, B>]>,
    index: usize,
}

impl <L, R, B> BiMapIterator<L, R, B> {
    pub fn new(left_data: Box<[Bucket<L, usize, B>]>, right_data: Box<[Bucket<R, usize, B>]>) -> Self {
        BiMapIterator {
            left_data,
            right_data,
            index: 0
        }
    }
}

impl <L, R, B> Iterator for BiMapIterator<L, R, B> {
    type Item = (L, R);

    fn next(&mut self) -> Option<Self::Item> {
        let &mut BiMapIterator { ref left_data, ref right_data, ref mut index } = self;
        let output = loop {
            *index += 1;
            if *index >= left_data.len() {
                break None;
            }
            if let Some((ref left, value)) = left_data[*index].data {
                let right = &right_data[value].data.as_ref().unwrap().0;
                unsafe {
                    break Some((duplicate(left), duplicate(right)));
                }
            }
        };
        output
    }
}
