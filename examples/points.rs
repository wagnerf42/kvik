extern crate rand;
use rand::Rng;

#[derive(Debug, PartialEq)]
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

#[cfg(feature = "logs")]
fn main() {
    use rayon::prelude::*;
    let pool = rayon_logs::ThreadPoolBuilder::new()
        .build()
        .expect("failed creating pool");

    let points = create_random_points(10);

    let (_, log) = pool.logging_install(|| {
        let iter = points.par_iter().enumerate().filter_map(|(i, a)| {
            let inner_iter = points[i + 1..].par_iter().map(|b| a.distance_to(b));
            rayon_logs::Logged::new(inner_iter).min_by(|x, y| x.partial_cmp(y).unwrap())
        });
        let min = rayon_logs::Logged::new(iter)
            .min_by(|x, y| x.partial_cmp(y).unwrap())
            .unwrap();

        println!("Closest points have a distance of {}", min);
    });

    log.save_svg("points.svg")
        .expect("failed saving execution trace");
}

#[cfg(not(feature = "logs"))]
fn main() {
    println!("you should run me with the logs feature");
}
