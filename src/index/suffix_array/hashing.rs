use serde::{Deserialize, Serialize};
use std::ops::Range;
use sufsort_rs::sufsort::SA;
use xxhash_rust::xxh32::xxh32;

#[derive(Serialize, Deserialize)]
pub struct Hashing {
    array: Vec<u32>,
    offsets: Vec<u32>,
    k: usize,
    mask: usize,
}

impl Hashing {
    pub fn new(text: &[u8], k: usize, bits: usize) -> Self {
        assert!(text.len() <= u32::MAX as usize + 1);

        let sa = SA::<i32>::new(text);
        let array: Vec<_> = sa.sarray.into_iter().map(|x| x as u32).collect();

        let hashtable_len = 1 << bits;
        let mask = hashtable_len - 1;
        let mut counts = vec![0u32; hashtable_len];
        for i in 0..=(text.len() - k) {
            let seq = &text[i..i + k];
            if seq
                .iter()
                .any(|x| *x == 0 || *x == crate::sequence::DUMMY_CODE)
            {
                continue;
            }
            counts[xxh32(seq, 0) as usize & mask] += 1;
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
        let mut ssa = vec![0; array.len()];
        for s in array.into_iter() {
            if s as usize + k > text.len() {
                continue;
            }
            let seq = &text[s as usize..][..k];
            if seq
                .iter()
                .any(|x| *x == 0 || *x == crate::sequence::DUMMY_CODE)
            {
                continue;
            }
            let idx = xxh32(seq, 0) as usize & mask;
            let p = pos[idx] as usize;
            ssa[p] = s;
            pos[idx] += 1;
        }

        Self {
            array: ssa,
            offsets,
            k,
            mask,
        }
    }
}

impl super::SuffixArrayImpl for Hashing {
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
        let hash = xxh32(&query[..self.k], 0) as usize & self.mask;
        let mut begin = self.offsets[hash] as usize;
        let mut end = self.offsets[hash + 1] as usize;
        if begin == end {
            return None;
        }

        unsafe {
            super::equal_range(
                &self.array,
                text.as_ptr(),
                query.as_ptr(),
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
