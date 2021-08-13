use crate::{hash::HashFunc, sequence};

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, ops::Range};
use sufsort_rs::sufsort::SA;

#[derive(Serialize, Deserialize)]
pub struct Hashing {
    array: Vec<u32>,
    offsets: Vec<u32>,
    k: usize,
    hash_func: HashFunc,
    mask: u32,
}

impl Hashing {
    pub fn new(text: &[u8], k: usize, bits: usize, hash_func: HashFunc) -> Self {
        assert!(text.len() <= u32::MAX as usize + 1);

        let sa = SA::<i32>::new(text);
        let sa = sa.sarray.into_iter().map(|x| x as u32);

        let hashtable_len = 1 << bits;
        let mask = (hashtable_len - 1) as u32;

        let mut counts = vec![0u32; hashtable_len];
        for i in 0..=(text.len() - k) {
            let seq = &text[i..i + k];
            if seq.iter().any(|x| *x == 0 || *x == sequence::DUMMY_CODE) {
                continue;
            }
            counts[(hash_func.hash(seq) & mask) as usize] += 1;
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
        let mut array = vec![0; cum_sum as usize];
        for s in sa {
            if s as usize + k > text.len() {
                continue;
            }
            let seq = &text[s as usize..][..k];
            if seq.iter().any(|x| *x == 0 || *x == sequence::DUMMY_CODE) {
                continue;
            }
            let idx = (hash_func.hash(seq) & mask) as usize;
            array[pos[idx] as usize] = s;
            pos[idx] += 1;
        }

        Self {
            array,
            offsets,
            k,
            hash_func,
            mask,
        }
    }
}

impl super::SuffixArrayVariant for Hashing {
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
        let idx = (self.hash_func.hash(&query[..self.k]) & self.mask) as usize;
        let mut begin = self.offsets[idx] as usize;
        let mut end = self.offsets[idx + 1] as usize;
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
    }
}
