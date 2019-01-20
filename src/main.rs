use std::{env, time, thread};

use mpegts::{psi, textcode};

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
    pub nit_version: u8,
    pub network_id: u16,
    pub network: String,
    pub codepage: u8,
    pub onid: u16,

    pub multiplex_list: Vec<Multiplex>
}


#[derive(Default, Debug)]
pub struct Multiplex {
    pub tsid: u16,
    pub onid: u16,
    pub enable: bool,

    // TODO: delivery system

    pub service_list: Vec<Service>
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

    // Parse config
    parse_config(&mut instance, &arg)?;

    // NIT
    let mut nit = psi::Nit::default();
    nit.table_id = 0x40;
    nit.version = instance.nit_version;
    nit.network_id = instance.network_id;

    if ! instance.network.is_empty() {
        nit.descriptors.push(
            psi::Descriptor::Desc40(
                psi::Desc40 {
                    name: textcode::StringDVB::from_str(
                        instance.network.as_str(),
                        instance.codepage
                    )
                }
            )
        );
    }

    for multiplex in &instance.multiplex_list {
        if multiplex.enable {
            let mut item = psi::NitItem::default();
            item.tsid = multiplex.tsid;
            item.onid = multiplex.onid;

            nit.items.push(item);
        }
    }

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
