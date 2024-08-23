use std::{num::NonZeroU16, rc::Rc};

use bit_vec::BitVec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Square {
    x: u16,
    y: u16,
    size: NonZeroU16,
}

impl Square {
    fn new(x: u16, y: u16, size: u16) -> Self {
        Self {
            x,
            y,
            size: NonZeroU16::new(size).unwrap(),
        }
    }

    fn size(&self) -> u16 {
        self.size.get()
    }

    fn bitmap(&self, grid_size: u16) -> BitVec {
        let grid_size = grid_size as usize;
        BitVec::from_fn(grid_size * grid_size, |i| {
            let bx = i % grid_size;
            let by = i / grid_size;
            self.contains(bx as u16, by as u16)
        })
    }

    fn contains(&self, x: u16, y: u16) -> bool {
        self.x <= x && x < self.x + self.size.get() && self.y <= y && y < self.y + self.size.get()
    }

    // Returns { sq : Sqaure | sq.sz = self.sz + 1 && self ⊂ sq }
    fn next_larger_squares(&self, grid_size: u16) -> Vec<Square> {
        let mut squares = Vec::new();
        if self.size.get() == grid_size {
            return squares;
        }

        let new_size = self.size.get() + 1;

        let min_x = self.x.saturating_sub(new_size - 1);
        let max_x = if self.x + new_size <= grid_size {
            self.x
        } else {
            self.x - 1
        };

        let min_y = self.y.saturating_sub(new_size - 1);
        let max_y = if self.y + new_size <= grid_size {
            self.y
        } else {
            self.y - 1
        };

        for x in min_x..=max_x {
            for y in min_y..=max_y {
                let sq = Square::new(x, y, new_size);
                debug_assert!(sq.contains(self.x, self.y));
                squares.push(sq);
            }
        }

        squares
    }

    // Returns { sq : Square | sq.sz = self.sz - 1 && sq ⊂ self }
    //
    // There are only 4 such squares
    fn next_smaller_squares(&self) -> Option<[Square; 4]> {
        if self.size.get() == 1 {
            return None;
        }

        let directions = [(0, 0), (1, 0), (0, 1), (1, 1)];
        let squares = directions.map(|(dx, dy)| {
            let x = self.x + dx;
            let y = self.y + dy;
            Square::new(x, y, self.size.get() - 1)
        });

        Some(squares)
    }

    /// Perfect hash function for square objects
    fn idx(&self, grid_size: usize) -> usize {
        debug_assert!(self.x < grid_size as u16);
        debug_assert!(self.y < grid_size as u16);
        debug_assert!(self.size() > 0);

        let size = self.size.get() as usize;
        let x = self.x as usize;
        let y = self.y as usize;

        x + y * grid_size + (size - 1) * grid_size * grid_size
    }

    fn from_index(idx: usize, grid_size: usize) -> Square {
        let size = idx / (grid_size * grid_size);
        let idx = idx % (grid_size * grid_size);

        let y = idx / grid_size;
        let x = idx % grid_size;

        Square::new(x as u16, y as u16, size as u16 + 1)
    }
}

/// A lookup table for the 'next larger squares' operation
#[derive(Debug)]
pub struct LookupTable {
    size: u16,

    // The shrinking table uses the sq_idx method to get the entry for a square
    smaller_squares: Vec<Option<[Square; 4]>>,

    // The growing table has arbitrary entry length, so we use a table to map sq_idx to the start of the entry
    // The end of the entry is the start of the next entry
    index: Vec<usize>,
    larger_squares: Vec<Square>,

    // Precompute the bitmap for each square, uses sq_idx
    bitmaps: Vec<BitVec>,
}

impl LookupTable {
    pub fn new(size: u16) -> Self {
        let gz = size as usize;

        let mut smaller_squares = vec![None; gz * gz * gz];
        let mut index = vec![0; gz * gz * gz];
        let mut larger_squares = Vec::new();
        let mut bitmaps = vec![BitVec::new(); gz * gz * gz];

        for idx in 0..gz * gz * gz {
            let sq = Square::from_index(idx, gz);

            debug_assert!(sq.size.get() >= 1);

            if let Some(squares) = sq.next_smaller_squares() {
                smaller_squares[idx] = Some(squares);
            }

            let start = larger_squares.len();
            larger_squares.extend(sq.next_larger_squares(gz as u16));
            index[idx] = start;

            bitmaps[idx] = sq.bitmap(gz as u16);
        }

        Self {
            size: gz as u16,
            smaller_squares,
            index,
            larger_squares,
            bitmaps,
        }
    }

    fn get_larger(&self, square: Square) -> &[Square] {
        let idx = square.idx(self.size as usize);
        let start = self.index[idx];
        let end = self
            .index
            .get(idx + 1)
            .copied()
            .unwrap_or(self.larger_squares.len());

        &self.larger_squares[start..end]
    }

    fn get_smaller(&self, square: Square) -> Option<&[Square; 4]> {
        let idx = square.idx(self.size as usize);
        self.smaller_squares[idx].as_ref()
    }

    fn get_bitmap(&self, square: Square) -> BitVec {
        self.bitmaps[square.idx(self.size as usize)].clone()
    }
}

#[derive(Debug, Clone)]
pub struct PumpkinPatch {
    bitmap: BitVec,
    ids: Vec<Option<NonZeroU16>>,
    ids_transposed: Vec<Option<NonZeroU16>>,
    size: u16,
    lookup_table: Rc<LookupTable>,
}

impl PumpkinPatch {
    pub fn new(size: u16, lookup_table: Rc<LookupTable>) -> Self {
        let sz = size as usize;
        Self {
            bitmap: BitVec::from_elem(sz * sz, false),
            ids: vec![None; sz * sz],
            ids_transposed: vec![None; sz * sz],
            size,
            lookup_table,
        }
    }

    pub fn new_make_table(size: u16) -> Self {
        Self::new(size, Rc::new(LookupTable::new(size)))
    }

    fn index(&self, x: u16, y: u16) -> usize {
        (y * self.size + x) as usize
    }

    pub fn get(&self, x: u16, y: u16) -> Option<NonZeroU16> {
        self.ids[self.index(x, y)]
    }

    pub fn contains(&self, x: u16, y: u16) -> bool {
        self.bitmap[self.index(x, y)]
    }

    fn check_boundary(&self, sq: &Square) -> bool {
        #[cfg(debug_assertions)]
        println!("Checking boundary for {:?}", sq);

        // north is +y
        if sq.y < self.size - sq.size.get() {
            let inside_idx: usize = ((sq.y + sq.size.get() - 1) * self.size + sq.x) as usize;
            let inside = &self.ids[inside_idx..inside_idx + sq.size.get() as usize];
            let outside_idx: usize = ((sq.y + sq.size.get()) * self.size + sq.x) as usize;
            let outside = &self.ids[outside_idx..outside_idx + sq.size.get() as usize];

            #[cfg(debug_assertions)]
            println!("NORTH inside: {:?}, outside: {:?}", inside, outside);

            if inside
                .iter()
                .zip(outside.iter())
                .any(|(a, b)| b.is_some() && a == b)
            {
                return false;
            }
        }

        // south is -y
        if sq.y > 0 {
            let inside_idx = (sq.y * self.size + sq.x) as usize;
            let inside = &self.ids[inside_idx..inside_idx + sq.size.get() as usize];
            let outside_idx = ((sq.y - 1) * self.size + sq.x) as usize;
            let outside = &self.ids[outside_idx..outside_idx + sq.size.get() as usize];

            #[cfg(debug_assertions)]
            println!("SOUTH inside: {:?}, outside: {:?}", inside, outside);

            if inside
                .iter()
                .zip(outside.iter())
                .any(|(a, b)| b.is_some() && a == b)
            {
                return false;
            }
        }

        // east is +x
        // uses the transposed ids
        if sq.x < self.size - sq.size.get() {
            let inside_idx = ((sq.x + sq.size.get() - 1) * self.size + sq.y) as usize;
            let inside = &self.ids_transposed[inside_idx..inside_idx + sq.size.get() as usize];
            let outside_idx = ((sq.x + sq.size.get()) * self.size + sq.y) as usize;
            let outside = &self.ids_transposed[outside_idx..outside_idx + sq.size.get() as usize];

            #[cfg(debug_assertions)]
            println!("EAST inside: {:?}, outside: {:?}", inside, outside);

            if inside
                .iter()
                .zip(outside.iter())
                .any(|(a, b)| b.is_some() && a == b)
            {
                return false;
            }
        }

        // west is -x
        // uses the transposed ids
        if sq.x > 0 {
            let inside_idx = (sq.x * self.size + sq.y) as usize;
            let inside = &self.ids_transposed[inside_idx..inside_idx + sq.size.get() as usize];
            let outside_idx = ((sq.x - 1) * self.size + sq.y) as usize;
            let outside = &self.ids_transposed[outside_idx..outside_idx + sq.size.get() as usize];

            #[cfg(debug_assertions)]
            println!("WEST inside: {:?}, outside: {:?}", inside, outside);

            if inside
                .iter()
                .zip(outside.iter())
                .any(|(a, b)| b.is_some() && a == b)
            {
                return false;
            }
        }

        true
    }

    /// DFS algorithm to fund the largest square containing (x, y) that can be merged into a bigger pumpkin
    pub fn add(&mut self, x: u16, y: u16) -> Square {
        debug_assert!(!self.contains(x, y));
        self.bitmap.set(self.index(x, y), true);

        let start = Square::new(x, y, 1);
        let mut largest_square = start;

        let sz = self.size as usize;
        let mut visited = BitVec::from_elem(sz * sz * sz, false);
        visited.set(start.idx(sz), true);
        let mut stack = vec![start];

        while let Some(square) = stack.pop() {
            debug_assert_eq!(
                self.lookup_table.get_bitmap(square),
                square.bitmap(self.size)
            );

            if !self.lookup_table.get_bitmap(square).and(&self.bitmap) {
                let neighbors: Vec<Square> = self
                    .lookup_table
                    .get_larger(square)
                    .iter()
                    .filter(|sq| !visited.get(sq.idx(sz)).unwrap())
                    .cloned()
                    .collect();

                for sq in &neighbors {
                    visited.set(sq.idx(sz), true);
                }
                stack.extend(neighbors);

                if square.size > largest_square.size && self.check_boundary(&square) {
                    largest_square = square;
                }
            }
        }

        // Fill the bitmap and ids with the new square
        let id = NonZeroU16::new(largest_square.y * self.size + largest_square.x + 1);
        for y in largest_square.y..largest_square.y + largest_square.size.get() {
            for x in largest_square.x..largest_square.x + largest_square.size.get() {
                let idx = (y * self.size + x) as usize;
                let idx_t = (x * self.size + y) as usize;
                self.ids[idx] = id;
                self.ids_transposed[idx_t] = id;
            }
        }

        largest_square
    }
}

impl std::fmt::Display for PumpkinPatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Print the ids in a grid, but reverse the order of the y direction
        for y in (0..self.size).rev() {
            for x in 0..self.size {
                let id = self.ids[self.index(x, y)].map_or(0, |id| id.get());
                write!(f, "{:3} ", id)?;
            }
            writeln!(f)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::seq::SliceRandom;

    #[test]
    fn test_bit_sq() {
        let bit_sq = Square::new(0, 0, 3);
        // (1, 1) and (2, 2) should be contained
        assert!(bit_sq.contains(1, 1), "bit_sq should contain (1, 1)");
        assert!(bit_sq.contains(2, 2), "bit_sq should contain (2, 2)");
        // (0, 3) should not be contained
        assert!(!bit_sq.contains(0, 3), "bit_sq should not contain (0, 3)");
        // (2, 1)
        assert!(bit_sq.contains(2, 1), "bit_sq should contain (2, 1)");

        // Try another square
        let bit_sq = Square::new(5, 5, 5);
        // (5, 5) and (9, 9) should be contained
        assert!(bit_sq.contains(5, 5), "bit_sq should contain (5, 5)");
        assert!(bit_sq.contains(9, 9), "bit_sq should contain (9, 9)");
    }

    #[test]
    fn test_pumpkins_merge_2() {
        let mut pumpkins = PumpkinPatch::new_make_table(2);
        let order: &[(u16, u16)] = &[(0, 0), (0, 1), (1, 0), (1, 1)];

        let mut iter = order.iter().cloned();

        for _ in 0..3 {
            let (x, y) = iter.next().unwrap();
            let sq = pumpkins.add(x, y);
            println!("{}", pumpkins);
            assert!(sq.size.get() == 1, "Square should be size 1");
            assert!(
                (sq.x, sq.y) == (x, y),
                "Square should be at the correct position"
            );
        }

        // the last one should merge with the first 3
        let sq = pumpkins.add(1, 1);
        println!("{}", pumpkins);
        assert!(sq.size.get() == 2, "Square should be size 2");
        assert!(
            (sq.x, sq.y) == (0, 0),
            "Square should be at the correct position"
        );
    }

    #[test]
    fn test_merge_3() {
        let mut pumpkins = PumpkinPatch::new_make_table(3);
        // make this kind of pattern
        // # # 0
        // # # #
        // 0 # #
        let order: &[(u16, u16)] = &[
            (2, 2),
            (2, 1),
            (1, 2),
            (1, 1),
            (1, 0),
            (0, 1),
            (0, 0),
            (2, 0), // add the 0's to make a 3x3
            (0, 2),
        ];

        let sqs = &[
            (2, 2, 1),
            (2, 1, 1),
            (1, 2, 1),
            (1, 1, 2),
            (1, 0, 1),
            (0, 1, 1),
            (0, 0, 1),
            (2, 0, 1),
            (0, 0, 3),
        ];

        for ((x, y), (e_x, e_y, e_size)) in order.iter().zip(sqs.iter()) {
            let sq = pumpkins.add(*x, *y);
            println!("{}", pumpkins);
            assert!(
                sq.size.get() == *e_size,
                "Square should be size {}, got {}",
                e_size,
                sq.size
            );
            assert!(
                (sq.x, sq.y) == (*e_x, *e_y),
                "Got {:?}, expected {:?}",
                (sq.x, sq.y),
                (e_x, e_y)
            );
        }
    }

    #[test]
    fn test_fill() {
        // Filling any size grid should return a single square
        for size in 2..=10 {
            let mut pumpkins = PumpkinPatch::new_make_table(size);

            let mut rng = rand::thread_rng();
            let mut order = (0..size * size).collect::<Vec<_>>();
            order.shuffle(&mut rng);

            for idx in order.iter().take(order.len() - 1) {
                let x = idx % size;
                let y = idx / size;
                pumpkins.add(x, y);
            }

            let x = order.last().unwrap() % size;
            let y = order.last().unwrap() / size;
            assert!(
                pumpkins.add(x, y).size.get() == size,
                "Square should be size {}",
                size
            );
        }
    }

    #[test]
    fn idx() {
        // checks that the idx and reverse idx are correct
        for size in 1..=10 {
            for x in 0..size {
                for y in 0..size {
                    let square = Square::new(x, y, size);
                    let idx = square.idx(10);
                    let sq = Square::from_index(idx, 10);

                    assert_eq!(square, sq)
                }
            }
        }
    }
}
