#![allow(dead_code)]

use std::rc::Rc;

use graph::{LookupTable, PumpkinPatch};
use rand::seq::SliceRandom;

pub mod graph;

fn interactive(size: u16) {
    let start = std::time::Instant::now();
    let elapsed = start.elapsed();
    println!("Built lookup table in {:?}", elapsed);

    let lookup_table = Rc::new(LookupTable::new(size));
    let mut pumpkins = PumpkinPatch::new(size, lookup_table);
    let mut order = (0..size * size).collect::<Vec<_>>();
    order.shuffle(&mut rand::thread_rng());

    for idx in order {
        let (x, y) = (idx % size, idx / size);
        println!(
            "Insert: {} / {:?} | {:?}",
            idx + 1,
            (x, y),
            pumpkins.add(x, y)
        );
        print!("{}", pumpkins);
        // wait for user input
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
    }
}

fn benchmark(size: u16) {
    const N: usize = 5;

    // benchmark - run 1000 random orderings
    let lookup_table = Rc::new(LookupTable::new(size));
    let mut rng = rand::thread_rng();
    let samples = (0..N).map(|_| {
        let mut order = (0..size * size).collect::<Vec<_>>();
        order.shuffle(&mut rng);
        order
    });

    let start = std::time::Instant::now();
    for order in samples {
        let mut pumpkins = PumpkinPatch::new(size, lookup_table.clone());
        for idx in order {
            let (x, y) = (idx % size, idx / size);
            pumpkins.add(x, y);
        }
    }
    let elapsed = start.elapsed();
    println!(
        "Size {}x{} - Elapsed: {:?} - ΔT: {:?}",
        size,
        size,
        elapsed / N as u32,
        elapsed / (N * N * N) as u32
    );
}

fn main() {
    #[cfg(debug_assertions)]
    interactive(20);

    #[cfg(not(debug_assertions))]
    for sz in [10, 20, 30, 40, 50, 60, 70, 80] {
        benchmark(sz)
    }
}
