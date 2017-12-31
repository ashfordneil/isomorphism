use ::bitfield::BitField;

/// A single bucket within a hopscotch hashed hashmap.
#[derive(Clone, Debug)]
pub struct Bucket<K, V, B> {
    /// Key, value, ideal hash position triplet.
    pub data: Option<(K, V, usize)>,
    /// A bitfield representing the next <sizeof bitfield> buckets in the hashmap (including this
    /// one). A one in this bitfield means that the bucket contains a value which should be in this
    /// bucket, a zero in this bitfield means that the bucket is either empty, or contains a value
    /// which should not be in this bucket.
    pub neighbourhood: B,
}

impl <K, V, B: BitField + Copy> Bucket<K, V, B> {

    /// Create a new heap allocated array, with a given size, of empty buckets.
    pub fn empty_vec(size: usize) -> Box<[Self]> {
        let mut output = Vec::with_capacity(size);

        for _ in 0..size {
            let element: Self = Bucket {
                data: None,
                neighbourhood: B::one_at(0) & B::zero_at(0),
            };

            output.push(element);
        }

        output.into()
    }
}

#[cfg(test)]
mod tests {
    use bitfield::DefaultBitField;
    use bucket::Bucket;

    #[test]
    fn test_empty_vec() {
        let vec: Box<[Bucket<(), (), DefaultBitField>]> = Bucket::empty_vec(0);
        assert!(vec.len() == 0)
    }

    #[test]
    fn test_full_vec() {
        let length = 1024;
        let vec: Box<[Bucket<(), (), DefaultBitField>]> = Bucket::empty_vec(length);
        assert!(vec.len() == length);
        assert!(vec.iter().all(|element| element.data == None && element.neighbourhood == 0));
    }
}
