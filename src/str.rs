use crate::prelude::*;
use itertools::Itertools;

impl<'a> Divisible for &'a str {
    type Controlled = True;
    fn should_be_divided(&self) -> bool {
        self.len() > 1
    }
    fn divide(self) -> (Self, Self) {
        let mid = self.len() / 2;
        self.divide_at(mid)
    }
    fn divide_at(self, index: usize) -> (Self, Self) {
        let up = index..self.len();
        let down = (0..index).rev();
        let real_index = up
            .interleave(down)
            .find(|&i| self.is_char_boundary(i))
            .unwrap_or(self.len());
        self.split_at(real_index)
    }
}

impl<'a> Divisible for &'a mut str {
    type Controlled = True;
    fn should_be_divided(&self) -> bool {
        self.len() > 1
    }
    fn divide(self) -> (Self, Self) {
        let mid = self.len() / 2;
        self.divide_at(mid)
    }
    fn divide_at(self, index: usize) -> (Self, Self) {
        let up = index..self.len();
        let down = (0..index).rev();
        let real_index = up
            .interleave(down)
            .find(|&i| self.is_char_boundary(i))
            .unwrap_or(self.len());
        self.split_at_mut(real_index)
    }
}
