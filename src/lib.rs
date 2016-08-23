#![crate_type = "lib"]
#![crate_name = "convolutional_code"]
#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]


extern crate serde;
extern crate serde_json;
extern crate rand;

use std::io::{Error, ErrorKind};
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use rand::random;

#[derive(Deserialize)]
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

        // check generator and align
        if self.gs.len() <= 0 {
            return Err(Error::new(ErrorKind::InvalidInput, "No generators"));
        }

        let mut max_len = 0;
        for g in &self.gs {
            if g.len() > max_len {
                max_len = g.len();
            }
        }

        if max_len <= 0 {
            return Err(Error::new(ErrorKind::InvalidInput, "At least one of the generator is empty"));
        }

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

#[derive(Serialize)]
pub struct Output {
    pub encoded: Vec<u8>,
    pub observed: Vec<u8>,
    pub decoded: Vec<u8>,
    // paths: Vec<CodePath>,
}

pub struct Gens {
    gs: Vec<Vec<u8>>,
    m: usize,
    n: usize,
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

// does not add M zeros
fn encode_inner(xs: &Vec<u8>, gs: &Gens) -> Vec<u8> {
    let mut c: Vec<u8> = Vec::new(); // TODO with_capacity
    for (i, _) in xs.iter().enumerate() {
        c.extend_from_slice(&encode_step(xs, gs, i))
    }
    c
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

pub fn encode(xs: &Vec<u8>, gs: &Gens) -> Vec<u8> {
    // make a copy and add M number of zeros
    let mut xs = xs.clone();
    for _ in 0..gs.m {
        xs.push(0);
    }
    encode_inner(&xs, gs)
}

/// Returns xs[i - j] when possible otherwise 0
fn getx(xs: &Vec<u8>, i: usize, j: usize) -> u8 {
    if j > i {
        return 0;
    }
    xs[i - j]
}

fn decode_inner(obs: &Vec<u8>, gs: &Gens, p: f64) -> CodePath {
    let mut heap = BinaryHeap::new();
    let l = obs.len() / gs.n - gs.m;
    // println!("n {}, m {}, l {}", n, m, l);

    heap.push(CodePath { path: Vec::new(), mus: Vec::new() , code: Vec::new() });
    loop {
        let last = heap.pop().unwrap();
        if last.path.len() >= gs.m + l {
            // println!("path {:?}, mu {:?}", last.path, last.mu);
            return last;
        }

        let paths = last.extend(l, obs, gs, p);
        for path in paths {
            heap.push(path);
        }
        // println!("stack {:?}", heap);
    }
}

pub fn decode(obs: &Vec<u8>, gs: &Gens, p: f64) -> Vec<u8> {
    let mut ys = decode_inner(obs, gs, p).path;
    // drop the final M zeros
    for _ in 0..gs.m {
        ys.pop().unwrap(); // unwrwap shouldn't fail if pre_process is correct
    }
    ys
}

/// A path in the tree
#[derive(Clone, Debug, Serialize)]
struct CodePath {
    path: Vec<u8>,
    mus: Vec<f64>,
    code: Vec<Vec<u8>>,
}

impl PartialEq for CodePath {
    fn eq(&self, other: &CodePath) -> bool {
        f64_eq(&self.mu(), &other.mu(), &1e-6)
    }
}

impl Eq for CodePath {}

impl PartialOrd for CodePath {
    fn partial_cmp(&self, other: &CodePath) -> Option<Ordering> {
        self.mu().partial_cmp(&other.mu())
    }
}

/// Implementation for Ord is required BinaryHeap
impl Ord for CodePath {
    fn cmp(&self, other: &CodePath) -> Ordering {
        self.mu().partial_cmp(&other.mu()).unwrap()
    }
}

impl CodePath {
    fn mu(&self) -> f64 {
        if self.mus.is_empty() {
            0f64
        } else {
            *self.mus.last().unwrap()
        }
    }

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
    fn fano(&mut self, x: u8, ys: &Vec<u8>, gs: &Gens, p: f64) -> f64 {
        assert!(p > 0f64 && p < 1f64);
        assert!(x == 0 || x == 1);
        assert_eq!(self.path.len(), self.code.len());

        self.path.push(x);
        let py = 0.5f64;
        let r = 1f64 / gs.n as f64;
        let _idx = self.path.len() - 1;
        let _xs = encode_step(&self.path, gs, _idx);
        let _ys = &ys[_idx*gs.n .. (_idx+1)*gs.n];
        println!("path - {:?}, xs - {:?}, ys - {:?}", self.path, _xs, _ys);

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
        mu = self.mu() + mu;
        self.mus.push(mu);

        // update code
        self.code.push(_xs);
        mu
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
    assert_eq!(encode_inner(&xs, &gs1), vec![1, 1, 1, 0, 0, 0, 0, 1]);

    let gs2 = Gens::new(vec![vec![1, 1, 1], vec![1, 1, 0]]);
    assert_eq!(encode_inner(&xs, &gs2), vec![1, 1, 1, 1, 0, 1, 0, 0]);
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
fn test_decode() {
    let obs = vec![0,0,1,0,0,1,0,1,1,1,0,1];
    let gs = Gens::new(vec![vec![1, 1, 1], vec![1, 1, 0], vec![1, 0, 1]]);
    let p = 1f64/16f64;
    assert_eq!(vec![1,1], decode(&obs, &gs, p));
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

/*
#[test]
fn test_fano() {
    let obs = vec![0,0,1,0,0,1,0,1,1,1,0,1];
    let gs = Gens::new(vec![vec![1, 1, 1], vec![1, 1, 0], vec![1, 0, 1]]);
    let p = 1f64/16f64;
    let mut path = CodePath { path: vec![0, 0, 0, 0], mus: vec![0f64], code: vec![] };
    assert!(f64_eq(&-16.55865642634889, &path.fano(&obs, &gs, p), &1e-6));
}
*/

#[test]
fn test_system() {
    // TODO randomise these
    let orig = vec![0,1,0,1];
    let gs = Gens::new(vec![vec![1, 1, 1], vec![1, 1, 0], vec![1, 0, 1]]);
    let p = 1f64/10f64;
    // let r = 1f64/gs.n as f64;

    let ys = create_noise(&encode(&orig, &gs), p);
    println!("ys {:?}", ys);
    let xs = decode(&ys, &gs, p);
    assert_eq!(orig, xs);
}
