use crate::sequence;

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, ops::Range};
use sufsort_rs::sufsort::SA;

#[derive(Serialize, Deserialize)]
pub struct VariableLengthBuckets {
    array: Vec<u32>,
    offsets: Vec<u32>,
    buckets: Vec<u32>,
    k: usize,
    f: f64,
}

impl VariableLengthBuckets {
    pub fn new(text: &[u8], k: usize, f: f64) -> Self {
        assert!(text.len() <= u32::MAX as usize + 1);

        let sa = SA::<i32>::new(text);

        let offsets_len = 1 << (2 * k);
        let mut counts = vec![0u32; offsets_len];
        for i in 0..=(text.len() - k) {
            let seq = &text[i..i + k];
            if seq.iter().any(|x| *x == 0 || *x == sequence::DUMMY_CODE) {
                continue;
            }
            let mut idx = 0;
            for (j, x) in seq.iter().enumerate() {
                idx |= (sequence::code_to_two_bit(*x) as usize) << (2 * j);
            }
            counts[idx] += 1;
        }

        let mut buckets_len = 0;
        let mut offsets = Vec::new();
        for count in &counts {
            let w = ((*count as f64 * f).log(4.0).max(0.0) as usize).min(31);
            offsets.push(buckets_len as u32);
            buckets_len += 1 << (2 * w);
        }
        offsets.push(buckets_len as u32);

        let mut ssa = Vec::new();
        let mut buckets = vec![u32::MAX; buckets_len];
        let mut prev_bucket = 0;
        for s in sa.sarray.into_iter().map(|x| x as u32) {
            if s as usize + k > text.len() {
                continue;
            }
            let seq = &text[s as usize..s as usize + k];
            if seq.iter().any(|x| *x == 0 || *x == sequence::DUMMY_CODE) {
                continue;
            }
            let mut idx = 0;
            for (j, x) in seq.iter().rev().enumerate() {
                idx |= (sequence::code_to_two_bit(*x) as usize) << (2 * j);
            }
            let w = ((offsets[idx + 1] - offsets[idx]).trailing_zeros() / 2) as usize;
            if s as usize + k + w > text.len() {
                continue;
            }
            let seq2 = &text[s as usize + k..][..w];
            if seq2.iter().any(|x| *x == 0 || *x == sequence::DUMMY_CODE) {
                continue;
            }
            let mut idx2 = 0;
            for (j, x) in seq2.iter().rev().enumerate() {
                idx2 |= (sequence::code_to_two_bit(*x) as usize) << (2 * j);
            }
            assert!(idx2 < (1 << (2 * w)));
            let j = offsets[idx] as usize + idx2;
            assert!(prev_bucket <= j, "{} {}", prev_bucket, j);
            if buckets[j] == u32::MAX {
                buckets[j] = ssa.len() as u32;
            }
            prev_bucket = j;
            ssa.push(s);
        }
        buckets.push(ssa.len() as u32);

        for i in (0..buckets_len).rev() {
            if buckets[i] == u32::MAX {
                buckets[i] = buckets[i + 1];
            }
        }

        for i in 0..offsets_len {
            assert!(offsets[i] <= offsets[i + 1]);
        }
        for i in 0..buckets_len {
            assert!(buckets[i] <= buckets[i + 1]);
        }

        Self {
            array: ssa,
            offsets,
            buckets,
            k,
            f,
        }
    }
}

impl super::SuffixArrayVariant for VariableLengthBuckets {
    fn index_to_pos(&self, index: usize) -> usize {
        self.array[index] as usize
    }

    fn extension_search(
        &self,
        text: &[u8],
        query: &[u8],
        min_len: usize,
        max_hits: usize,
    ) -> Option<(Range<usize>, usize)> {
        debug_assert!(self.k <= min_len && min_len <= query.len());

        let mut idx = 0;
        for (i, x) in query[..self.k].iter().rev().enumerate() {
            idx |= (sequence::code_to_two_bit(*x) as usize) << (2 * i);
        }

        let bucket_begin = self.offsets[idx] as usize;
        let bucket_end = self.offsets[idx + 1] as usize;
        if bucket_begin == bucket_end {
            return None;
        }

        let w = ((bucket_end - bucket_begin).trailing_zeros() / 2) as usize;
        let mut idx2 = 0;
        for (i, x) in query[self.k..self.k + w].iter().rev().enumerate() {
            idx2 |= (sequence::code_to_two_bit(*x) as usize) << (2 * i);
        }

        let mut begin = self.buckets[bucket_begin + idx2] as usize;
        let mut end = self.buckets[bucket_begin + idx2 + 1] as usize;
        if begin == end {
            return None;
        }

        unsafe {
            super::equal_range(
                &self.array,
                text.as_ptr().add(self.k + w),
                query.as_ptr().add(self.k + w),
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

    fn bucket_size_distribution(&self) -> BTreeMap<usize, usize> {
        let mut map = BTreeMap::new();
        for i in 0..(self.offsets.len() - 1) {
            let size = self.offsets[i + 1] - self.offsets[i];
            map.entry(size as usize)
                .and_modify(|i| *i += 1)
                .or_insert(1);
        }
        map
    }

    fn size_bytes(&self) -> usize {
        self.array.len() * std::mem::size_of_val(&self.array[0])
            + self.offsets.len() * std::mem::size_of_val(&self.offsets[0])
            + self.buckets.len() * std::mem::size_of_val(&self.buckets[0])
    }
}
