extern crate convolutional_stack;
extern crate gtk;
extern crate cairo;

use std::io::{Error, ErrorKind};
use std::rc::Rc;
use std::cell::RefCell;
use convolutional_stack as cs;
use gtk::{Orientation, Align, MessageType, ButtonsType};
use gtk::prelude::*;

const STEP_PX: f64 = 100.;

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
        m: gs.m,
        n: gs.n,
        encoded: ys,
        observed: noisy_ys,
        decoded: path.path,
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

    fn run(&self, parent: &gtk::Window, res: cs::StackResults) {

        // properties derived from results
        let drawing_w = max_len(&res.paths) * STEP_PX as usize + 100;
        let max_lvl = res.paths.len();
        let decoded_l = res.decoded.len() - res.m;

        // widgets
        let box_popup = gtk::Box::new(Orientation::Vertical, 0);
        let box_nav = gtk::Box::new(Orientation::Horizontal, 0);

        let drawing = gtk::DrawingArea::new();
        drawing.set_size_request(drawing_w as i32, 800);

        let btn_next = gtk::Button::new_with_label(">");
        let btn_back = gtk::Button::new_with_label("<");
        btn_next.set_halign(Align::Center);
        btn_back.set_halign(Align::Center);

        // set layout
        pack_start!(box_nav, true, false => btn_back, btn_next);
        box_popup.pack_start(&drawing, true, true, 0);
        box_popup.pack_end(&box_nav, true, true, 0);

        // callbacks
        let shared_lvl = self.shared_lvl.clone();
        btn_next.connect_clicked(clone!(drawing => move |_| {
            let lvl = *shared_lvl.borrow_mut();
            if lvl < max_lvl {
                *shared_lvl.borrow_mut() = lvl + 1;
            }
            drawing.queue_draw();
        }));

        let shared_lvl = self.shared_lvl.clone();
        btn_back.connect_clicked(clone!(drawing => move |_| {
            let lvl = *shared_lvl.borrow_mut();
            if lvl > 0 {
                *shared_lvl.borrow_mut() = lvl - 1;
            }
            drawing.queue_draw();
        }));

        let shared_lvl = self.shared_lvl.clone();
        drawing.connect_draw(clone!(drawing => move |_, cr| {
            let h = drawing.get_allocated_height() as f64;
            // show a message if there's nothing to be drawn
            if *shared_lvl.borrow() == 0 {
                cr.move_to(0., h / 2.);
                cr.set_font_size(20.);
                cr.show_text("Click the '>' button to incrementally draw the tree");
                cr.move_to(0., h / 2. + 30.);
                cr.show_text("and the '<' button to step back.");
                cr.stroke();
                return Inhibit(false);
            }

            // do the drawing
            for i in 0..*shared_lvl.borrow() {
                let path = &res.paths[i];
                cr.move_to(0., h / 2.);

                // use red for the final path
                if i == res.paths.len() - 1 {
                    cr.set_line_width(4.);
                    cr.set_source_rgba(0.5, 0.0, 0.0, 1.0);
                } else {
                    cr.set_line_width(2.);
                    cr.set_source_rgba(0.0, 0.0, 0.5, 1.0);
                }

                DrawingWindow::draw_path(cr, h / 2., 1, path.path.clone(), path.mu, decoded_l);
            }

            Inhibit(false)
        }));

        // make and show popup
        // no need delete event because the default action is to destroy the window
        let popup = gtk::Window::new(gtk::WindowType::Toplevel);
        popup.set_modal(true);
        popup.set_transient_for(Some(parent));
        popup.set_title("tree");
        popup.set_border_width(10);
        popup.add(&box_popup);

        // move to the right of the parent window
        let (parent_x, parent_y) = parent.get_position();
        let (parent_w, _) = parent.get_size();
        popup.move_(parent_x + parent_w, parent_y);

        popup.show_all();
    }

    fn draw_path(cr: &cairo::Context, h: f64, lvl: usize, mut path: Vec<u8>, mu: f64, l: usize) {
        if path.is_empty() {
            cr.rel_move_to(0., -15.); // no need to move back because we're return at the end
            cr.set_font_size(20.);
            cr.show_text(&format!("{:.2}", mu));
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
        DrawingWindow::draw_path(cr, h, lvl+1, path, mu, l)
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
            dw.run(&window, res); // blocks?
        }));

        // main window
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
}

fn main() {
    let mw = MainWindow::new();
    mw.run();
}
