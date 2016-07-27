#![crate_type = "lib"]
#![crate_name = "convolutional_code"]

pub fn encode(xs: &Vec<u8>, gss: &Vec<Vec<u8>>) -> Vec<u8> {
    let mut c: Vec<u8> = Vec::new();
    for (i, _) in xs.iter().enumerate() {
        for gs in gss {
            let mut sum = 0;
            for (j, g) in gs.iter().enumerate() {
                sum ^= g * value_or_0(xs, i, j);
            }
            c.push(sum);
        }
    }
    c
}

fn value_or_0(xs: &Vec<u8>, i: usize, j: usize) -> u8 {
    let idx = i as i32 - j as i32;
    if idx >= 0 {
        return xs[idx as usize];
    }
    0
}

#[test]
fn it_works() {
    let xs = vec![1, 0, 1, 1];
    let gss1 = vec![vec![1, 1, 1], vec![1, 0, 1]];
    assert_eq!(encode(&xs, &gss1), vec![1, 1, 1, 0, 0, 0, 0, 1]);

    let gss2 = vec![vec![1, 1, 1], vec![1, 1, 0]];
    assert_eq!(encode(&xs, &gss2), vec![1, 1, 1, 1, 0, 1, 0, 0]);
}
