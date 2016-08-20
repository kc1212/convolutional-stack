#![crate_type = "lib"]
#![crate_name = "convolutional_code"]

extern crate rand;

use std::io::Error;
use std::f64;
use rand::random;

fn encode_inner(xs: &Vec<u8>, gs: &Vec<Vec<u8>>) -> Vec<u8> {
    let mut c: Vec<u8> = Vec::new();
    for (i, _) in xs.iter().enumerate() {
        for g in gs {
            let mut sum = 0;
            for (j, coeff) in g.iter().enumerate() {
                sum ^= coeff * getx(&xs, i, j);
            }
            c.push(sum);
        }
    }
    c
}

pub fn encode(xs: &Vec<u8>, gs: &Vec<Vec<u8>>) -> Vec<u8> {
    // make a copy and add M number of zeros
    let mut xs = xs.clone();
    let m = gs.len() - 1;
    for _ in 0..m {
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

fn decode_inner(obs: &Vec<u8>, gs: &Vec<Vec<u8>>, p: f64, r: f64) -> Vec<u8> {
    let mut stack = Vec::new();
    let n = gs.len();
    let m = gs[0].len() - 1;
    let l = obs.len() / n - m;
    println!("n {}, m {}, l {}", n, m, l);

    stack.push(CodePath { path: Vec::new(), mu: f64::NEG_INFINITY });
    loop {
        let last = stack.pop().unwrap();
        if last.path.len() >= m + l {
            println!("path {:?}, mu {:?}", last.path, last.mu);
            return last.path;
        }
        let paths = last.extend(l);
        for mut path in paths {
            path.fano(obs, gs, p, r);
            stack.push(path);
        }
        stack.sort_by(|a, b| a.mu.partial_cmp(&b.mu).unwrap()); // we shouldn't see NaN here so ok to unwrap
        println!("stack {:?}", stack);
    }
}

pub fn decode(obs: &Vec<u8>, gs: &Vec<Vec<u8>>, p: f64, r: f64) -> Vec<u8> {
    let mut ys = decode_inner(obs, gs, p, r);
    // drop the final M zeros
    let m = gs.len() - 1;
    for _ in 0..m {
        ys.pop().unwrap(); // unwrwap shouldn't fail if pre_process is correct
    }
    ys
}

/// A path in the tree
#[derive(Clone, Debug)]
struct CodePath {
    path: Vec<u8>,
    mu: f64,
}

impl CodePath {
    /// Consumes myself and create new branches
    fn extend(mut self, l: usize) -> Vec<CodePath> {
        let mut v = Vec::new();
        if self.path.len() < l {
            let mut p1 = self;
            let mut p2 = p1.clone();
            p1.path.push(0);
            p2.path.push(1);
            v.push(p1);
            v.push(p2);
        } else {
            self.path.push(0);
            v.push(self);
        }
        v
    }

    fn fano(&mut self, ys: &Vec<u8>, gs: &Vec<Vec<u8>>, p: f64, r: f64) -> f64 {
        let xs = encode_inner(&self.path, gs);
        let py = 0.5f64;
        // let n = xs.len() as f64;
        let mut res = 0f64;
        // println!("xs: {:?}, res: {}", xs, res);
        for (x, y) in xs.iter().zip(ys.iter()) {
            if x == y {
                res += ((1f64 - p) / py).log2();
            } else {
                res += (p / py).log2();
            }
        }
        self.mu = res - (xs.len() as f64) * r;
        self.mu
    }
}

fn deserialise(input: &str) -> Result<Vec<u8>, Error> {
    use std::io::ErrorKind::InvalidData;
    let mut res = Vec::new();
    for s in input.chars() {
        match s {
            '0' => res.push(0),
            '1' => res.push(1),
            _   => return Err(Error::new(InvalidData, "Must be '0' or '1'")),
        }
    }
    Ok(res)
}

fn add_noise(xs: Vec<u8>, p: f64) -> Vec<u8> {
    use std::u32;
    assert!(p > 0f64 && p < 1f64);
    let scaled_p = (p * u32::MAX as f64) as u32; // better to compute using Rational
    xs.into_iter().map(|mut y| {
        if random::<u32>() < scaled_p {
            y = 1 - y;
        }
        y
    }).collect()
}

fn f64_eq(a: f64, b: f64, eps: f64) -> bool {
    let abs_difference = (a - b).abs();
    if abs_difference < eps {
        return true;
    }
    false
}

#[test]
fn test_encode1() {
    let xs = vec![1, 0, 1, 1];
    let gs1 = vec![vec![1, 1, 1], vec![1, 0, 1]];
    assert_eq!(encode_inner(&xs, &gs1), vec![1, 1, 1, 0, 0, 0, 0, 1]);

    let gs2 = vec![vec![1, 1, 1], vec![1, 1, 0]];
    assert_eq!(encode_inner(&xs, &gs2), vec![1, 1, 1, 1, 0, 1, 0, 0]);
}

#[test]
fn test_encode2() {
    let gs = vec![vec![1, 1, 1], vec![1, 1, 0], vec![1, 0, 1]];
    let xs1 = vec![1, 1, 1, 0];
    assert_eq!(encode(&xs1, &gs), vec![1, 1, 1, 0, 0, 1, 1, 0, 0, 0, 1, 1, 1, 0, 1, 0, 0, 0]);

    let xs2 = vec![1, 0, 1, 0];
    assert_eq!(encode(&xs2, &gs), vec![1, 1, 1, 1, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 1, 0, 0, 0]);
}

#[test]
fn test_decode() {
    let obs = vec![0,0,1,0,0,1,0,1,1,1,0,1];
    let gs = vec![vec![1, 1, 1], vec![1, 1, 0], vec![1, 0, 1]];
    let p = 1f64/16f64;
    let r = 1f64/3f64;
    assert_eq!(vec![1,1], decode(&obs, &gs, p, r));
}

#[test]
fn test_noise() {
    let cnt = 1000000;
    let p = 0.1;
    let len = add_noise(vec![0; cnt], 0.1)
        .into_iter()
        .filter(|&x| x == 1 )
        .collect::<Vec<u8>>()
        .len();
    assert!(f64_eq(p, len as f64 / cnt as f64, 1e-3))
}

#[test]
fn test_fano() {
    let obs = vec![0,0,1,0,0,1,0,1,1,1,0,1];
    let gs = vec![vec![1, 1, 1], vec![1, 1, 0], vec![1, 0, 1]];
    let p = 1f64/16f64;
    let r = 1f64/3f64;
    let mut path = CodePath { path: vec![0, 0, 0, 0], mu: 0f64 };
    assert!(f64_eq(-16.55865642634889, path.fano(&obs, &gs, p, r), 1e-6));
}

#[test]
fn test_system() {
    // TODO randomise these
    let orig = vec![0,1,0,1];
    let gs = vec![vec![1, 1, 1], vec![1, 1, 0], vec![1, 0, 1]];
    let p = 1f64/16f64;
    let r = 1f64/(gs.len() as f64);

    let ys = encode(&orig, &gs);
    println!("ys {:?}", ys);
    let xs = decode(&ys, &gs, p, r);
    assert_eq!(orig, xs);
}
