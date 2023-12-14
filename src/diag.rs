pub struct Loc {
    pub file_path: String,
    pub row: usize,
    pub col: usize,
}

macro_rules! report {
    ($loc:expr, $level:literal, $fmt:literal) => {
        let diag::Loc{file_path, row, col} = $loc;
        let level = $level;
        eprint!("{file_path}:{row}:{col}: {level}: ");
        eprintln!($fmt);
    };
    ($loc:expr, $level:literal, $fmt:literal, $($args:tt)*) => {
        let Loc{file_path, row, col} = $loc;
        let level = $level;
        eprint!("{file_path}:{row}:{col}: {level}: ");
        eprintln!($fmt, $($args)*);
    };
}
