use std::path::Path;
use std::rc::Rc;

pub struct Loc {
    pub file_path: Rc<Path>,
    pub row: usize,
    pub col: usize,
}

macro_rules! report {
    ($loc:expr, $level:literal, $fmt:literal) => {
        eprint!("{}:{}:{}: {}: ", $loc.file_path.display(), $loc.row, $loc.col, $level);
        eprintln!($fmt);
    };
    ($loc:expr, $level:literal, $fmt:literal, $($args:tt)*) => {
        eprint!("{}:{}:{}: {}: ", $loc.file_path.display(), $loc.row, $loc.col, $level);
        eprintln!($fmt, $($args)*);
    };
}
