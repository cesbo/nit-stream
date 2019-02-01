use std::{env, cmp, time, thread};

use udp::UdpSocket;
use mpegts::{ts, psi, textcode, psi::PsiDemux};

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


#[derive(Debug)]
pub enum Output {
    None,
    Udp(udp::UdpSocket),
}

impl Default for Output {
    fn default() -> Self {
        Output::None
    }
}

impl Output {
    pub fn open(addr: &str) -> Result<Self> {
        let dst = addr.splitn(2, "://").collect::<Vec<&str>>();
        match dst[0] {
            "udp" => {
                let s = UdpSocket::open(dst[1])?;
                Ok(Output::Udp(s))
            },
            _ => {
                Err(Error::from(format!("unknown output type [{}]", dst[0])))
            }
        }
    }

    pub fn send(&self, data: &[u8]) -> Result<()> {
        match self {
            Output::Udp(ref udp) => {
                udp.sendto(data)?;
            },
            Output::None => {},
        };
        Ok(())
    }

    pub fn is_open(&self) -> bool {
        match self {
            Output::None => false,
            _ => true,
        }
    }
}


#[derive(Default, Debug)]
pub struct Instance {
    pub output: Output,
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

    pub delivery: Delivery,

    pub service_list: Vec<Service>
}


#[derive(Default, Debug)]
pub struct Delivery {
    pub frequency: u32,
    pub fec_outer: u8,
    pub modulation: u8,
    pub symbol_rate: u32,
    pub fec: u8
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

            let d = &multiplex.delivery;
            item.descriptors.push(
                psi::Descriptor::Desc44(
                    psi::Desc44 {
                        frequency: d.frequency * 1000000,
                        fec_outer: d.fec_outer,
                        modulation: d.modulation,
                        symbol_rate: d.symbol_rate,
                        fec: d.fec
                    }
                )
            );

            let mut desc_41 = psi::Desc41::default();
            let mut desc_83 = psi::Desc83::default();
            for service in &multiplex.service_list {
                desc_41.items.push(
                    (service.pnr, service.service_type)
                );
                desc_83.items.push(
                    (service.pnr, 1, service.lcn)
                );
            }
            item.descriptors.push(
                psi::Descriptor::Desc41(desc_41)
            );
            item.descriptors.push(
                psi::Descriptor::Desc83(desc_83)
            );

            nit.items.push(item);
        }
    }

    let mut cc = 0;
    let mut ts = Vec::<u8>::new();

    loop {
        nit.demux(psi::NIT_PID, &mut cc, &mut ts);
        let pps = time::Duration::from_nanos(
            1_000_000_000_u64 / (((6 + ts.len() / ts::PACKET_SIZE) / 7) as u64)
        );

        let mut skip = 0;
        while skip < ts.len() {
            let pkt_len = cmp::min(ts.len() - skip, 1316);
            let next = skip + pkt_len;
            instance.output.send(&ts[skip..next]).unwrap();
            thread::sleep(pps);
            skip = next;
        }

        ts.clear();
    }
}


fn main() {
    if let Err(e) = wrap() {
        println!("Error: {}", e.to_string());
    }
}
