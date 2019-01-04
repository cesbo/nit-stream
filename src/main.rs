use std::{env, time, thread};

mod error;
use crate::error::{Error, Result};

mod config;
use crate::config::parse_config;

include!(concat!(env!("OUT_DIR"), "/build.rs"));

fn version() {
    println!("nit-stream v.{} commit:{}", env!("CARGO_PKG_VERSION"), COMMIT);
}

fn usage(program: &str) {
    println!(r#"Usage: {} CONFIG

OPTIONS:
    -v, --version       Version information
    -h, --help          Print this text

CONFIG:
    Path to configuration file
"#, program);
}

#[derive(Default, Debug)]
pub struct Instance {
    pub multiplex_list: Vec<Multiplex>,

    pub codepage: u8,
    pub network_id: u16,
    pub nit_version: u8,
    pub provider: String,
    pub network: String,
    pub onid: u16,
}

#[derive(Default, Debug)]
pub struct Multiplex {
    pub codepage: u8,
    pub network_id: u16,
    pub nit_version: u8,
    pub provider: String,
    pub network: String,
    pub onid: u16,

    pub enable: bool,
    pub name: String,
    pub tsid: u16,

    // TODO: delivery system

    pub service_list: Vec<Service>,
}

#[derive(Default, Debug)]
pub struct Service {
    pub name: String,
    pub pnr: u16,
    pub service_type: u8,
    pub lcn: u16,
}

fn wrap() -> Result<()> {
    // Parse Options
    let mut args = env::args();
    let program = args.next().unwrap();
    let arg = match args.next() {
        Some(v) => match v.as_ref() {
            "-v" | "--version" => { version(); return Ok(()); },
            "-h" | "--help" => { usage(&program); return Ok(()); },
            _ => v,
        },
        None => {
            usage(&program);
            return Err(Error::from("Path to configuration file requried"));
        },
    };

    let mut instance = Instance::default();

    // Prase config
    parse_config(&mut instance, &arg)?;

    // TODO: check configuration
    // TODO: Main loop

    let loop_delay_ms = time::Duration::from_millis(250);

    loop {
        thread::sleep(loop_delay_ms);
    }
}

fn main() {
    if let Err(e) = wrap() {
        println!("Error: {}", e.to_string());
    }
}
