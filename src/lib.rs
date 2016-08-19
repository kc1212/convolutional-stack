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
    let n = gs.len();
    let m = gs[0].len() - 1;
    let l = obs.len() / n - m;
    println!("n {}, m {}, l {}", n, m, l);

    stack.push(CodePath { path: Vec::new(), mu: f64::NEG_INFINITY });
    loop {
        let last = stack.pop().unwrap();
        if last.path.len() >= m + l {
            return last.path;
        }
        let paths = last.extend(n);
        for mut path in paths {
            path.fano(obs, gs, p, r);
            stack.push(path);
        }
        stack.sort_by(|a, b| a.mu.partial_cmp(&b.mu).unwrap()); // we shouldn't see NaN here so ok to unwrap
        println!("stack {:?}", stack);
    }
}

/// A path in the tree
#[derive(Clone, Debug)]
struct CodePath {
    path: Vec<u8>,
    mu: f64,
}

impl CodePath {
    /// Consumes myself and create new branches
    fn extend(mut self, n: usize) -> Vec<CodePath> {
        let mut v = Vec::new();
        if self.path.len() < n {
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
fn test_decode() {
    let obs = vec![0,0,1,0,0,1,0,1,1,1,0,1];
    let gs = vec![vec![1, 1, 1], vec![1, 1, 0], vec![1, 0, 1]];
    let p = 1f64/16f64;
    let r = 1f64/3f64;
    assert_eq!(vec![1,1,0,0], decode(&obs, &gs, p, r));
}

#[test]
fn test_fano() {
    let obs = vec![0,0,1,0,0,1,0,1,1,1,0,1];
    let gs = vec![vec![1, 1, 1], vec![1, 1, 0], vec![1, 0, 1]];
    let p = 1f64/16f64;
    let r = 1f64/3f64;
    let mut path = CodePath { path: vec![0, 0, 0, 0], mu: 0f64 };
    // TODO fix
    assert_eq!(-16.6, path.fano(&obs, &gs, p, r));
}
