extern crate convolutional_stack;
extern crate gtk;
extern crate cairo;

use std::io::{self, Error, ErrorKind};
use std::rc::Rc;
use std::cell::RefCell;
use convolutional_stack as cs;
use gtk::{Orientation, Align};
use gtk::prelude::*;
use cairo::Context;

// make pack_start easier for default values
macro_rules! pack_start {
    ( $b:ident, $e:expr, $f:expr => $( $i:ident ),+ ) => {
        $(
            $b.pack_start(&$i, $e, $f, 0);
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

fn format_bin(xs: &Vec<u8>) -> String {
    xs.iter().map(|x| {
        match x {
            &0 => '0',
            &1 => '1',
            _ => panic!("Not binary!"),
        }
    }).collect()
}

fn encode_main(xs: &str, gs: &str) -> Result<Vec<u8>, Error> {
    // shadow the input params
    let xs = try!(cs::parse_bin(&xs));
    let gs = try!(cs::parse_gs(&gs));
    Ok(cs::encode(&xs, &gs))
}

fn stack(xs: &str, gs: &str, pr: &str) -> Result<cs::StackResults, Error>{
    // shadow the input params
    let xs = try!(cs::parse_bin(xs));
    let gs = try!(cs::parse_gs(gs));
    let pr = try!(cs::parse_pr(pr));

    let ys = cs::encode(&xs, &gs);
    let noisy_ys = cs::create_noise(&ys, pr);
    let (path, paths) = cs::decode_(&noisy_ys, &gs, pr);

    Ok(cs::StackResults {
        m: gs.m,
        n: gs.n,
        encoded: ys,
        observed: noisy_ys,
        decoded: path.path,
        paths: paths
    })
}

struct MainWindow {
    shared_results: Rc<RefCell<cs::StackResults>>,
    shared_lvl: Rc<RefCell<usize>>,
}

impl MainWindow {
    fn new() -> MainWindow {
        MainWindow {
            shared_results: Rc::new(RefCell::new(cs::StackResults::new())),
            shared_lvl: Rc::new(RefCell::new(0)),
        }
    }

    fn run(&self) {
        gtk::init().unwrap();

        // containers
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        let box_main = gtk::Box::new(Orientation::Horizontal, 0);
        let box_left = gtk::Box::new(Orientation::Vertical, 0);
        let box_right = gtk::Box::new(Orientation::Vertical, 0);
        let sep_margin = 20;

        // input
        let lbl_xs = gtk::Label::new(Some("Binary input,\nany characters other than '0' or '1' are ignored."));
        let ent_xs = gtk::Entry::new_with_buffer(&gtk::EntryBuffer::new(Some("0101")));
        let sep_xs = gtk::Separator::new(Orientation::Horizontal);
        lbl_xs.set_halign(Align::Start);
        sep_xs.set_valign(Align::Center);
        sep_xs.set_margin_top(sep_margin);
        sep_xs.set_margin_bottom(sep_margin);

        // generators
        let lbl_gs = gtk::Label::new(Some("Generators,\nseparated by commas."));
        let ent_gs = gtk::Entry::new_with_buffer(&gtk::EntryBuffer::new(Some("101,110")));
        let sep_gs = gtk::Separator::new(Orientation::Horizontal);
        lbl_gs.set_halign(Align::Start);
        sep_gs.set_valign(Align::Center);
        sep_gs.set_margin_top(sep_margin);
        sep_gs.set_margin_bottom(sep_margin);

        // error probability
        let lbl_pr = gtk::Label::new(Some("Error probability p,\nwhere 0 < p < 1."));
        let ent_pr = gtk::Entry::new_with_buffer(&gtk::EntryBuffer::new(Some("0.1")));
        let sep_pr = gtk::Separator::new(Orientation::Horizontal);
        lbl_pr.set_halign(Align::Start);
        sep_pr.set_valign(Align::Center);
        sep_pr.set_margin_top(sep_margin);
        sep_pr.set_margin_bottom(sep_margin);

        // transmitted
        let lbl_tx = gtk::Label::new(Some("Transmitted bits"));
        let ent_tx = gtk::Label::new(Some("Click the refresh button to update")); // actually a label
        let btn_tx = gtk::Button::new_with_label("Refresh"); // TODO use refresh icon
        let sep_tx = gtk::Separator::new(Orientation::Horizontal);
        lbl_tx.set_halign(Align::Start);
        ent_tx.set_selectable(true);
        ent_tx.set_halign(Align::Start);
        sep_tx.set_valign(Align::Center);
        sep_tx.set_margin_top(sep_margin);
        sep_tx.set_margin_bottom(sep_margin);
        btn_tx.connect_clicked(clone!(ent_xs, ent_gs, ent_tx => move |_| {
            let xs = ent_xs.get_buffer().get_text();
            let gs = ent_gs.get_buffer().get_text();
            let ys = encode_main(&xs, &gs).unwrap(); // TODO handle error
            ent_tx.set_text(&format_bin(&ys));
        }));

        // received
        let lbl_rx = gtk::Label::new(Some("Received bits"));
        let ent_rx = gtk::Entry::new_with_buffer(&gtk::EntryBuffer::new(None));
        let btn_rx = gtk::Button::new_with_label("Randomise"); // TODO use dice icon
        let sep_rx = gtk::Separator::new(Orientation::Horizontal);
        lbl_rx.set_halign(Align::Start);
        sep_rx.set_valign(Align::Center);
        sep_rx.set_margin_top(sep_margin);
        sep_rx.set_margin_bottom(sep_margin);
        btn_rx.connect_clicked(clone!(ent_pr, ent_tx, ent_rx => move |_| {
            let ys = cs::parse_bin(&{
                match ent_tx.get_text() {
                    Some(x) => x,
                    None    => "".to_string(),
                }
            }).unwrap(); // TODO
            let pr = cs::parse_pr(&ent_pr.get_buffer().get_text()).unwrap(); // TODO
            let noisy_ys = cs::create_noise(&ys, pr);
            ent_rx.set_text(&format_bin(&noisy_ys));
        }));

        // drawing area
        let drawing = gtk::DrawingArea::new();
        let shared_results = self.shared_results.clone();
        let code_lvl = self.shared_lvl.clone();
        drawing.set_size_request(500, 800);
        drawing.connect_draw(clone!(drawing => move |_, cr| {
            let res = shared_results.borrow();

            cr.set_source_rgba(0.0, 0.0, 0.5, 1.0);
            let w = drawing.get_allocated_width() as f64;
            let h = drawing.get_allocated_height() as f64;

            // show a message if there's nothing to be drawn
            if res.m == 0 && res.n == 0 {
                cr.move_to(w / 2., h / 2.);
                cr.show_text("Nothing here");
                cr.stroke();
                return Inhibit(false);
            }

            // do the drawing
            for path in &res.paths { // TODO draw incrementally
                println!("{:?}, {}", path, *code_lvl.borrow());
                cr.move_to(0., h / 2.);
                MainWindow::draw_path(cr, h / 2., 1, path.path.clone(), path.mu, *code_lvl.borrow());
            }

            Inhibit(false)
        }));

        // start button
        let btn_start = gtk::Button::new_with_label("START");
        let shared_results = self.shared_results.clone();
        let shared_lvl = self.shared_lvl.clone();
        btn_start.connect_clicked(clone!(ent_xs, ent_gs, ent_pr, drawing => move |_| {
            let xs = ent_xs.get_buffer().get_text();
            let gs = ent_gs.get_buffer().get_text();
            let pr = ent_pr.get_buffer().get_text();
            let res = stack(&xs, &gs, &pr).unwrap(); // TODO handle error
            *shared_lvl.borrow_mut() = res.decoded.len() - res.m;
            *shared_results.borrow_mut() = res;
            drawing.queue_draw();
        }));

        // arrange widgets
        box_main.pack_start(&box_left, false, false, 0);
        box_main.pack_start(&box_right, true, true, 0);
        pack_start!(box_left, false, false => lbl_xs, ent_xs, sep_xs);
        pack_start!(box_left, false, false => lbl_gs, ent_gs, sep_gs);
        pack_start!(box_left, false, false => lbl_pr, ent_pr, sep_pr);
        pack_start!(box_left, false, false => lbl_tx, ent_tx, btn_tx, sep_tx);
        pack_start!(box_left, false, false => lbl_rx, ent_rx, btn_rx, sep_rx);
        box_left.pack_end(&btn_start, false, false, 0);
        box_right.pack_start(&drawing, true, true, 0);

        window.set_title("convolutional-stack");
        window.set_border_width(10);
        // window.maximize();
        // window.set_default_size(800, 600);

        window.connect_delete_event(|_, _| {
            gtk::main_quit();
            Inhibit(false)
        });

        window.add(&box_main);
        window.show_all();
        gtk::main();
    }

    fn draw_path(cr: &cairo::Context, h: f64, lvl: usize, mut path: Vec<u8>, mu: f64, code_lvl: usize) {
        const STEP_PX: f64 = 100.;
        if path.is_empty() {
            cr.rel_move_to(0., -15.); // no need to move back because we're return at the end
            cr.set_font_size(20.);
            cr.show_text(&format!("{:.2}", mu));
            cr.set_line_width(2.);
            cr.stroke();
            return
        }

        let p = path.remove(0);
        let (x, y) = cr.get_current_point();
        let h = h / 2.0; // shadow
        cr.arc(x, y, 5., 0., 2. * ::std::f64::consts::PI);
        // cr.fill();

        cr.move_to(x, y);

        // draw straight line if we're at the last m positions
        if lvl > code_lvl {
            cr.rel_line_to(STEP_PX, 0.);
        }
        else if p == 0 {
            cr.rel_line_to(STEP_PX, -h);
        }
        else if p == 1 {
            cr.rel_line_to(STEP_PX, h);
        }
        else {
            panic!("Must be 0 or 1");
        }

        // recursive step
        MainWindow::draw_path(cr, h, lvl+1, path, mu, code_lvl)
    }
}

fn main() {
    let mw = MainWindow::new();
    mw.run();
}
