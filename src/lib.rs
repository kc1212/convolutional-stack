#![crate_type = "lib"]
#![crate_name = "convolutional_code"]

pub fn encode(xs: &Vec<u8>, gss: &Vec<Vec<u8>>) -> Vec<u8> {
    let mut c: Vec<u8> = Vec::new();
    for (i, _) in xs.iter().enumerate() {
        for gs in gss {
            let mut sum = 0;
            for (j, g) in gs.iter().enumerate() {
                let idx = i as i32 - j as i32;
                let mut x = 0;
                if idx >= 0 {
                    x = xs[idx as usize];
                }
                sum ^= g * x;
            }
            c.push(sum);
        }
    }
    c
}

#[test]
fn it_works() {
    let xs = vec![1, 0, 1, 1];
    let g1 = vec![1, 1, 1];
    let g2 = vec![1, 0, 1];
    let gss = vec![g1, g2];
    assert_eq!(encode(&xs, &gss), vec![1, 1, 1, 0, 0, 0, 0, 1]);
}
