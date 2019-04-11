use std::{
    env,
    cmp,
    time,
    thread,
};

use udp::UdpSocket;
use mpegts::{
    ts,
    psi,
    textcode,
    psi::PsiDemux,
    constants,
};

mod error;
use crate::error::{
    Error,
    Result,
};

use config::{
    Config,
    Schema,
};


include!(concat!(env!("OUT_DIR"), "/build.rs"));


fn version() {
    println!("nit-stream v.{} commit:{}", env!("CARGO_PKG_VERSION"), COMMIT);
}


fn usage(program: &str) {
    println!(r#"Usage: {} CONFIG

OPTIONS:
    -v, --version       Version information
    -h, --help          Print this text
    -H                  Configuration file format

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
    let codepage_validator = |s: &str| -> bool {
        let v = s.parse::<usize>().unwrap_or(1000);
        ((v <= 11) || (v >= 13 && v <= 15) || (v == 21))
    };

    // Schema
    let mut schema_service = Schema::new("service",
        "Service configuration. Multiplex contains one or more services");
    schema_service.set("pnr",
        "Program Number. Required. Should be in range 1 .. 65535",
        true, Schema::range(1 .. 65535));
    schema_service.set("type",
        "Default: 1. Range: 0 .. 255. Available values:\n\
        ; 1 - Digital Television service\n\
        ; 2 - Digital Radio service\n\
        ; 3 - Teletext service\n\
        ; More information available in EN 300 468 (Table 61: Service type coding)",
        false, Schema::range(0 .. 255));
    schema_service.set("lcn",
        "Logical Channel Number. Default: 0 - not set",
        false, Schema::range(0 .. 1000));

    let mut schema_dvb_c = Schema::new("dvb-c",
        "Options for DVB-C delivery system");
    schema_dvb_c.set("frequency",
        "Frequency in MHz. Required",
        true, None);
    schema_dvb_c.set("modulation",
        "Modulation scheme. Default: not set. Available values:\n\
        ; QAM16, QAM32, QAM64, QAM128, QAM256",
        false, None);
    schema_dvb_c.set("symbolrate",
        "Symbolrate in Msymbol/s. Required",
        true, None);
    schema_dvb_c.set("fec",
        "Inner FEC scheme. Default: not set. Range: 0 .. 15. Available values:\n\
        ; 1 - 1/2\n\
        ; 2 - 2/3\n\
        ; 3 - 3/4\n\
        ; 4 - 5/6\n\
        ; 5 - 7/8",
        false, Schema::range(0 .. 15));

    let mut schema_multiplex = Schema::new("multiplex",
        "Multiplex configuration. App contains one or more multiplexes");
    schema_multiplex.set("tsid",
        "Transport Stream Identifier. Required. Range: 1 .. 65535",
        true, Schema::range(1 .. 65535));
    schema_multiplex.set("onid",
        "Redefine Original Network Identifier for multiplex. Range: 1 .. 65535",
        false, Schema::range(1 .. 65535));
    schema_multiplex.push(schema_dvb_c);
    schema_multiplex.push(schema_service);

    let mut schema = Schema::new("",
        "nit-stream - MPEG-TS NIT (Network Information Table) streamer\n");
    // TODO: udp address validator
    schema.set("output",
        "UDP Address. Requried. Example: udp://239.255.1.1:10000",
        true, None);
    schema.set("nit_version",
        "Table version. Default: 0. Range: 0 .. 31",
        false, Schema::range(0 .. 31));
    schema.set("network_id",
        "Unique network identifier. Default: 1. Range: 0 .. 65535",
        false, Schema::range(0 .. 65535));
    schema.set("network",
        "Network name. Default: not set",
        false, None);
    schema.set("onid",
        "Original Network Identifier. Default: 1. Range 1 .. 65535",
        false, Schema::range(1 .. 65535));
    schema.set("codepage",
        "Codepage for network name. Default: 0 - Latin (ISO 6937). Available values:\n\
        ; 1 - Western European (ISO 8859-1)\n\
        ; 2 - Central European (ISO 8859-2)\n\
        ; 3 - South European (ISO 8859-3)\n\
        ; 4 - North European (ISO 8859-4)\n\
        ; 5 - Cyrillic (ISO 8859-5)\n\
        ; 6 - Arabic (ISO 8859-6)\n\
        ; 7 - Greek (ISO 8859-7)\n\
        ; 8 - Hebrew (ISO 8859-8)\n\
        ; 9 - Turkish (ISO 8859-9)\n\
        ; 10 - Nordic (ISO 8859-10)\n\
        ; 11 - Thai (ISO 8859-11)\n\
        ; 13 - Baltic Rim (ISO 8859-13)\n\
        ; 14 - Celtic (ISO 8859-14)\n\
        ; 15 - Western European (ISO 8859-15)\n\
        ; 21 - UTF-8",
        false, codepage_validator);
    schema.push(schema_multiplex);

    // Parse Options
    let mut args = env::args();
    let program = args.next().unwrap();
    let arg = match args.next() {
        Some(v) => match v.as_ref() {
            "-v" | "--version" => {
                version();
                return Ok(());
            },
            "-h" | "--help" => {
                usage(&program);
                return Ok(());
            },
            "-H" => {
                println!("Configuration file format:\n\n{}", &schema.info());
                return Ok(());
            },
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
    schema.check(&config)?;

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
