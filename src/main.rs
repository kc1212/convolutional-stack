extern crate convolutional_code;
extern crate serde_json;

use std::io::{self, Write};
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
    let (path, rest) = cc::decode_(&noisy_ys, &gs, inp.p);
    let output = cc::Output{ encoded: ys,
                             observed: noisy_ys,
                             code_path: path,
                             code_path_rest: rest };
    json::ser::to_writer(&mut io::stdout(), &output).unwrap();
}
