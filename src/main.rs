extern crate convolutional_code;
extern crate serde_json;

use std::io::{self};
use serde_json as json;
use convolutional_code as cc;

/*
fn err_and_exit(e: json::error::Error) {
    writeln!(&mut io::stderr(), "{}", e);
    ::std::process::exit(-1)
}
*/

fn main() {
    // TODO make it safer
    let mut inp: cc::Input = json::de::from_reader(io::stdin()).unwrap();
    inp.validate().unwrap();
    let gs = cc::Gens::new(inp.gs);
    let ys = cc::encode(&inp.xs, &gs);
    let noisy_ys = cc::create_noise(&ys, inp.p);
    let (path, paths) = cc::decode_(&noisy_ys, &gs, inp.p);
    let output = cc::Progress{ encoded: ys,
                             observed: noisy_ys,
                             decoded: path.path,
                             paths: paths };
    json::ser::to_writer(&mut io::stdout(), &output).unwrap();
}
