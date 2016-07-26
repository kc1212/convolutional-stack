#![crate_type = "lib"]
#![crate_name = "convolutional_code"]

pub fn encode_one(xs: &Vec<u8>, gs: &Vec<u8>) -> Vec<u8> {
    let mut c: Vec<u8> = Vec::new();
    for (i, _) in xs.iter().enumerate() {
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
    c
}

#[test]
fn it_works() {
    let xs = vec![1, 0, 1, 1];
    let gs = vec![1, 1, 1];
    assert_eq!(encode_one(&xs, &gs), vec![1, 1, 0, 0]);
}
