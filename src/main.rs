extern crate convolutional_stack;
extern crate gtk;
extern crate cairo;

use std::io::{Error, ErrorKind};
use std::rc::Rc;
use std::cell::RefCell;
use convolutional_stack as cs;
use gtk::{Orientation, Align, MessageType, ButtonsType};
use gtk::prelude::*;

const STEP_PX: f64 = 120.;

// make pack_start easier for default values
macro_rules! pack_start {
    ($b:ident, $e:expr, $f:expr => $( $i:ident ),+) => {
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

// make error handling easier
macro_rules! error_dialog {
    ($w:ident, $e:expr) => {
        match $e {
            Ok(x) => x,
            Err(e) => {
                let dialog = gtk::MessageDialog::new(Some(&$w),
                                                     gtk::DialogFlags::empty(),
                                                     MessageType::Error,
                                                     ButtonsType::Close,
                                                     &e.to_string());
                dialog.run();
                dialog.destroy();
                return
            }
        }
    }
}

// markup in pango
fn format_gens(gs: &Vec<Vec<u8>>) -> String {
    let format_gen = |g: &Vec<u8>| -> String {
        let mut res = "".to_string();
        for (i, val) in g.iter().enumerate() {
            match i {
                0 => res.push_str(&format!("{} + ", val)),
                _ => {
                    match val {
                        &0 => (),
                        &1 => res.push_str(&format!("x<sup>{}</sup> + ", i)),
                        _ => panic!("Not binary!"),
                    }
                }
            }
        }
        let new_len = res.len() - 3;
        res.truncate(new_len);
        res
    };

    let mut res = "".to_string();
    for g in gs {
        res.push_str(&format_gen(g));
        res.push('\n');
    }
    res.pop().unwrap(); // remove the final \n
    res
}

fn bin_to_char(x: &u8) -> char {
    match x {
        &0 => '0',
        &1 => '1',
        _ => panic!("Not binary!"),
    }
}

fn format_bin(xs: &Vec<u8>) -> String {
    xs.iter().map(|x| {
        bin_to_char(x)
    }).collect()
}

// markup with pango
fn format_bin_with_error(xs: &Vec<u8>, ys: &Vec<u8>) -> String {
    xs.iter().zip(ys.iter()).map(|(x, y)| {
        match x == y {
            true => format!("<span>{}</span>", bin_to_char(y)),
            false => format!("<span foreground=\"red\">{}</span>", bin_to_char(y)),
        }
    }).collect()
}

fn encode_main(xs: &str, gs: &str) -> Result<Vec<u8>, Error> {
    // shadow the input params
    let xs = try!(cs::parse_bin(&xs));
    let gs = try!(cs::parse_gs(&gs));
    Ok(cs::encode(&xs, &gs))
}

fn run_stack_algo(xs: &str, gs: &str, pr: &str, rx: &str) -> Result<cs::StackResults, Error>{
    // shadow the input params
    let xs = try!(cs::parse_bin(xs));
    let gs = try!(cs::parse_gs(gs));
    let pr = try!(cs::parse_pr(pr));
    let ys = cs::encode(&xs, &gs);

    let noisy_ys = try!(cs::parse_bin(rx));
    if noisy_ys.len() != ys.len() {
        return Err(Error::new(ErrorKind::InvalidInput, "Transmitted and received bits have different lengths"));
    }

    let (path, paths) = cs::decode_(&noisy_ys, &gs, pr);

    Ok(cs::StackResults {
        gens: gs,
        input: xs,
        encoded: ys,
        received: noisy_ys,
        decoded: path,
        paths: paths
    })
}

fn max_len(vv: &Vec<cs::CodePath>) -> usize {
    vv.iter().map(|v| v.path.len()).max().unwrap()
}

struct DrawingWindow {
    shared_lvl: Rc<RefCell<usize>>,
}

impl DrawingWindow {
    fn new() -> DrawingWindow {
        DrawingWindow {
            shared_lvl: Rc::new(RefCell::new(0)),
        }
    }

    fn run(&self, res: cs::StackResults) {
        // properties derived from results
        let drawing_w = max_len(&res.paths) * STEP_PX as usize + 150;
        let max_lvl = res.paths.len();
        let decoded_l = res.decoded.len();

        // containers and widgets
        let box_popup = gtk::Box::new(Orientation::Horizontal, 0);
        let box_drawing = gtk::Box::new(Orientation::Vertical, 0);
        let box_nav = gtk::Box::new(Orientation::Horizontal, 0);
        let grid_info = gtk::Grid::new();
        grid_info.set_column_spacing(10);

        let drawing = gtk::DrawingArea::new();
        drawing.set_size_request(drawing_w as i32, 800);

        let btn_next = gtk::Button::new_with_label(">");
        let btn_back = gtk::Button::new_with_label("<");
        btn_next.set_halign(Align::Center);
        btn_back.set_halign(Align::Center);

        let lbl_xs = gtk::Label::new(Some("Input:"));
        let lbl_tx = gtk::Label::new(Some("Encoded:"));
        let lbl_rx = gtk::Label::new(Some("Received:"));
        let lbl_out = gtk::Label::new(Some("Decoded:"));
        let lbl_m = gtk::Label::new(Some("Order (m):"));
        let lbl_rate = gtk::Label::new(Some("Code rate:"));
        let lbl_gens = gtk::Label::new(Some("Generators:"));

        let data_xs = gtk::Label::new(Some(&format_bin(&res.input)));
        let data_tx = gtk::Label::new(Some(&format_bin(&res.encoded)));
        let data_rx = gtk::Label::new(None);
        data_rx.set_markup(&format_bin_with_error(&res.encoded, &res.received));
        let data_out = gtk::Label::new(None);
        data_out.set_markup(&format_bin_with_error(&res.input, &res.decoded));
        let data_m = gtk::Label::new(Some(&res.gens.m.to_string()));
        let rate = decoded_l as f64 / res.encoded.len() as f64;
        let data_rate = gtk::Label::new(Some(&format!("{:.2}", rate)));
        let data_gens = gtk::Label::new(None);
        data_gens.set_markup(&format_gens(&res.gens.gs));

        // set layout
        grid_info.attach(&lbl_xs, 0, 0, 1, 1);
        grid_info.attach(&lbl_tx, 0, 1, 1, 1);
        grid_info.attach(&lbl_rx, 0, 2, 1, 1);
        grid_info.attach(&lbl_out, 0, 3, 1, 1);
        grid_info.attach(&lbl_m, 0, 4, 1, 1);
        grid_info.attach(&lbl_rate, 0, 5, 1, 1);
        grid_info.attach(&lbl_gens, 0, 6, 1, 1);

        grid_info.attach(&data_xs, 1, 0, 1, 1);
        grid_info.attach(&data_tx, 1, 1, 1, 1);
        grid_info.attach(&data_rx, 1, 2, 1, 1);
        grid_info.attach(&data_out, 1, 3, 1, 1);
        grid_info.attach(&data_m, 1, 4, 1, 1);
        grid_info.attach(&data_rate, 1, 5, 1, 1);
        grid_info.attach(&data_gens, 1, 6, 1, 1);

        pack_start!(box_nav, true, false => btn_back, btn_next);
        box_drawing.pack_start(&drawing, true, true, 0);
        box_drawing.pack_end(&box_nav, true, true, 0);

        box_popup.pack_start(&box_drawing, true, true, 0);
        box_popup.pack_start(&grid_info, true, true, 0);

        // callbacks
        let shared_lvl = self.shared_lvl.clone();
        btn_next.connect_clicked(clone!(drawing, btn_back, btn_next => move |_| {
            let lvl = *shared_lvl.borrow_mut();
            if lvl < max_lvl {
                *shared_lvl.borrow_mut() = lvl + 1;
                drawing.queue_draw();

                btn_back.set_sensitive(true);
                if *shared_lvl.borrow_mut() == max_lvl {
                    btn_next.set_sensitive(false);
                }
            }
        }));

        let shared_lvl = self.shared_lvl.clone();
        btn_back.connect_clicked(clone!(drawing, btn_back, btn_next => move |_| {
            let lvl = *shared_lvl.borrow_mut();
            if lvl > 0 {
                *shared_lvl.borrow_mut() = lvl - 1;
                drawing.queue_draw();

                btn_next.set_sensitive(true);
                if *shared_lvl.borrow_mut() == 0 {
                    btn_back.set_sensitive(false);
                }
            }
        }));

        let shared_lvl = self.shared_lvl.clone();
        drawing.connect_draw(clone!(drawing, btn_back => move |_, cr| {
            let h = drawing.get_allocated_height() as f64;
            // show a message if there's nothing to be drawn
            if *shared_lvl.borrow() == 0 {
                cr.move_to(0., h / 2.);
                cr.set_font_size(20.);
                cr.show_text("Click the '>' button to incrementally draw the tree");
                cr.move_to(0., h / 2. + 30.);
                cr.show_text("and the '<' button to step back.");
                cr.stroke();
                btn_back.set_sensitive(false);
                return Inhibit(false);
            }

            // do the drawing
            for i in 0..*shared_lvl.borrow() {
                let path = &res.paths[i];
                cr.move_to(0., h / 2.);

                // use red for the final path
                if i == res.paths.len() - 1 {
                    cr.set_line_width(4.);
                    cr.set_source_rgb(1.0, 0.5, 0.5);
                } else {
                    cr.set_line_width(2.);
                    cr.set_source_rgb(0.5, 0.5, 1.0);
                }

                DrawingWindow::draw_path(cr, h / 2., 1, path.path.clone(), &path.code, path.mu, decoded_l);
            }

            Inhibit(false)
        }));

        // make and show popup
        // no need delete event because the default action is to destroy the window
        let popup = gtk::Window::new(gtk::WindowType::Toplevel);
        popup.set_title("tree");
        popup.set_border_width(10);
        popup.add(&box_popup);
        popup.show_all();
    }

    fn draw_path(cr: &cairo::Context, h: f64, lvl: usize, mut path: Vec<u8>, code: &Vec<u8>, mu: f64, l: usize) {
        if path.is_empty() {
            cr.rel_move_to(0., -15.); // no need to move back because we're return at the end
            cr.set_font_size(15.);
            cr.set_source_rgb(0., 0., 0.);
            cr.show_text(&format!("{} | {:.2}", &format_bin(code), mu));
            cr.stroke();
            return
        }

        let p = path.remove(0);
        let (x, y) = cr.get_current_point();
        let h = h / 2.0; // shadow
        cr.arc(x, y, 5., 0., 2. * ::std::f64::consts::PI);

        // draw straight line if we're at the last m positions
        if lvl > l {
            cr.set_dash(&[], 0.);
            cr.rel_line_to(STEP_PX, 0.);
        }
        else if p == 0 {
            cr.set_dash(&[], 0.);
            cr.rel_line_to(STEP_PX, -h);
        }
        else if p == 1 {
            cr.set_dash(&[8.0], 0.);
            cr.rel_line_to(STEP_PX, h);
        }
        else {
            panic!("Must be 0 or 1");
        }

        // prepare the current point for the recursive step
        let (x, y) = cr.get_current_point();
        cr.stroke();
        cr.move_to(x, y);

        // recursive step
        DrawingWindow::draw_path(cr, h, lvl+1, path, code, mu, l)
    }
}

struct MainWindow {}

impl MainWindow {
    fn new() -> MainWindow {
        MainWindow {}
    }

    fn run(&self) {
        gtk::init().unwrap();

        // containers
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        let box_main = gtk::Box::new(Orientation::Vertical, 0);
        let box_tx = gtk::Box::new(Orientation::Horizontal, 0);
        let box_rx = gtk::Box::new(Orientation::Horizontal, 0);
        let sep_margin = 20;

        // input
        let lbl_xs = gtk::Label::new(Some("Binary input,\nany characters other than '0' or '1' are ignored."));
        let ent_xs = gtk::Entry::new_with_buffer(&gtk::EntryBuffer::new(Some("01")));
        let sep_xs = gtk::Separator::new(Orientation::Horizontal);
        lbl_xs.set_halign(Align::Start);
        sep_xs.set_valign(Align::Center);
        sep_xs.set_margin_top(sep_margin);
        sep_xs.set_margin_bottom(sep_margin);

        // generators
        let lbl_gs = gtk::Label::new(Some("Generators, separated by commas."));
        let ent_gs = gtk::Entry::new_with_buffer(&gtk::EntryBuffer::new(Some("101,110")));
        let sep_gs = gtk::Separator::new(Orientation::Horizontal);
        lbl_gs.set_halign(Align::Start);
        sep_gs.set_valign(Align::Center);
        sep_gs.set_margin_top(sep_margin);
        sep_gs.set_margin_bottom(sep_margin);

        // error probability
        let lbl_pr = gtk::Label::new(Some("Error probability p, where 0 < p < 1."));
        let ent_pr = gtk::Entry::new_with_buffer(&gtk::EntryBuffer::new(Some("0.1")));
        let sep_pr = gtk::Separator::new(Orientation::Horizontal);
        lbl_pr.set_halign(Align::Start);
        sep_pr.set_valign(Align::Center);
        sep_pr.set_margin_top(sep_margin);
        sep_pr.set_margin_bottom(sep_margin);

        // transmitted
        let lbl_tx = gtk::Label::new(Some("Transmitted bits\nclick the 'r' button to refresh"));
        let ent_tx = gtk::Label::new(Some("")); // actually a label
        let btn_tx = gtk::Button::new_with_label("r"); // TODO use refresh icon
        let sep_tx = gtk::Separator::new(Orientation::Horizontal);
        lbl_tx.set_halign(Align::Start);
        ent_tx.set_selectable(true);
        ent_tx.set_halign(Align::Start);
        sep_tx.set_valign(Align::Center);
        sep_tx.set_margin_top(sep_margin);
        sep_tx.set_margin_bottom(sep_margin);

        // received
        let lbl_rx = gtk::Label::new(Some("Received bits\nclick the 'r' button to randomise"));
        let ent_rx = gtk::Entry::new_with_buffer(&gtk::EntryBuffer::new(None));
        let btn_rx = gtk::Button::new_with_label("r"); // TODO use dice icon
        let sep_rx = gtk::Separator::new(Orientation::Horizontal);
        lbl_rx.set_halign(Align::Start);
        sep_rx.set_valign(Align::Center);
        sep_rx.set_margin_top(sep_margin);
        sep_rx.set_margin_bottom(sep_margin);

        // start button
        let btn_start = gtk::Button::new_with_label("START");

        // arrange widgets
        pack_start!(box_main, false, false => lbl_xs, ent_xs, sep_xs);
        pack_start!(box_main, false, false => lbl_gs, ent_gs, sep_gs);
        pack_start!(box_main, false, false => lbl_pr, ent_pr, sep_pr);

        box_tx.pack_start(&ent_tx, true, true, 0);
        box_tx.pack_end(&btn_tx, false, false, 0);
        pack_start!(box_main, false, false => lbl_tx, box_tx, sep_tx);

        box_rx.pack_start(&ent_rx, true, true, 0);
        box_rx.pack_end(&btn_rx, false, false, 0);
        pack_start!(box_main, false, false => lbl_rx, box_rx, sep_rx);

        box_main.pack_end(&btn_start, false, false, 0);

        // call backs
        btn_tx.connect_clicked(clone!(ent_xs, ent_gs, ent_tx => move |_| {
            let xs = ent_xs.get_buffer().get_text();
            let gs = ent_gs.get_buffer().get_text();
            let ys = encode_main(&xs, &gs).unwrap();
            ent_tx.set_text(&format_bin(&ys));
        }));

        btn_rx.connect_clicked(clone!(ent_pr, ent_tx, ent_rx, window => move |_| {
            let ys = error_dialog!(window, cs::parse_bin(&{
                match ent_tx.get_text() {
                    Some(x) => x,
                    None    => "".to_string(),
                }
            }));
            let pr = error_dialog!(window, cs::parse_pr(&ent_pr.get_buffer().get_text()));
            let noisy_ys = cs::create_noise(&ys, pr);
            ent_rx.set_text(&format_bin(&noisy_ys));
        }));

        btn_start.connect_clicked(clone!(ent_xs, ent_gs, ent_pr, ent_rx, window => move |_| {
            let xs = ent_xs.get_buffer().get_text();
            let gs = ent_gs.get_buffer().get_text();
            let pr = ent_pr.get_buffer().get_text();
            let rx = ent_rx.get_buffer().get_text();

            let res = error_dialog!(window, run_stack_algo(&xs, &gs, &pr, &rx));

            let dw = DrawingWindow::new();
            dw.run(res); // blocks?
        }));

        // main window
        window.set_title("convolutional-stack");
        window.set_border_width(10);
        // window.maximize();
        // window.set_default_size(800, 600);

        window.connect_delete_event(|_, _| {
            // also closes the DrawingWindow(s)
            gtk::main_quit();
            Inhibit(false)
        });

        window.add(&box_main);
        window.show_all();
        gtk::main();
    }
}

fn main() {
    let mw = MainWindow::new();
    mw.run();
}
