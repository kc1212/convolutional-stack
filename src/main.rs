extern crate convolutional_stack;
extern crate gtk;
extern crate cairo;

use std::io::{self, Error, ErrorKind};
use convolutional_stack as cs;
use gtk::{Orientation, Align};
use gtk::prelude::*;
use cairo::Context;

// make pack_start easier for default values
macro_rules! pack_start {
    ( $b:ident => $( $i:ident ),+ ) => {
        $(
            $b.pack_start(&$i, true, true, 0);
        )+
    }
}

// make moving clones into closures more convenient
macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
                move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
                move |$(clone!(@param $p),)+| $body
        }
    );
}

fn stack(xs: String, gs: String, pr: String) -> Result<(), Error>{
    // shadow the input params
    let xs = try!(cs::parse_bin(&xs));
    let gs = try!(cs::parse_gs(&gs));
    let pr = try!(cs::parse_pr(&pr));

    let ys = cs::encode(&xs, &gs);
    let noisy_ys = cs::create_noise(&ys, pr);
    let (path, paths) = cs::decode_(&noisy_ys, &gs, pr);

    let output = cs::Results {
        m: gs.m,
        n: gs.n,
        encoded: ys,
        observed: noisy_ys,
        decoded: path.path,
        paths: paths
    };
    println!("{:?}", output);

    Ok(())
}

fn main() {
    gtk::init().unwrap();

    let window = gtk::Window::new(gtk::WindowType::Toplevel);

    // layout containers
    let box_main = gtk::Box::new(Orientation::Horizontal, 0);
    let box_left = gtk::Box::new(Orientation::Vertical, 0);
    let box_right = gtk::Box::new(Orientation::Vertical, 0);

    // input
    let lbl_xs = gtk::Label::new(Some("Binary input,\nany characters other than '0' or '1' are ignored."));
    let ent_xs = gtk::Entry::new_with_buffer(&gtk::EntryBuffer::new(Some("0101")));
    let sep_xs = gtk::Separator::new(Orientation::Horizontal);
    lbl_xs.set_halign(Align::Start);
    sep_xs.set_valign(Align::Center);

    // generators
    let lbl_gs = gtk::Label::new(Some("Generators,\nseparated by commas."));
    let ent_gs = gtk::Entry::new_with_buffer(&gtk::EntryBuffer::new(Some("101,110")));
    let sep_gs = gtk::Separator::new(Orientation::Horizontal);
    lbl_gs.set_halign(Align::Start);
    sep_gs.set_valign(Align::Center);

    // error probability
    let lbl_pr = gtk::Label::new(Some("Error probability p,\nwhere 0 < p < 1."));
    let ent_pr = gtk::Entry::new_with_buffer(&gtk::EntryBuffer::new(Some("0.1")));
    let sep_pr = gtk::Separator::new(Orientation::Horizontal);
    lbl_pr.set_halign(Align::Start);
    sep_pr.set_valign(Align::Center);

    // transmitted
    let lbl_tx = gtk::Label::new(Some("Transmitted bits"));
    let ent_tx = gtk::Entry::new_with_buffer(&gtk::EntryBuffer::new(None));
    let btn_tx = gtk::Button::new_with_label("Refresh"); // TODO use refresh icon
    let sep_tx = gtk::Separator::new(Orientation::Horizontal);
    lbl_tx.set_halign(Align::Start);
    sep_tx.set_valign(Align::Center);

    // received
    let lbl_rx = gtk::Label::new(Some("Received bits"));
    let ent_rx = gtk::Entry::new_with_buffer(&gtk::EntryBuffer::new(None));
    let btn_rx = gtk::Button::new_with_label("Randomise"); // TODO use dice icon
    let sep_rx = gtk::Separator::new(Orientation::Horizontal);
    lbl_rx.set_halign(Align::Start);
    sep_rx.set_valign(Align::Center);

    let btn_start = gtk::Button::new_with_label("START");
    let drawing = gtk::DrawingArea::new();
    btn_start.connect_clicked(clone!(ent_xs, ent_gs, ent_pr => move |_| {
        let xs = ent_xs.get_buffer().get_text();
        let gs = ent_gs.get_buffer().get_text();
        let pr = ent_pr.get_buffer().get_text();
        stack(xs, gs, pr).unwrap();
    }));

    // arrange widgets
    pack_start!(box_main => box_left, box_right);
    pack_start!(box_left => lbl_xs, ent_xs, sep_xs);
    pack_start!(box_left => lbl_gs, ent_gs, sep_gs);
    pack_start!(box_left => lbl_pr, ent_pr, sep_pr);
    pack_start!(box_left => lbl_tx, ent_tx, btn_tx, sep_tx);
    pack_start!(box_left => lbl_rx, ent_rx, btn_rx, sep_rx);
    box_left.pack_end(&btn_start, true, true, 0);
    box_right.pack_start(&drawing, true, true, 0);

    window.set_title("convolutional-stack");
    window.set_border_width(10);
    window.set_default_size(800, 600);

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    window.add(&box_main);
    window.show_all();
    gtk::main();
}
