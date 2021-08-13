use crate::{hash::HashFunc, sequence};

use serde::{Deserialize, Serialize};
use std::ops::Range;
use sufsort_rs::sufsort::SA;

// Grabowski, S., and M. Raniszewski. "Compact and Hash Based Variants of the Suffix Array."
// Bulletin of the Polish Academy of Sciences: Technical Sciences 65, no. No 4 (2017): 407–18.

const LUT_WIDTH: usize = 2;

#[derive(Serialize, Deserialize)]
pub struct SaHash {
    array: Vec<u32>,
    lut: Vec<(u32, u32)>,
    hashtable: Vec<(u32, u32)>,
    k: usize,
    hash_func: HashFunc,
    mask: u32,
}

impl SaHash {
    pub fn new(text: &[u8], k: usize, bits: usize, hash_func: HashFunc) -> Self {
        assert!(text.len() <= u32::MAX as usize + 1);

        let sa = SA::<i32>::new(text);
        let sa = sa.sarray.into_iter().map(|x| x as u32);

        let mut array = Vec::with_capacity(sa.len());

        let lut_len = 1 << (2 * LUT_WIDTH);
        let mut lut_counts = vec![0u32; lut_len];

        let hashtable_len = 1 << bits;
        let mask = (hashtable_len - 1) as u32;
        let mut hashtable = vec![(u32::MAX, u32::MAX); hashtable_len];
        let mut l = 0;
        let mut j = usize::MAX;
        let mut prev_str = None;

        for s in sa {
            if s as usize + k > text.len() {
                continue;
            }
            let seq = &text[s as usize..][..k];
            if seq.iter().any(|x| *x == 0 || *x == sequence::DUMMY_CODE) {
                continue;
            }

            let i = array.len();
            array.push(s);

            let mut idx = 0;
            for (j, x) in seq[..LUT_WIDTH].iter().enumerate() {
                idx |= (sequence::code_to_two_bit(*x) as usize) << (2 * (LUT_WIDTH - j - 1));
            }
            lut_counts[idx] += 1;

            if let Some(prev_str) = prev_str {
                if seq == prev_str {
                    continue;
                }
            }
            if j != usize::MAX {
                hashtable[j] = (l as u32, i as u32);
            }
            l = i;
            prev_str = Some(seq);
            let init_j = (hash_func.hash(seq) & mask) as usize;
            j = init_j;
            while hashtable[j] != (u32::MAX, u32::MAX) {
                j = (j + 1) & (mask as usize);
                if j == init_j {
                    panic!("Hashtable is full");
                }
            }
        }
        hashtable[j] = (l as u32, array.len() as u32);

        let mut lut = Vec::with_capacity(lut_len);
        let mut lut_cum_sum = 0;
        for count in lut_counts {
            let start = lut_cum_sum;
            lut_cum_sum += count;
            lut.push((start, lut_cum_sum));
        }
        assert_eq!(lut_cum_sum as usize, array.len());

        Self {
            array,
            lut,
            hashtable,
            k,
            hash_func,
            mask,
        }
    }
}

impl super::SuffixArrayVariant for SaHash {
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
        let mut idx = 0;
        for (j, x) in query[..LUT_WIDTH].iter().enumerate() {
            idx |= (sequence::code_to_two_bit(*x) as usize) << (2 * (LUT_WIDTH - j - 1));
        }
        let (beg, end) = self.lut[idx];
        if beg >= end {
            return None;
        }

        let prefix = &query[..self.k];
        let mut j = (self.hash_func.hash(prefix) & self.mask) as usize;
        let (mut begin, mut end) = loop {
            let (l, r) = self.hashtable[j];
            if (l, r) == (u32::MAX, u32::MAX) {
                return None;
            }
            if beg <= l && l < end && &text[self.array[l as usize] as usize..][..self.k] == prefix {
                break (l as usize, r as usize);
            }
            j = (j + 1) & (self.mask as usize);
        };
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
        None
    }

    fn size_bytes(&self) -> usize {
        self.array.len() * std::mem::size_of_val(&self.array[0])
            + self.lut.len() * std::mem::size_of_val(&self.lut[0])
            + self.hashtable.len() * std::mem::size_of_val(&self.hashtable[0])
    }
}
