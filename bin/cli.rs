extern mod extra;

use extra::getopts::*;
use std::os;
use std::io;
use linearscan::*;

#[path="../src/linearscan.rs"]
mod linearscan;

fn print_usage(program: ~str) {
  io::println(fmt!("Usage: %s [options] input.ls", program));
  io::println(fmt!("-h, --help\tPrint this message"));
}

fn main() {
  let args = os::args();
  let program = args[0].clone();

  let opts = ~[
    optflag("h"),
    optflag("help")
  ];

  let matches = getopts(args.tail(), opts).get();
  if opt_present(&matches, "h") || opt_present(&matches, "help") {
    return print_usage(program);
  }
}
