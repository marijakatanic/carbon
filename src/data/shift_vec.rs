use std::ops::{Index, IndexMut};

pub(crate) struct ShiftVec<T> {
    offset: usize,
    items: Vec<T>,
}

impl<T> ShiftVec<T> {
    pub fn new(offset: usize) -> Self {
        ShiftVec {
            offset,
            items: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.offset + self.items.len()
    }

    pub fn push(&mut self, item: T) {
        self.items.push(item)
    }
}

impl<T> Index<usize> for ShiftVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        if index < self.offset || index >= self.offset + self.items.len() {
            panic!("Called `ShiftVec::Index` with index out of bounds");
        }

        &self.items[index - self.offset]
    }
}

impl<T> IndexMut<usize> for ShiftVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index < self.offset || index >= self.offset + self.items.len() {
            panic!("Called `ShiftVec::Index` with index out of bounds");
        }

        &mut self.items[index - self.offset]
    }
}
