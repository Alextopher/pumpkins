use std::{
    collections::{HashMap, HashSet},
    num::NonZeroU16,
    rc::Rc,
};

use bit_vec::BitVec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Square {
    x: u16,
    y: u16,
    size: u16,
}

impl Square {
    fn new(x: u16, y: u16, size: u16) -> Self {
        Self { x, y, size }
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
        self.x <= x && x < self.x + self.size && self.y <= y && y < self.y + self.size
    }

    /// There are at most size ** 2 new squares that encompass the current square
    ///
    /// They all have top-left less than or equal to (in both axes) the current square's top-left
    fn next_larger_squares(&self, grid_size: u16) -> Vec<Square> {
        let mut squares = Vec::new();
        if self.size == grid_size {
            return squares;
        }

        let new_size = self.size + 1;

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
}

/// A lookup table for the 'next larger squares' operation
#[derive(Debug)]
pub struct LookupTable {
    index: HashMap<Square, (usize, usize)>,
    squares: Vec<Square>,

    // Lookup table for Square bitmaps
    bitmaps: HashMap<Square, BitVec>,
}

impl LookupTable {
    pub fn new(size: u16) -> Self {
        let mut index = HashMap::new();
        let mut squares = Vec::new();
        let mut bitmaps = HashMap::new();

        for x in 0..size {
            for y in 0..size {
                for s in 1..=size {
                    let square = Square::new(x, y, s);
                    let neighbors = square.next_larger_squares(size);
                    let start = squares.len();
                    squares.extend(neighbors);
                    let end = squares.len();

                    index.insert(square, (start, end));

                    bitmaps.insert(square, square.bitmap(size));
                }
            }
        }

        Self {
            index,
            squares,
            bitmaps,
        }
    }

    fn get(&self, square: Square) -> &[Square] {
        let (start, end) = self.index[&square];
        &self.squares[start..end]
    }

    fn get_bitmap(&self, square: Square) -> BitVec {
        self.bitmaps[&square].clone()
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
        if sq.y < self.size - sq.size {
            let inside_idx: usize = ((sq.y + sq.size - 1) * self.size + sq.x) as usize;
            let inside = &self.ids[inside_idx..inside_idx + sq.size as usize];
            let outside_idx: usize = ((sq.y + sq.size) * self.size + sq.x) as usize;
            let outside = &self.ids[outside_idx..outside_idx + sq.size as usize];

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
            let inside = &self.ids[inside_idx..inside_idx + sq.size as usize];
            let outside_idx = ((sq.y - 1) * self.size + sq.x) as usize;
            let outside = &self.ids[outside_idx..outside_idx + sq.size as usize];

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
        if sq.x < self.size - sq.size {
            let inside_idx = ((sq.x + sq.size - 1) * self.size + sq.y) as usize;
            let inside = &self.ids_transposed[inside_idx..inside_idx + sq.size as usize];
            let outside_idx = ((sq.x + sq.size) * self.size + sq.y) as usize;
            let outside = &self.ids_transposed[outside_idx..outside_idx + sq.size as usize];

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
            let inside = &self.ids_transposed[inside_idx..inside_idx + sq.size as usize];
            let outside_idx = ((sq.x - 1) * self.size + sq.y) as usize;
            let outside = &self.ids_transposed[outside_idx..outside_idx + sq.size as usize];

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

        let mut visited = HashSet::from([start]);
        let mut stack = vec![start];

        while let Some(square) = stack.pop() {
            if !self.lookup_table.get_bitmap(square).and(&self.bitmap) {
                let neighbors: Vec<Square> = self
                    .lookup_table
                    .get(square)
                    .iter()
                    .filter(|sq| !visited.contains(sq))
                    .cloned()
                    .collect();

                visited.extend(&neighbors);
                stack.extend(neighbors);

                if square.size > largest_square.size && self.check_boundary(&square) {
                    largest_square = square;
                }
            }
        }

        // Fill the bitmap and ids with the new square
        let id = NonZeroU16::new(largest_square.y * self.size + largest_square.x + 1);
        for y in largest_square.y..largest_square.y + largest_square.size {
            for x in largest_square.x..largest_square.x + largest_square.size {
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
            assert!(sq.size == 1, "Square should be size 1");
            assert!(
                (sq.x, sq.y) == (x, y),
                "Square should be at the correct position"
            );
        }

        // the last one should merge with the first 3
        let sq = pumpkins.add(1, 1);
        println!("{}", pumpkins);
        assert!(sq.size == 2, "Square should be size 2");
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
                sq.size == *e_size,
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
        for size in 1..=10 {
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
                pumpkins.add(x, y).size == size,
                "Square should be size {}",
                size
            );
        }
    }
}
