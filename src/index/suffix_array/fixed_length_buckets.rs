use crate::sequence;
use serde::{Deserialize, Serialize};
use std::ops::Range;
use sufsort_rs::sufsort::SA;

#[derive(Serialize, Deserialize)]
pub struct FixedLengthBuckets {
    pub array: Vec<u32>,
    pub offsets: Vec<u32>,
    pub bucket_width: usize,
}

impl FixedLengthBuckets {
    pub fn new(text: &[u8], bucket_width: usize) -> Self {
        assert!(text.len() <= u32::MAX as usize + 1);
        assert!(bucket_width * 2 < std::mem::size_of::<usize>() * 8);

        let sa = SA::<i32>::new(text);
        let array: Vec<_> = sa.sarray.into_iter().map(|x| x as u32).collect();

        let buckets_len = 1 << (2 * bucket_width);
        let mut counts = vec![0u32; buckets_len];
        for i in 0..=(text.len() - bucket_width) {
            let seq = &text[i..i + bucket_width];
            if seq.iter().any(|x| *x == 0 || *x == sequence::DUMMY_CODE) {
                continue;
            }
            let mut idx = 0;
            for (j, x) in seq.iter().enumerate() {
                idx |= (sequence::code_to_two_bit(*x) as usize) << (2 * j);
            }
            counts[idx] += 1;
        }

        let mut cum_sum = 0;
        for count in counts.iter_mut() {
            let x = *count;
            *count = cum_sum;
            cum_sum += x;
        }
        counts.push(cum_sum);

        let offsets = counts;

        let mut pos = offsets.clone();
        let mut ssa = vec![0; cum_sum as usize];
        for s in array.iter() {
            if *s as usize + bucket_width > text.len() {
                continue;
            }
            let seq = &text[*s as usize..][..bucket_width];
            if seq.iter().any(|x| *x == 0 || *x == sequence::DUMMY_CODE) {
                continue;
            }

            let mut idx = 0;
            for (j, x) in seq.iter().enumerate() {
                idx |= (sequence::code_to_two_bit(*x) as usize) << (2 * j);
            }
            let p = pos[idx] as usize;
            ssa[p] = *s;
            pos[idx] += 1;
        }

        Self {
            array: ssa,
            offsets,
            bucket_width,
        }
    }
}

impl super::SuffixArrayImpl for FixedLengthBuckets {
    fn index_to_pos(&self, i: usize) -> usize {
        self.array[i] as usize
    }

    fn extension_search(
        &self,
        text: &[u8],
        query: &[u8],
        min_len: usize,
        max_hits: usize,
    ) -> Option<(Range<usize>, usize)> {
        debug_assert!(self.bucket_width <= min_len && min_len <= query.len());

        let mut idx = 0;
        for (i, x) in query[..self.bucket_width].iter().enumerate() {
            idx |= (sequence::code_to_two_bit(*x) as usize) << (2 * i);
        }

        let mut begin = self.offsets[idx] as usize;
        let mut end = self.offsets[idx + 1] as usize;
        if begin == end {
            return None;
        }

        unsafe {
            super::equal_range(
                &self.array,
                text.as_ptr().add(self.bucket_width),
                query.as_ptr().add(self.bucket_width),
                query.as_ptr().add(min_len),
                &mut begin,
                &mut end,
            );
        }
        if begin == end {
            return None;
        }

        let mut depth = min_len;
        let query_len = query.len();

        while depth < query_len && end - begin > max_hits {
            unsafe {
                super::equal_range(
                    &self.array,
                    text.as_ptr().add(depth),
                    query.as_ptr().add(depth),
                    query.as_ptr().add(depth + 1),
                    &mut begin,
                    &mut end,
                );
            }
            if begin == end {
                return None;
            }
            depth += 1;
        }

        if depth == query_len && end - begin > max_hits {
            None
        } else {
            Some((begin..end, depth))
        }
    }

    fn bucket_size_distribution(&self) -> Option<std::collections::BTreeMap<usize, usize>> {
        let mut map = std::collections::BTreeMap::new();
        for i in 0..(self.offsets.len() - 1) {
            let size = self.offsets[i + 1] - self.offsets[i];
            map.entry(size as usize)
                .and_modify(|i| *i += 1)
                .or_insert(1);
        }
        Some(map)
    }
}
