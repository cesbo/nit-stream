use std::fs::File;
use std::io::{Read, BufReader};

use ini::{IniReader, IniItem};

use crate::error::{Error, Result};
use crate::{Instance, Multiplex, Service};

fn parse_multiplex<R: Read>(instance: &mut Instance, config: &mut IniReader<R>) -> Result<()> {
    let mut multiplex = Multiplex::default();
    multiplex.codepage = instance.codepage;
    multiplex.network_id = instance.network_id;
    multiplex.nit_version = instance.nit_version;
    multiplex.provider.push_str(&instance.provider);
    multiplex.network.push_str(&instance.network);
    multiplex.onid = instance.onid;
    multiplex.enable = true;

    while let Some(e) = config.next() {
        match e? {
            IniItem::EndSection => break,
            IniItem::Property(key, value) => {
                match key.as_ref() {
                    "codepage" => multiplex.codepage = value.parse()?,
                    "network_id" => multiplex.network_id = value.parse()?,
                    "nit_version" => multiplex.nit_version = value.parse()?,
                    "provider" => { multiplex.provider.clear(); multiplex.provider.push_str(&value); },
                    "network" => { multiplex.network.clear(); multiplex.network.push_str(&value); },
                    "onid" => multiplex.onid = value.parse()?,

                    "enable" => multiplex.enable = value.parse().unwrap_or(false),
                    "name" => multiplex.name.push_str(&value),
                    "tsid" => multiplex.tsid = value.parse()?,
                    _ => {},
                }
            },
            _ => {},
        };
    }

    instance.multiplex_list.push(multiplex);
    Ok(())
}

fn parse_service<R: Read>(instance: &mut Instance, config: &mut IniReader<R>) -> Result<()> {
    let multiplex = match instance.multiplex_list.last_mut() {
        Some(v) => v,
        None => return Err(Error::from("multiplex section not found")),
    };

    let mut service = Service::default();

    while let Some(e) = config.next() {
        match e? {
            IniItem::EndSection => break,
            IniItem::Property(key, value) => {
                match key.as_ref() {
                    "name" => service.name.push_str(&value),
                    "pnr" => service.pnr = value.parse()?,
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

    while let Some(e) = config.next() {
        match e? {
            IniItem::StartSection(name) => match name.as_ref() {
                "multiplex" => parse_multiplex(instance, &mut config)?,
                // TODO: delivery system
                "service" => parse_service(instance, &mut config)?,
                _ => {},
            },
            IniItem::Property(key, value) => match key.as_ref() {
                "codepage" => instance.codepage = value.parse()?,
                _ => {},
            },
            _ => {},
        };
    }

    Ok(())
}
