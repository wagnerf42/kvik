#[macro_use]
extern crate criterion;
extern crate rand;
extern crate rayon;
extern crate kvik;

use criterion::{Criterion, ParameterizedBenchmark};
use rand::Rng;
use rayon::{ThreadPool, ThreadPoolBuilder};

#[derive(PartialEq)]
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    pub fn distance_to(&self, other: &Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

fn create_random_points(size: usize) -> Vec<Point> {
    let mut rng = rand::thread_rng();

    std::iter::repeat_with(|| Point::new(rng.gen::<f64>(), rng.gen::<f64>()))
        .take(size)
        .collect::<Vec<Point>>()
}

fn all_composed(points: &Vec<Point>, tp: &ThreadPool) -> f64 {
    use kvik::prelude::*;

    let len = points.len();
    let enumeration = 0..len as u64;

    let threads = rayon::current_num_threads();
    let limit = (((threads as f64).log(2.0).ceil()) as usize) + 1;

    tp.install(|| {
        points
            .par_iter()
            .zip(enumeration)
            .map(|(a, i)| {
                points[(i as usize) + 1..]
                    .par_iter()
                    .map(|b| a.distance_to(b))
                    .rayon(limit)
                    .composed()
                    .min_by(|x, y| x.partial_cmp(y).unwrap())
            })
            .filter(|e| e.is_some())
            .map(|e| e.unwrap())
            .rayon(limit)
            .composed()
            .min_by(|x, y| x.partial_cmp(y).unwrap())
            .unwrap()
    })
}

fn rayon_outer(points: &Vec<Point>, tp: &ThreadPool) -> f64 {
    use kvik::prelude::*;

    let len = points.len();
    let enumeration = 0..len as u64;

    let threads = rayon::current_num_threads();
    let limit = (((threads as f64).log(2.0).ceil()) as usize) + 1;

    tp.install(|| {
        points
            .par_iter()
            .zip(enumeration)
            .map(|(a, i)| {
                points[(i as usize) + 1..]
                    .iter()
                    .map(|b| a.distance_to(b))
                    .min_by(|x, y| x.partial_cmp(y).unwrap())
            })
            .filter(|e| e.is_some())
            .map(|e| e.unwrap())
            .rayon(limit)
            .min_by(|x, y| x.partial_cmp(y).unwrap())
            .unwrap()
    })
}

fn rayon_both(points: &Vec<Point>, tp: &ThreadPool) -> f64 {
    use kvik::prelude::*;

    let len = points.len();
    let enumeration = 0..len as u64;

    let threads = rayon::current_num_threads();
    let limit = (((threads as f64).log(2.0).ceil()) as usize) + 1;

    tp.install(|| {
        points
            .par_iter()
            .zip(enumeration)
            .map(|(a, i)| {
                points[(i as usize) + 1..]
                    .par_iter()
                    .map(|b| a.distance_to(b))
                    .rayon(limit)
                    .min_by(|x, y| x.partial_cmp(y).unwrap())
            })
            .filter(|e| e.is_some())
            .map(|e| e.unwrap())
            .rayon(limit)
            .min_by(|x, y| x.partial_cmp(y).unwrap())
            .unwrap()
    })
}

fn composed_benchmarks(c: &mut Criterion) {
    let sizes: Vec<usize> = vec![100, 250, 500, 750, 1000, 2000, 5000, 10_000];

    c.bench(
        "Points",
        ParameterizedBenchmark::new(
            "composed",
            |b, input_size| {
                b.iter_with_setup(
                    || {
                        let pool = ThreadPoolBuilder::new()
                            .num_threads(4)
                            .build()
                            .expect("Failed to initialize thread pool");
                        (pool, create_random_points(*input_size))
                    },
                    |(pool, i)| all_composed(&i, &pool),
                )
            },
            sizes.clone(),
        )
        .with_function("rayon outer", |b, input_size| {
            b.iter_with_setup(
                || {
                    let pool = ThreadPoolBuilder::new()
                        .num_threads(4)
                        .build()
                        .expect("Failed to initialize thread pool");
                    (pool, create_random_points(*input_size))
                },
                |(pool, i)| rayon_outer(&i, &pool),
            )
        })
        .with_function("rayon + rayon", |b, input_size| {
            b.iter_with_setup(
                || {
                    let pool = ThreadPoolBuilder::new()
                        .num_threads(4)
                        .build()
                        .expect("Failed to initialize thread pool");
                    (pool, create_random_points(*input_size))
                },
                |(pool, i)| rayon_both(&i, &pool),
            )
        }),
    );
}

criterion_group! {
    name = composed;
    config = Criterion::default();
    targets = composed_benchmarks
}

criterion_main!(composed);
