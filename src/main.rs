use std::{env, cmp, time, thread};

use udp::UdpSocket;
use mpegts::{ts, psi, textcode, psi::PsiDemux, constants};

mod error;
use crate::error::{Error, Result};

use config::Config;


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
    Udp(UdpSocket),
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

    let mut output;

    // Parse config
    let config = Config::open(&arg)?;

    let mut nit = psi::Nit::default();
    nit.table_id = 0x40;

    match config.get_str("output") {
        Some(v) => output = Output::open(v)?,
        None => return Err(Error::from("output not defined")),
    };

    nit.version = config.get("nit_version", 0)?;
    nit.network_id = config.get("network_id", 1)?;

    let onid = config.get("onid", 1)?;
    let codepage = config.get("codepage", 0)?;

    if let Some(v) = config.get_str("network") {
        nit.descriptors.push(
            psi::Desc40 {
                name: textcode::StringDVB::from_str(v, codepage)
            }
        );
    }

    for s in config.iter() {
        if s.get_name() != "multiplex" || false == s.get("enable", true)? {
            continue;
        }

        let mut item = psi::NitItem::default();
        item.tsid = s.get("tsid", 1)?;
        item.onid = s.get("onid", onid)?;

        let mut desc_41 = psi::Desc41::default();
        let mut desc_83 = psi::Desc83::default();

        for s in s.iter() {
            match s.get_name() {
                "dvb-c" => {
                    item.descriptors.push(
                        psi::Desc44 {
                            frequency: s.get("frequency", 0)? * 1_000_000,
                            fec_outer: 0,
                            modulation: match s.get_str("modulation").unwrap_or("") {
                                "QAM16" => constants::MODULATION_DVB_C_16_QAM,
                                "QAM32" => constants::MODULATION_DVB_C_32_QAM,
                                "QAM64" => constants::MODULATION_DVB_C_64_QAM,
                                "QAM128" => constants::MODULATION_DVB_C_128_QAM,
                                "QAM256" => constants::MODULATION_DVB_C_256_QAM,
                                _ => constants::MODULATION_DVB_C_NOT_DEFINED
                            },
                            symbol_rate: s.get("symbolrate", 0)?,
                            fec: s.get("fec", 0)?,
                        }
                    );
                },
                "service" => {
                    let pnr: u16 = s.get("pnr", 0)?;
                    let service_type: u8 = s.get("type", 1)?;
                    let lcn: u16 = s.get("lcn", 0)?;

                    desc_41.items.push((pnr, service_type));
                    desc_83.items.push((pnr, 1, lcn));
                },
                _ => {},
            }
        }

        if ! desc_41.items.is_empty() {
            item.descriptors.push(desc_41);
        }

        if ! desc_83.items.is_empty() {
            item.descriptors.push(desc_83);
        }

        nit.items.push(item);
    }

    nit.items.sort_by(|a, b| a.tsid.cmp(&b.tsid));

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
            output.send(&ts[skip..next]).unwrap();
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
