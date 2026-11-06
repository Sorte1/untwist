const WORD: usize = u64::BITS as usize;

pub(super) struct Claims {
    bits: Vec<u64>,
    len: usize,
    count: usize,
    in_order: bool,
}

impl Claims {
    pub(super) fn new(len: usize) -> Self {
        Self {
            bits: vec![0; len.div_ceil(WORD)],
            len,
            count: 0,
            in_order: true,
        }
    }

    pub(super) fn claim(&mut self, index: usize) -> bool {
        let Some(word) = self.bits.get_mut(index / WORD) else {
            return false;
        };
        let bit = 1 << (index % WORD);
        if *word & bit != 0 {
            return false;
        }
        *word |= bit;
        self.in_order &= index == self.count;
        self.count += 1;
        true
    }

    pub(super) fn is_claimed(&self, index: usize) -> bool {
        self.bits
            .get(index / WORD)
            .is_some_and(|word| word & (1 << (index % WORD)) != 0)
    }

    pub(super) fn count(&self) -> usize {
        self.count
    }

    pub(super) fn in_order(&self) -> bool {
        self.in_order && self.count == self.len
    }

    pub(super) fn unclaimed(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.len).filter(|&index| !self.is_claimed(index))
    }
}
