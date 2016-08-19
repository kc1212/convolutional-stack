#![crate_type = "lib"]
#![crate_name = "convolutional_code"]

use std::io::Error;
use std::f64;

pub fn encode(xs: &Vec<u8>, gs: &Vec<Vec<u8>>) -> Vec<u8> {
    let mut c: Vec<u8> = Vec::new();
    for (i, _) in xs.iter().enumerate() {
        for g in gs {
            let mut sum = 0;
            for (j, coeff) in g.iter().enumerate() {
                sum ^= coeff * getx(xs, i, j);
            }
            c.push(sum);
        }
    }
    c
}

/// Returns xs[i - j] when possible otherwise 0
fn getx(xs: &Vec<u8>, i: usize, j: usize) -> u8 {
    if j > i {
        return 0;
    }
    xs[i - j]
}

pub fn decode(obs: &Vec<u8>, gs: &Vec<Vec<u8>>, p: f64, r: f64) -> Vec<u8> {
    let mut stack = Vec::new();
    stack.push(CodePath { path: Vec::new(), mu: f64::NEG_INFINITY });
    loop {
        let (mut p1, mut p2) = stack.remove(0).split();
        p1.fano(obs, gs, p, r);
        p2.fano(obs, gs, p, r);
        stack.push(p1);
        stack.push(p2);
        stack.sort_by(|a, b| a.mu.partial_cmp(&b.mu).unwrap()); // we shouldn't see NaN here so ok to unwrap
        // TODO stop at some point...
    }
    stack.remove(0).path
}

/// A path in the tree
#[derive(Clone)]
struct CodePath {
    path: Vec<u8>,
    mu: f64,
}

impl CodePath {
    /// Consumes myself and split to two
    fn split(self) -> (CodePath, CodePath) {
        let mut p1 = self;
        let mut p2 = p1.clone();
        p1.path.push(0);
        p2.path.push(1);
        (p1, p2)
    }

    fn fano(&mut self, ys: &Vec<u8>, gs: &Vec<Vec<u8>>, p: f64, r: f64) -> f64 {
        let xs = encode(&self.path, gs);
        // let n = xs.len() as f64;
        let mut res = 0f64;
        // println!("xs: {:?}, res: {}", xs, res);
        for (x, y) in xs.iter().zip(ys.iter()) {
            if x == y {
                res += (2f64 * (1f64 - p)).log2();
            } else {
                res += (2f64 * p).log2();
            }
        }
        res - (xs.len() as f64) * r
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
    };
    Ok(res)
}

#[test]
fn test_generator() {
    let xs = vec![1, 0, 1, 1];
    let gs1 = vec![vec![1, 1, 1], vec![1, 0, 1]];
    assert_eq!(encode(&xs, &gs1), vec![1, 1, 1, 0, 0, 0, 0, 1]);

    let gs2 = vec![vec![1, 1, 1], vec![1, 1, 0]];
    assert_eq!(encode(&xs, &gs2), vec![1, 1, 1, 1, 0, 1, 0, 0]);
}

#[test]
fn test_source() {
    let gs = vec![vec![1, 1, 1], vec![1, 1, 0], vec![1, 0, 1]];
    let xs1 = vec![1, 1, 1, 0, 0, 0];
    assert_eq!(encode(&xs1, &gs), vec![1, 1, 1, 0, 0, 1, 1, 0, 0, 0, 1, 1, 1, 0, 1, 0, 0, 0]);

    let xs2 = vec![1, 0, 1, 0, 0, 0];
    assert_eq!(encode(&xs2, &gs), vec![1, 1, 1, 1, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 1, 0, 0, 0]);
}

#[test]
fn test_fano() {
    // TODO check
    let obs = vec![0,0,1,0,0,1,0,1,1,1,0,1];
    let gs = vec![vec![1, 1, 1], vec![1, 1, 0], vec![1, 0, 1]];
    let p = 1f64/16f64;
    let r = 1f64/3f64;
    let mut path = CodePath { path: vec![0, 0, 0, 0], mu: 0f64 };
    assert_eq!(-16.6, path.fano(&obs, &gs, p, r));
}
