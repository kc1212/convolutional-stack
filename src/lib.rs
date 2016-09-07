#![crate_type = "lib"]
#![crate_name = "convolutional_stack"]

extern crate rand;

use std::io::{Error, ErrorKind};
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use rand::random;

pub struct Input {
    pub xs: Vec<u8>,
    pub gs: Vec<Vec<u8>>,
    pub p: f64,
}

impl Input {
    pub fn validate(&mut self) -> Result<(), Error> {
        // check input
        if self.xs.len() <= 0 {
            return Err(Error::new(ErrorKind::InvalidInput, "No input"));
        }

        for x in &self.xs {
            if x != &0 && x != &1 {
                return Err(Error::new(ErrorKind::InvalidInput, "Invalid input, must be 0 or 1"));
            }
        }

        // check generator and align
        if self.gs.len() <= 0 {
            return Err(Error::new(ErrorKind::InvalidInput, "No generators"));
        }

        let mut max_len = 0;
        for g in &self.gs {
            if g.len() > max_len {
                max_len = g.len();
            }
            for x in g {
                if x != &0 && x != &1 {
                    return Err(Error::new(ErrorKind::InvalidInput, "Invalid generator(s), must be 0 or 1"));
                }
            }
        }

        if max_len <= 0 {
            return Err(Error::new(ErrorKind::InvalidInput, "At least one of the generator is empty"));
        }

        // pad short generators with zeros
        // TODO consider moving this part to another function and make this function take &self?
        for mut g in &mut self.gs {
            for _ in 0..max_len - g.len() {
                g.push(0);
            }
        }

        // check probability
        if self.p > 1f64 || self.p < 0f64 {
            return Err(Error::new(ErrorKind::InvalidInput, "Probability is invalid"));
        }

        Ok(())
    }
}

/// For tracking the decoding progress and some key data
pub struct Results {
    pub m: usize,
    pub n: usize,
    pub encoded: Vec<u8>,
    pub observed: Vec<u8>,
    pub decoded: Vec<u8>,
    /// The paths in the order which they are evaluated by the algorithm
    pub paths: Vec<CodePath>,
}

pub struct Gens {
    gs: Vec<Vec<u8>>,
    pub m: usize,
    pub n: usize,
}

impl Gens {
    pub fn new(gs: Vec<Vec<u8>>) -> Gens {
        Gens {
            m: gs[0].len() - 1,
            n: gs.len(),
            gs: gs,
        }
    }
}

// not the most efficient way encoding since the returned value must be drained
// but we need this function to perform decoding
fn encode_step(xs: &Vec<u8>, gs: &Gens, i: usize) -> Vec<u8> {
    let mut c: Vec<u8> = Vec::with_capacity(gs.n);
    for g in &gs.gs {
        let mut sum = 0;
        for (j, coeff) in g.iter().enumerate() {
            assert!(coeff == &0 || coeff == &1);
            sum ^= coeff * getx(&xs, i, j);
        }
        c.push(sum);
    }
    c
}

/// Same as `encode`, but without pre-processing
pub fn encode_(xs: &Vec<u8>, gs: &Gens) -> Vec<u8> {
    let mut c: Vec<u8> = Vec::new(); // TODO with_capacity
    for (i, _) in xs.iter().enumerate() {
        c.extend_from_slice(&encode_step(xs, gs, i))
    }
    c
}

/// Perform convolutional encoding
pub fn encode(xs: &Vec<u8>, gs: &Gens) -> Vec<u8> {
    // make a copy and add M number of zeros
    let mut xs = xs.clone();
    for _ in 0..gs.m {
        xs.push(0);
    }
    encode_(&xs, gs)
}

/// Returns xs[i - j] when possible otherwise 0 representing the register
fn getx(xs: &Vec<u8>, i: usize, j: usize) -> u8 {
    if j > i {
        return 0;
    }
    xs[i - j]
}

// TODO provide an option to disable logging
/// Same as `decode` but without post-processing, and returns a CodePath
pub fn decode_(obs: &Vec<u8>, gs: &Gens, p: f64) -> (CodePath, Vec<CodePath>) {
    let mut heap = BinaryHeap::new();
    let l = obs.len() / gs.n - gs.m;
    let mut progress = Vec::new();

    heap.push(CodePath { path: Vec::new(), mu: 0f64 });
    loop {
        let best = heap.pop().unwrap();
        if best.path.len() >= gs.m + l {
            return (best, progress);
        }

        let paths = best.extend(l, obs, gs, p);
        for path in paths {
            progress.push(path.clone());
            heap.push(path);
        }
    }
}

/// Perform decoding using the stack algorithm
pub fn decode(obs: &Vec<u8>, gs: &Gens, p: f64) -> Vec<u8> {
    let mut ys = decode_(obs, gs, p).0.path;
    // drop the final M zeros
    for _ in 0..gs.m {
        ys.pop().unwrap(); // unwrwap shouldn't fail if `encode` is used
    }
    ys
}

/// A path in the tree
#[derive(Clone, Debug)]
pub struct CodePath {
    pub path: Vec<u8>,
    pub mu: f64,
}

impl PartialEq for CodePath {
    fn eq(&self, other: &CodePath) -> bool {
        f64_eq(&self.mu, &other.mu, &1e-6)
    }
}

impl Eq for CodePath {}

impl PartialOrd for CodePath {
    fn partial_cmp(&self, other: &CodePath) -> Option<Ordering> {
        self.mu.partial_cmp(&other.mu)
    }
}

/// Implementation for Ord is required BinaryHeap
impl Ord for CodePath {
    fn cmp(&self, other: &CodePath) -> Ordering {
        self.mu.partial_cmp(&other.mu).unwrap()
    }
}

impl CodePath {
    /// Consumes myself and create new branches,
    /// this function depends on previously computed paths and fano metric.
    fn extend(mut self, l: usize, ys: &Vec<u8>, gs: &Gens, p: f64) -> Vec<CodePath> {
        let mut v = Vec::new();
        if self.path.len() < l {
            let mut p1 = self;
            let mut p2 = p1.clone();
            p1.fano(0, ys, gs, p);
            p2.fano(1, ys, gs, p);
            v.push(p1);
            v.push(p2);
        } else {
            self.fano(0, ys, gs, p);
            v.push(self);
        }
        v
    }

    /// Update the path and the fano metric,
    /// this function depends on previously computed paths and fano metric.
    fn fano(&mut self, x: u8, ys: &Vec<u8>, gs: &Gens, p: f64) {
        assert!(p > 0f64 && p < 1f64);
        assert!(x == 0 || x == 1);

        self.path.push(x);
        let py = 0.5f64;
        let r = 1f64 / gs.n as f64;
        let _idx = self.path.len() - 1;
        let _xs = encode_step(&self.path, gs, _idx);
        let _ys = &ys[_idx*gs.n .. (_idx+1)*gs.n];

        // mu is the fano metric for one iteration
        let mut mu = 0f64;
        for (x, y) in _xs.iter().zip(_ys.iter()) {
            if x == y {
                mu += ((1f64 - p) / py).log2() - r;
            } else {
                mu += (p / py).log2() - r;
            }
        }

        // update mu to be the fano metric for the whole path
        self.mu = self.mu + mu;
    }
}

pub fn create_noise(xs: &[u8], p: f64) -> Vec<u8> {
    use std::u32;
    assert!(p > 0f64 && p < 1f64);
    let scaled_p = (p * u32::MAX as f64) as u32; // better to compute using Rational
    xs.clone().into_iter().map(|&y| {
        if random::<u32>() < scaled_p {
            1 - y
        } else {
            y
        }
    }).collect()
}

fn f64_eq(a: &f64, b: &f64, eps: &f64) -> bool {
    let abs_difference = (a - b).abs();
    if abs_difference < *eps {
        return true;
    }
    false
}

#[test]
fn test_encode1() {
    let xs = vec![1, 0, 1, 1];
    let gs1 = Gens::new(vec![vec![1, 1, 1], vec![1, 0, 1]]);
    assert_eq!(encode_(&xs, &gs1), vec![1, 1, 1, 0, 0, 0, 0, 1]);

    let gs2 = Gens::new(vec![vec![1, 1, 1], vec![1, 1, 0]]);
    assert_eq!(encode_(&xs, &gs2), vec![1, 1, 1, 1, 0, 1, 0, 0]);
}

#[test]
fn test_encode2() {
    let gs = Gens::new(vec![vec![1, 1, 1], vec![1, 1, 0], vec![1, 0, 1]]);
    let xs1 = vec![1, 1, 1, 0];
    assert_eq!(encode(&xs1, &gs), vec![1, 1, 1, 0, 0, 1, 1, 0, 0, 0, 1, 1, 1, 0, 1, 0, 0, 0]);

    let xs2 = vec![1, 0, 1, 0];
    assert_eq!(encode(&xs2, &gs), vec![1, 1, 1, 1, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 1, 0, 0, 0]);
}

#[test]
fn test_decode_and_fano() {
    let obs = vec![0,0,1,0,0,1,0,1,1,1,0,1];
    let gs = Gens::new(vec![vec![1, 1, 1], vec![1, 1, 0], vec![1, 0, 1]]);
    let p = 1f64/16f64;
    assert_eq!(vec![1,1], decode(&obs, &gs, p));

    // using the same params we can test the fano metric too
    let best = decode_(&obs, &gs, p).0;
    // let worst = rest.first().unwrap();
    assert!(f64_eq(&-0.9310940439148156, &best.mu, &1e-6));
    // println!("{}", &worst.mu());
    // assert!(f64_eq(&-16.093109404391484, &worst.mu(), &1e-6));
}

#[test]
fn test_noise() {
    const CNT: usize = 1000000;
    let p = 0.1;
    let len = create_noise(&[0; CNT], 0.1)
        .into_iter()
        .filter(|&x| x == 1 )
        .collect::<Vec<u8>>()
        .len();
    assert!(f64_eq(&p, &(len as f64 / CNT as f64), &1e-3))
}

#[test]
fn test_system() {
    // TODO randomise these
    let orig = vec![0,1,0,1];
    let gs = Gens::new(vec![vec![1, 1, 1], vec![1, 1, 0], vec![1, 0, 1]]);
    let p = 1f64/10f64;

    let ys = create_noise(&encode(&orig, &gs), p);
    // println!("ys {:?}", ys);
    let xs = decode(&ys, &gs, p);
    assert_eq!(orig, xs);
}
