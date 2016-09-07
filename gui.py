from typing import List

import math
import json
import subprocess
import gi
gi.require_version('Gtk', '3.0')
from gi.repository import Gtk, cairo

def parse_bin(s: str) -> List[int]:
    # die if we can't parse
    return [int(x) for x in s]

def parse_gen(s: str) -> List[List[int]]:
    return [parse_bin(s) for s in s.split(',')]

def pack_start_all(box, widgets, expand = True, fill = True, padding = 0):
    for w in widgets:
        box.pack_start(w, expand, fill, padding)

def max_len(ls, f = lambda x: x) -> int:
    return max([len(f(l)) for l in ls])

STEP_PX = 100
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
        self.path_n = 0
        self.drawing = Gtk.DrawingArea()
        self.drawing.connect("draw", self.draw)
        self.drawing.set_size_request(max_len(self.tree_data, lambda x: x["path"]) * STEP_PX + 50, 800)

        # self.scrolled = Gtk.ScrolledWindow()
        # self.scrolled.add_with_viewport(self.drawing)
        # self.scrolled.set_min_content_height(500)
        # self.scrolled.set_min_content_width(500)
        pack_start_all(vbox, [lbl, self.drawing, hbox])

        self.btn_back = Gtk.Button(label="<<", halign=Gtk.Align.END)
        self.btn_forward = Gtk.Button(label=">>", halign=Gtk.Align.START)
        self.btn_back.connect("clicked", self.on_btn_back)
        self.btn_forward.connect("clicked", self.on_btn_forward)

        pack_start_all(hbox, [self.btn_back, self.btn_forward]) # order matters

        self.show_all()

    def on_btn_back(self, btn):
        if self.path_n > 0:
            self.path_n -= 1
        self.drawing.queue_draw()

    def on_btn_forward(self, btn):
        if self.path_n < len(self.tree_data):
            self.path_n += 1
        self.drawing.queue_draw()

    def draw_path(self, cr, h, path, mu, lvl):
        if not path:
            cr.rel_move_to(0, -15) # no need to move back because we're return at the end
            cr.set_font_size(20)
            cr.show_text("{:.2f}".format(mu))
            cr.set_line_width(2)
            cr.stroke()
            return

        p = path.pop(0)

        # start at the point where we want to start drawing
        x, y = cr.get_current_point()
        h = h / 2
        cr.arc(x, y, 5., 0., 2 * math.pi)
        cr.fill()
        cr.move_to(x, y)

        # draw straight line if we're at the last m positions
        if lvl > self.l:
            cr.rel_line_to(STEP_PX, 0)
        elif p == 0:
            cr.rel_line_to(STEP_PX, -h)
        elif p == 1:
            cr.rel_line_to(STEP_PX, h)
        else:
            assert False, "Must be 0 or 1"

        # recursive step
        self.draw_path(cr, h, path, mu, lvl+1)

    def draw(self, drawing, cr):
        # blue
        cr.set_source_rgba(0.0, 0.0, 0.5, 1.0)

        # get the width and height of the drawing area
        w = self.drawing.get_allocated_width()
        h = self.drawing.get_allocated_height()

        for i in range(self.path_n):
            path = self.tree_data[i]
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
        self.entry_xs = Gtk.Entry(text="0101")
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
        try:
            d["xs"] = parse_bin(d['xs'])
            d["gs"] = parse_gen(d['gs'])
            d["p"] = float(d['p'])
        except ValueError as e:
            dialog = Gtk.MessageDialog(self, 0, Gtk.MessageType.ERROR,
                                       Gtk.ButtonsType.CLOSE, e)
            dialog.run()
            dialog.destroy()
            return


        p = subprocess.Popen(['./target/release/convolutional-stack'],
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
