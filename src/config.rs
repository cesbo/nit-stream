use std::fs::File;
use std::io::{Read, BufReader};

use ini::{IniReader, IniItem};
use mpegts::constants;

use crate::{Instance, Multiplex, Delivery, Service, Output};
use crate::misc::Parse;
use crate::error::{Error, Result};


fn parse_multiplex<R: Read>(instance: &mut Instance, config: &mut IniReader<R>) -> Result<()> {
    let mut multiplex = Multiplex::default();
    multiplex.onid = instance.onid;
    multiplex.enable = true;

    while let Some(e) = config.next() {
        match e? {
            IniItem::EndSection => break,
            IniItem::Property(key, value) => {
                match key.as_ref() {
                    "tsid" => multiplex.tsid = value.parse()?,
                    "onid" => multiplex.onid = value.parse()?,
                    "enable" => multiplex.enable = bool::int_parse(value),
                    _ => {},
                }
            },
            _ => {},
        };
    }

    instance.multiplex_list.push(multiplex);
    Ok(())
}


fn parse_delivery<R: Read>(instance: &mut Instance, config: &mut IniReader<R>) -> Result<()> {
    let multiplex = match instance.multiplex_list.last_mut() {
        Some(v) => v,
        None => return Err(Error::from("multiplex section not found")),
    };
    let mut delivery = Delivery::default();

    while let Some(e) = config.next() {
        match e? {
            IniItem::EndSection => break,
            IniItem::Property(key, value) => {
                match key.as_ref() {
                    "frequency" => delivery.frequency = value.parse()?,
                    "symbolrate" => delivery.symbol_rate = value.parse()?,
                    "fec" => delivery.fec = value.parse()?,
                    "modulation" => delivery.modulation = match value.as_str() {
                        "QAM16" => constants::MODULATION_DVB_C_16_QAM,
                        "QAM32" => constants::MODULATION_DVB_C_32_QAM,
                        "QAM64" => constants::MODULATION_DVB_C_64_QAM,
                        "QAM128" => constants::MODULATION_DVB_C_128_QAM,
                        "QAM256" => constants::MODULATION_DVB_C_256_QAM,
                        _ => constants::MODULATION_DVB_C_NOT_DEFINED
                    },
                    _ => {},
                }
            },
            _ => {},
        };
    }

    multiplex.delivery = delivery;
    Ok(())
}


fn parse_service<R: Read>(instance: &mut Instance, config: &mut IniReader<R>) -> Result<()> {
    let multiplex = match instance.multiplex_list.last_mut() {
        Some(v) => v,
        None => return Err(Error::from("multiplex section not found")),
    };

    let mut service = Service::default();
    service.service_type = 1;

    while let Some(e) = config.next() {
        match e? {
            IniItem::EndSection => break,
            IniItem::Property(key, value) => {
                match key.as_ref() {
                    "type" => service.service_type = value.parse()?,
                    "pnr" => service.pnr = value.parse()?,
                    "lcn" => service.lcn = value.parse()?,
                    "name" => service.name.push_str(&value),
                    _ => {},
                }
            },
            _ => {},
        };
    }

    multiplex.service_list.push(service);
    Ok(())
}


pub fn parse_config(instance: &mut Instance, path: &str) -> Result<()> {
    let config = File::open(path)?;
    let mut config = IniReader::new(BufReader::new(config));

    instance.nit_version = 0;
    instance.network_id = 1;
    instance.onid = 1;

    while let Some(e) = config.next() {
        match e? {
            IniItem::StartSection(name) => match name.as_ref() {
                "multiplex" => parse_multiplex(instance, &mut config)?,
                "dvb-c" => parse_delivery(instance, &mut config)?,
                "service" => parse_service(instance, &mut config)?,
                _ => {},
            },
            IniItem::Property(key, value) => match key.as_ref() {
                "output" => instance.output = Output::open(addr)?,
                "nit_version" => instance.nit_version = value.parse()?,
                "network_id" => instance.network_id = value.parse()?,
                "network" => instance.network.push_str(&value),
                "codepage" => instance.codepage = value.parse()?,
                "onid" => instance.onid = value.parse()?,
                _ => {},
            },
            _ => {},
        };
    }

    Ok(())
}
