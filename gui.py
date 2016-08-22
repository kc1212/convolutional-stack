from typing import List

import json
import subprocess
import gi
gi.require_version('Gtk', '3.0')
from gi.repository import Gtk, GdkPixbuf
from graphviz import Digraph

def parse_bin(s: str) -> List[int]:
    return [1 if x == '1' else 0 for x in s]

def parse_gen(s: str) -> List[List[int]]:
    return [parse_bin(s) for s in s.split(',')]

class Dialog(Gtk.Dialog):
    def __init__(self, parent):
        Gtk.Dialog.__init__(self, "My Dialog", parent, 0)

        vbox = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        hbox = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        box = self.get_content_area()
        box.add(vbox)

        # demo graph
        dot = Digraph(comment='The Round Table', format='svg')
        dot.edge('hello', 'world')

        self.img = Gtk.Image()
        pixbuf = GdkPixbuf.Pixbuf.new_from_file_at_scale(dot.render(directory='./target/svg', cleanup=True),
                                                         width = -1, height = -1, preserve_aspect_ratio = True)
        self.img.set_from_pixbuf(pixbuf)
        vbox.pack_start(self.img, True, True, 0)
        vbox.pack_start(hbox, True, True, 0)

        self.btn_back = Gtk.Button(label="<<")
        self.btn_forward = Gtk.Button(label=">>")
        hbox.pack_start(self.btn_back, True, True, 0)
        hbox.pack_start(self.btn_forward, True, True, 0)

        self.show_all()

class Window(Gtk.Window):
    def __init__(self):
        Gtk.Window.__init__(self, title="Demo")
        self.set_border_width(6)

        vbox = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        self.add(vbox)

        # input section
        self.lbl_xs = Gtk.Label(label="Binary input,\nany characters other than '0' or '1' are ignored.", halign=Gtk.Align.START)
        self.entry_xs = Gtk.Entry(text="00010001")
        self.sep_xs = Gtk.Separator(valign=Gtk.Align.CENTER)
        vbox.pack_start(self.lbl_xs, True, True, 0)
        vbox.pack_start(self.entry_xs, True, True, 0)
        vbox.pack_start(self.sep_xs, True, True, 0)

        # generators section
        self.lbl_gs = Gtk.Label(label="Generators,\nseparated by commas.", halign=Gtk.Align.START)
        self.entry_gs = Gtk.Entry(text="101,110")
        self.sep_gs = Gtk.Separator(valign=Gtk.Align.CENTER)
        vbox.pack_start(self.lbl_gs, True, True, 0)
        vbox.pack_start(self.entry_gs, True, True, 0)
        vbox.pack_start(self.sep_gs, True, True, 0)

        # probability section
        self.lbl_p = Gtk.Label(label="Error probability p,\nwhere 0 < p < 1.", halign=Gtk.Align.START)
        self.entry_p = Gtk.Entry(text="0.1")
        self.sep_p = Gtk.Separator(valign=Gtk.Align.CENTER)
        vbox.pack_start(self.lbl_p, True, True, 0)
        vbox.pack_start(self.entry_p, True, True, 0)
        vbox.pack_start(self.sep_p, True, True, 0)

        # start button
        self.btn_start = Gtk.Button(label="Start", halign=Gtk.Align.CENTER)
        vbox.pack_start(self.btn_start, True, True, 0)

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

        p = subprocess.Popen(['./target/release/convolutional-code'], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        out, err = p.communicate(json.dumps(d).encode())

        if err:
            print(err.decode())
            dialog = Gtk.MessageDialog(self, 0, Gtk.MessageType.ERROR,
                                       Gtk.ButtonsType.CLOSE, err.decode())
            dialog.run()
            dialog.destroy()
            return

        dialog = Dialog(self)
        dialog.run()
        dialog.destroy()


if __name__ == "__main__":
    win = Window()
    win.show_all()
    Gtk.main()
