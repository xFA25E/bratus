use {
    nix::{
        sys::signal::{kill, Signal::SIGTERM},
        unistd::Pid,
    },
    simple_signal::Signal,
    std::{
        fmt::{self, Display},
        io::{self, stdout, BufRead, BufReader, BufWriter, Write},
        process::{Child, ChildStdout, Command, Stdio},
        str::FromStr,
    },
    structopt::StructOpt,
};

type CmdOut = BufReader<ChildStdout>;
type Res<T> = io::Result<T>;

pub struct Color(Option<String>);
pub struct DrawColor<'a, D: Display>(&'a Color, D);

#[derive(StructOpt)]
/// Bspwm status watcher
pub struct Colors {
    #[structopt(long = "color-free", name = "COLOR_FREE", default_value = "")]
    /// A color for free desktop
    free: Color,
    #[structopt(long = "color-monitor", name = "COLOR_MONITOR", default_value = "")]
    /// A color for monitor
    monitor: Color,
    #[structopt(long = "color-occupied", name = "COLOR_OCCUPIED", default_value = "")]
    /// A color for occupied desktop
    occupied: Color,
    #[structopt(long = "color-urgent", name = "COLOR_URGENT", default_value = "")]
    /// A color for urgent desktop
    urgent: Color,
    #[structopt(long = "color-state", name = "COLOR_STATE", default_value = "")]
    /// A color for window state
    state: Color,
}

pub const BSPWM_CMD: &'static [&'static str] = &["bspc", "subscribe"];

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn run() -> Res<()> {
    let out = stdout();
    let mut out = BufWriter::new(out.lock());
    let colors = Colors::from_args();
    let (child, mut child_stdout) = command_stdout(BSPWM_CMD)?;
    let mut buf = String::new();
    let mut new_buf = String::new();

    simple_signal::set_handler(&[Signal::Term, Signal::Int], move |_| {
        if let Err(e) = kill(Pid::from_raw(child.id() as i32), SIGTERM) {
            eprintln!("{}", e);
        }
        std::process::exit(0);
    });

    loop {
        match child_stdout.read_line(&mut new_buf) {
            Ok(0) => break,
            Ok(_) => {
                new_buf.pop();
                if new_buf != buf {
                    print_bspwm(&colors, &mut out, &new_buf)?;
                }
                buf.clear();
                std::mem::swap(&mut new_buf, &mut buf);
            }
            error => error.map(|_| ())?,
        }
    }
    Ok(())
}

fn print_bspwm(c: &Colors, mut out: impl Write, bspwm: &str) -> Res<()> {
    fn split(s: &str) -> Option<(char, &str)> {
        if s.len() > 1 {
            Some((s.as_bytes()[0] as char, &s[1..]))
        } else {
            None
        }
    }

    for (start, name) in bspwm[1..].split(':').filter_map(split) {
        match start {
            'm' => write!(out, " {}  ", c.monitor.draw(name))?,
            'M' => write!(out, "-{}- ", c.monitor.draw(name))?,
            'f' => write!(out, " {}  ", c.free.draw(name))?,
            'F' => write!(out, "-{}- ", c.free.draw(name))?,
            'o' => write!(out, " {}  ", c.occupied.draw(name))?,
            'O' => write!(out, "-{}- ", c.occupied.draw(name))?,
            'u' => write!(out, " {}  ", c.urgent.draw(name))?,
            'U' => write!(out, "-{}- ", c.urgent.draw(name))?,
            'L' | 'T' | 'G' => write!(out, " {}", c.state.draw(name))?,
            _ => continue,
        }
    }
    writeln!(out)?;
    out.flush()
}

fn command_stdout(command: &[&str]) -> Res<(Child, CmdOut)> {
    let mut child = Command::new(command[0])
        .args(&command[1..])
        .stdout(Stdio::piped())
        .spawn()?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "No stdout of process"))?;

    Ok((child, BufReader::new(stdout)))
}

impl Color {
    pub fn draw<D: Display>(&self, element: D) -> DrawColor<D> {
        DrawColor(self, element)
    }
}

impl FromStr for Color {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Ok(Self(None))
        } else if s.len() == 7
            && s.starts_with("#")
            && s.chars().skip(1).all(|c| c.is_ascii_hexdigit())
        {
            Ok(Self(Some(s.into())))
        } else {
            Err(format!("Invalid hex color: {}", s))
        }
    }
}

impl<'a, D: Display> Display for DrawColor<'a, D> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(c) = &(self.0).0 {
            write!(f, "%{{F{}}}{}%{{F-}}", c, self.1)
        } else {
            write!(f, "{}", self.1)
        }
    }
}
