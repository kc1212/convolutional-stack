from typing import List

import json
import subprocess
import gi
gi.require_version('Gtk', '3.0')
from gi.repository import Gtk, cairo

def parse_bin(s: str) -> List[int]:
    return [1 if x == '1' else 0 for x in s]

def parse_gen(s: str) -> List[List[int]]:
    return [parse_bin(s) for s in s.split(',')]

def pack_start_all(box, widgets, expand = True, fill = True, padding = 0):
    for w in widgets:
        box.pack_start(w, expand, fill, padding)

class Dialog(Gtk.Dialog):
    def __init__(self, parent, results):
        print(results)
        Gtk.Dialog.__init__(self, "My Dialog", parent, 0)

        vbox = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        hbox = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        box = self.get_content_area()
        box.add(vbox)

        lbl = Gtk.Label("encoded:\n"
                        + str(results["encoded"]) + "\n"
                        + "observed:\n"
                        + str(results["observed"]) + "\n"
                        + "decoded:\n"
                        + str(results["decoded"]) + "\n")
        lbl.set_line_wrap(True)

        self.l = len(results["decoded"]) - results["m"]
        self.tree_data = results["paths"]
        self.darea = Gtk.DrawingArea()
        self.darea.connect("draw", self.draw)
        self.darea.set_size_request(500, 500)

        pack_start_all(vbox, [lbl, self.darea, hbox])

        self.btn_back = Gtk.Button(label="<<", halign=Gtk.Align.END)
        self.btn_forward = Gtk.Button(label=">>", halign=Gtk.Align.START)
        self.btn_back.connect("clicked", self.on_btn_back)
        self.btn_forward.connect("clicked", self.on_btn_forward)

        pack_start_all(hbox, [self.btn_back, self.btn_forward]) # order matters

        self.show_all()

    def on_btn_back(self, btn):
        pass

    def on_btn_forward(self, btn):
        pass

    def draw_path(self, cr, h, path, mu, lvl):
        if not path:
            cr.show_text("{:.2f}".format(mu))
            cr.set_line_width(1)
            cr.stroke()
            return

        p = path.pop(0)

        # start at the point where we want to start drawing
        x, y = cr.get_current_point()
        h = h / 2
        cr.rectangle(x - 10, y - 10, 20, 20)
        cr.move_to(x, y)

        # draw straight line if we're at the last m positions
        if lvl > self.l:
            cr.rel_line_to(100, 0)
        elif p == 0:
            cr.rel_line_to(100, -h)
        elif p == 1:
            cr.rel_line_to(100, h)
        else:
            assert False, "Must be 0 or 1"

        # recursive step
        self.draw_path(cr, h, path, mu, lvl+1)

    def draw(self, darea, cr):
        # red
        cr.set_source_rgba(0.5, 0.0, 0.0, 1.0)

        # get the width and height of the drawing area
        w = self.darea.get_allocated_width()
        h = self.darea.get_allocated_height()

        for path in self.tree_data:
            cr.move_to(0, h/2)
            self.draw_path(cr, h/2, list(path["path"]), path["mu"], 1)


class Window(Gtk.Window):
    def __init__(self):
        Gtk.Window.__init__(self, title="Demo")
        self.set_border_width(6)

        vbox = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        self.add(vbox)

        # input section
        self.lbl_xs = Gtk.Label(label="Binary input,\nany characters other than '0' or '1' are ignored.", halign=Gtk.Align.START)
        self.entry_xs = Gtk.Entry(text="01")
        self.sep_xs = Gtk.Separator(valign=Gtk.Align.CENTER)
        pack_start_all(vbox, [self.lbl_xs, self.entry_xs, self.sep_xs])

        # generators section
        self.lbl_gs = Gtk.Label(label="Generators,\nseparated by commas.", halign=Gtk.Align.START)
        self.entry_gs = Gtk.Entry(text="101,110")
        self.sep_gs = Gtk.Separator(valign=Gtk.Align.CENTER)
        pack_start_all(vbox, [self.lbl_gs, self.entry_gs, self.sep_gs])

        # probability section
        self.lbl_p = Gtk.Label(label="Error probability p,\nwhere 0 < p < 1.", halign=Gtk.Align.START)
        self.entry_p = Gtk.Entry(text="0.1")
        self.sep_p = Gtk.Separator(valign=Gtk.Align.CENTER)
        pack_start_all(vbox, [self.lbl_p, self.entry_p, self.sep_p])

        # start button
        self.btn_start = Gtk.Button(label="Start", halign=Gtk.Align.CENTER)
        vbox.pack_start(self.btn_start, True, True, 0)

        # pack everything
        # signals
        self.connect("delete-event", Gtk.main_quit)
        self.btn_start.connect("clicked", self.code)

    def code(self, btn):
        # TODO make this elegant
        user_data = {"xs": self.entry_xs,
                     "gs": self.entry_gs,
                     "p": self.entry_p}
        d = {k: v.get_buffer().get_text() for k, v in user_data.items()}
        d["xs"] = parse_bin(d['xs'])
        d["gs"] = parse_gen(d['gs'])
        d["p"] = float(d['p'])

        p = subprocess.Popen(['./target/release/convolutional-code'],
                             stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        out, err = p.communicate(json.dumps(d).encode())

        if err:
            print(err.decode())
            dialog = Gtk.MessageDialog(self, 0, Gtk.MessageType.ERROR,
                                       Gtk.ButtonsType.CLOSE, err.decode())
            dialog.run()
            dialog.destroy()
            return

        dialog = Dialog(self, json.loads(out.decode()))
        dialog.run()
        dialog.destroy()


if __name__ == "__main__":
    win = Window()
    win.show_all()
    Gtk.main()
