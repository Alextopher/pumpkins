use rand::seq::SliceRandom;

/// Bit 0 is (0, 0), bit 1 is (0, 1) etc
use std::num::NonZeroU8;

type LookupTable<const N: usize> = Vec<Vec<Square<N>>>;

/// const parameter N is the Farm Size. For example N = 10 for a 10x10 farm
/// In TFWR (x, y) would be the _bottom right_ corner of the square
#[derive(Debug, Clone, Copy)]
pub struct Square<const N: usize> {
    bitmask: u128,
    x: usize,
    y: usize,
    size: usize,
}

impl<const N: usize> Square<N> {
    fn new(x: usize, y: usize, size: usize) -> Self {
        assert!(x + size <= N && y + size <= N);

        let mut bitmask = 0;
        let row_mask = ((1 << size) - 1) << x;
        // remember that N is the row size
        for i in 0..size {
            bitmask |= row_mask << ((y + i) * N);
        }

        Self {
            bitmask,
            x,
            y,
            size,
        }
    }

    fn contains(&self, x: usize, y: usize) -> bool {
        self.bitmask & (1 << (x + y * N)) != 0
    }
}

// Returns all of the squares that can be made in a NxN grid.
fn all_squares<const N: usize>() -> Vec<Square<N>> {
    let mut squares = Vec::with_capacity(N * (N + 1) * (2 * N + 1) / 6);
    for x in 0..N {
        for y in 0..N {
            for l in 2..=N {
                if x + l <= N && y + l <= N {
                    squares.push(Square::new(x, y, l));
                }
            }
        }
    }
    squares
}

fn precompute_squares<const N: usize>() -> LookupTable<N> {
    let all_squares = all_squares::<N>();
    let mut table = Vec::with_capacity(N * N);

    for y in 0..N {
        for x in 0..N {
            table.push(
                all_squares
                    .iter()
                    .filter(|sq| sq.contains(x, y))
                    .copied()
                    .collect(),
            );
        }
    }

    table
}

pub struct Pumpkins<const N: usize> {
    // Option<NonZeroU8> means that '0' is reserved for empty cells
    ids: Vec<Option<NonZeroU8>>,
    ids_transposed: Vec<Option<NonZeroU8>>,
    bitmask: u128,
    size: usize,

    // Precomputed squares that could intersect with a new pumpkin (size >= 2)
    squares_lookup_table: LookupTable<N>,
}

impl<const N: usize> Default for Pumpkins<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Pumpkins<N> {
    pub fn new() -> Self {
        assert!(N * N <= 128, "size is too large");

        Self {
            ids: vec![None; N * N],
            ids_transposed: vec![None; N * N],
            bitmask: 0,
            size: N,
            squares_lookup_table: precompute_squares::<N>(),
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Option<NonZeroU8> {
        self.ids[y * self.size + x]
    }

    pub fn contains(&self, x: usize, y: usize) -> bool {
        self.bitmask & (1 << (y * self.size + x)) != 0
    }

    /// Checks if any pumpkins cross the boundary into this square
    ///
    /// 4 is both on within the square and outside it
    ///
    ///   10 11 12 4 4 13 14
    /// | ------------------ |
    /// |  1  2  3 4 4  5  6 |
    /// |  . . . . . . . . . |
    fn check_boundary(&self, sq: &Square<N>) -> bool {
        #[cfg(debug_assertions)]
        println!("Checking boundary for {:?}", sq);

        // north is +y
        if sq.y < N - sq.size {
            let inside_idx = (sq.y + sq.size - 1) * N + sq.x;
            let inside = &self.ids[inside_idx..inside_idx + sq.size];
            let outside_idx = (sq.y + sq.size) * N + sq.x;
            let outside = &self.ids[outside_idx..outside_idx + sq.size];

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
            let inside_idx = sq.y * N + sq.x;
            let inside = &self.ids[inside_idx..inside_idx + sq.size];
            let outside_idx = (sq.y - 1) * N + sq.x;
            let outside = &self.ids[outside_idx..outside_idx + sq.size];

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
        if sq.x < N - sq.size {
            let inside_idx = (sq.x + sq.size - 1) * N + sq.y;
            let inside = &self.ids_transposed[inside_idx..inside_idx + sq.size];
            let outside_idx = (sq.x + sq.size) * N + sq.y;
            let outside = &self.ids_transposed[outside_idx..outside_idx + sq.size];

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
            let inside_idx = sq.x * N + sq.y;
            let inside = &self.ids_transposed[inside_idx..inside_idx + sq.size];
            let outside_idx = (sq.x - 1) * N + sq.y;
            let outside = &self.ids_transposed[outside_idx..outside_idx + sq.size];

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

    /// Adds a new pumpkin to the patch, merging it as required
    ///
    /// 1. Gets all possible squares that contain the new pumpkin
    /// 2. Filters out squares that aren't full of pumpkins
    pub fn add(&mut self, x: usize, y: usize) -> Square<N> {
        debug_assert!(
            !self.contains(x, y),
            "A pumpkin can't be planted where one already exists"
        );

        self.bitmask |= 1 << (y * N + x);
        #[cfg(debug_assertions)]
        {
            println!("Bitmask {:?}", self.bitmask);
            println!("Squares {:?}", self.squares_lookup_table[y * N + x]);
        }

        if let Some(merge) = self.squares_lookup_table[y * N + x]
            .iter()
            .filter(|sq| sq.bitmask & self.bitmask == sq.bitmask)
            .filter(|sq| self.check_boundary(sq))
            // todo: sort the lookup table by size
            // .next()
            .max_by_key(|sq| sq.size)
        {
            // pumpkins in the merged square need their ids replaced with the bottom-right
            let id = NonZeroU8::new((merge.y * N + merge.x + 1) as u8);
            debug_assert!(id.is_some());
            for y in merge.y..merge.y + merge.size {
                for x in merge.x..merge.x + merge.size {
                    self.ids[y * N + x] = id;
                    self.ids_transposed[x * N + y] = id;
                }
            }
            return *merge;
        }

        let id = NonZeroU8::new((y * N + x + 1) as u8);
        debug_assert!(id.is_some());
        self.ids[y * N + x] = id;
        self.ids_transposed[x * N + y] = id;

        Square::new(x, y, 1)
    }
}

impl<const N: usize> std::fmt::Display for Pumpkins<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // print the ids with the None as a dot
        let mut ids = self
            .ids
            .iter()
            .map(|id| id.map(|id| id.get().to_string()).unwrap_or(".".to_string()));

        for _ in 0..N {
            for _ in 0..N {
                write!(f, "{:2} ", ids.next().unwrap())?;
            }
            writeln!(f)?;
        }

        Ok(())
    }
}

fn main() {
    const SIZE: usize = 8;
    let mut pumpkins: Pumpkins<SIZE> = Pumpkins::default();

    let mut order = (0..SIZE * SIZE).collect::<Vec<_>>();
    order.shuffle(&mut rand::thread_rng());

    for idx in order {
        let (x, y) = (idx % SIZE, idx / SIZE);
        println!("{:?}", pumpkins.add(x, y));
        print!("{}", pumpkins);
        // wait for user input
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_sq() {
        let bit_sq = Square::<10>::new(0, 0, 3);
        // (1, 1) and (2, 2) should be contained
        assert!(bit_sq.contains(1, 1), "bit_sq should contain (1, 1)");
        assert!(bit_sq.contains(2, 2), "bit_sq should contain (2, 2)");
        // (0, 3) should not be contained
        assert!(!bit_sq.contains(0, 3), "bit_sq should not contain (0, 3)");
        // (2, 1)
        assert!(bit_sq.contains(2, 1), "bit_sq should contain (2, 1)");

        // Try another square
        let bit_sq = Square::<10>::new(5, 5, 5);
        // (5, 5) and (9, 9) should be contained
        assert!(bit_sq.contains(5, 5), "bit_sq should contain (5, 5)");
        assert!(bit_sq.contains(9, 9), "bit_sq should contain (9, 9)");
    }

    #[test]
    fn test_pumpkins_merge_2() {
        let mut pumpkins = Pumpkins::<2>::new();
        let order: &[(usize, usize)] = &[(0, 0), (0, 1), (1, 0), (1, 1)];

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
        let mut pumpkins = Pumpkins::<3>::new();
        // make this kind of pattern
        // # # 0
        // # # #
        // 0 # #
        let order: &[(usize, usize)] = &[
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
}
