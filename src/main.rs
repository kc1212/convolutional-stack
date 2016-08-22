extern crate convolutional_code;
extern crate serde_json;

use std::io::{self, Write};
use serde_json as json;
use convolutional_code as cc;

fn err_and_exit(e: json::error::Error) {
    writeln!(&mut io::stderr(), "{}", e);
    ::std::process::exit(-1)
}

fn main() {
    // TODO make it safer
    let inp: cc::Input = json::de::from_reader(io::stdin()).unwrap();
    let gs = cc::Gens::new(inp.gs);
    let ys = cc::encode(&inp.xs, &gs);
    let noisy_ys = cc::add_noise(ys, inp.p);
    let result = cc::decode(&noisy_ys, &gs, inp.p);
    let output = cc::Output{ code: result };
    json::ser::to_writer(&mut io::stdout(), &output);
}