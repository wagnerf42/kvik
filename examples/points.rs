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

fn compute_closest(points: &Vec<Point>) -> f64 {
    use rayon::prelude::*;
    points
        .par_iter()
        .enumerate()
        .filter_map(|(i, a)| {
            points[i + 1..]
                .par_iter()
                .map(|b| a.distance_to(b))
                .min_by(|x, y| x.partial_cmp(y).unwrap())
        })
        .min_by(|x, y| x.partial_cmp(y).unwrap())
        .unwrap()
}

#[cfg(feature = "logs")]
fn compute_closest_logged(points: &Vec<Point>) -> f64 {
    use rayon_try_fold::prelude::*;

    let len = points.len();
    let enumeration = 0..len as u64;

    points
        .par_iter()
        .zip(enumeration)
        .map(|(a, i)| {
            rayon_logs::subgraph("inner", len - i as usize, || {
                points[(i as usize) + 1..]
                    .par_iter()
                    .map(|b| a.distance_to(b))
                    .min_by(|x, y| x.partial_cmp(y).unwrap())
            })
        })
        .filter(|e| e.is_some())
        .map(|e| e.unwrap())
        .min_by(|x, y| x.partial_cmp(y).unwrap())
        .unwrap()
}

#[cfg(feature = "logs")]
fn compute_closest_composed(points: &Vec<Point>) -> f64 {
    use rayon_try_fold::prelude::*;

    let len = points.len();
    let enumeration = 0..len as u64;

    points
        .par_iter()
        .zip(enumeration)
        .map(|(a, i)| {
            rayon_logs::subgraph("inner", len - i as usize, || {
                points[(i as usize) + 1..]
                    .par_iter()
                    .map(|b| a.distance_to(b))
                    .composed()
                    .min_by(|x, y| x.partial_cmp(y).unwrap())
            })
        })
        .filter(|e| e.is_some())
        .map(|e| e.unwrap())
        .composed()
        .min_by(|x, y| x.partial_cmp(y).unwrap())
        .unwrap()
}

#[cfg(feature = "logs")]
fn main() {
    let pool = rayon_logs::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .expect("failed creating pool");

    let points = create_random_points(10);

    let expected = compute_closest(&points);
    let (_, log) = pool.logging_install(|| {
        let min = compute_closest_composed(&points);
        assert_eq!(min, expected);
        println!("Closest points have a distance of {}", min);
    });

    log.save_svg("points.svg")
        .expect("failed saving execution trace");
}

#[cfg(not(feature = "logs"))]
fn main() {
    println!("you should run me with the logs feature");
}
