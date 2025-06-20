use crate::{config::Items, error::Error};
use std::io::{ Write, Read, Seek };

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MatchType {
    Exact,
    Wild,
}

#[derive(Debug)]
pub struct PatchInfo {
    pub modfile: String,
    pub undofile: Option<String>,
    pub sig: Vec<u8>,
    pub sigwild: Vec<MatchType>,
    pub xoffset: Option<u64>,
    pub yoffset: Option<u64>,
    pub occur: u32,

    pub setx: Option<u16>,
    pub sety: Option<u16>,
}
impl PatchInfo {
    pub fn from_items(section: &str, items: &Items, index: Option<u8>) -> Result<Self, Error> {
        struct Field<'a> {
            section: &'a str,
            field_name: &'static str,
            items: &'a Items,
            index: Option<u8>,
        }
        impl<'a> Field<'a> {
            fn get(&self) -> Result<&String, Error> {
                let actual_name: &str = match self.index {
                    Some(prefix) => &format!("p{prefix}{}", self.field_name),
                    None => self.field_name,
                };

                self.items
                    .get(actual_name)
                    .ok_or(Error::config_missing_field(self.section, self.field_name))
            }

            fn parse<T>(&self) -> Result<T, Error>
            where
                T: std::str::FromStr,
                T::Err: std::error::Error,
            {
                self.get().and_then(|x| {
                    x.parse().map_err(|x: T::Err| {
                        Error::config_field_parse(self.section, self.field_name, x.to_string())
                    })
                })
            }
        }

        let field_name = |base_name: &'static str| Field {
            items,
            section,
            field_name: base_name,
            index,
        };

        fn read_sig(section: &str, sig: &str) -> Result<Vec<u8>, Error> {
            (0..sig.len())
                .step_by(2)
                .map(|x| {
                    if x + 1 >= sig.len() {
                        return Err(Error::config_field_parse(
                            section,
                            "sig",
                            "Invalid hex string length".to_string(),
                        ));
                    }

                    let byte_pair = &sig[x..=x + 1];

                    u8::from_str_radix(byte_pair, 16).map_err(|_| {
                        Error::config_field_parse(
                            section,
                            "sig",
                            format!("Invalid hex byte pair: {byte_pair}"),
                        )
                    })
                })
                .collect()
        }

        let sig = { field_name("sig").get().and_then(|x| read_sig(section, x)) }?;

        let sigwild = field_name("sigwild").get().and_then(|sigwild| {
            sigwild
                .chars()
                .map(|c| match c {
                    '0' => Ok(MatchType::Exact),
                    '1' => Ok(MatchType::Wild),
                    x => Err(Error::config_error(format!(
                        "Invalid sigwild character: {x}"
                    ))),
                })
                .collect::<Result<Vec<_>, _>>()
        })?;

        Ok(Self {
            modfile: field_name("modfile").get().cloned()?,
            undofile: field_name("undofile").get().cloned().ok(),
            sig,
            sigwild,
            xoffset: field_name("xoffset").parse().ok(),
            yoffset: field_name("yoffset").parse().ok(),
            occur: field_name("occur").parse()?,
            setx: field_name("setx").parse().ok(),
            sety: field_name("sety").parse().ok(),
        })
    }

    pub fn apply_patch<T>(&self, file: T) where T: Read + Write + Seek {

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn parse_multiple_patches() {
        let section = "test";
        let items = HashMap::from_iter(
            [
                ("details", ""),
                ("checkfile", "swkotor.exe"),
                ("modfile", "swkotor.exe"),
                ("undofile", "swkotora.undo1"),
                ("sig", "3D20030000EFEFEFEFEFEF58020000"),
                ("sigwild", "000001111110000"),
                ("xoffset", "1"),
                ("yoffset", "11"),
                ("occur", "1"),
                ("p1modfile", "swkotor.exe"),
                ("p1undofile", "swkotora.undo2"),
                (
                    "p1sig",
                    "3D00040000B329EFEFEFEFEFEFEFEFEF3D00050000EFEF3D40060000",
                ),
                ("p1sigwild", "0000000111111111000001100000"),
                ("p1xoffset", "1"),
                ("p1occur", "1"),
                ("p1setx", "0"),
                ("p2modfile", "swkotor.exe"),
                ("p2undofile", "swkotora.undo3"),
                (
                    "p2sig",
                    "3D00040000B329EFEFEFEFEFEFEFEFEF3D00050000EFEF3D40060000",
                ),
                ("p2sigwild", "0000000111111111000001100000"),
                ("p2xoffset", "17"),
                ("p2occur", "1"),
                ("p2setx", "0"),
                ("p3modfile", "swkotor.exe"),
                ("p3undofile", "swkotora.undo4"),
                (
                    "p3sig",
                    "3D00040000B329EFEFEFEFEFEFEFEFEF3D00050000EFEF3D40060000",
                ),
                ("p3sigwild", "0000000111111111000001100000"),
                ("p3xoffset", "24"),
                ("p3occur", "1"),
                ("p3setx", "0"),
                ("p4modfile", "swkotor.exe"),
                ("p4undofile", "swkotorc.undom1"),
                ("p4sig", "800200007515813DD8D17800E001"),
                ("p4sigwild", "00000000000000"),
                ("p4xoffset", "0"),
                ("p4yoffset", "12"),
                ("p4occur", "1"),
                ("p5modfile", "swkotor.exe"),
                ("p5undofile", "swkotorc.undom2"),
                ("p5sig", "80020000C7442410E001"),
                ("p5sigwild", "0000000000"),
                ("p5xoffset", "0"),
                ("p5yoffset", "8"),
                ("p5occur", "1"),
            ]
            .map(|(a, b)| (a.to_string(), b.to_string())),
        );

        PatchInfo::from_items(section, &items, None).unwrap();
        PatchInfo::from_items(section, &items, Some(1)).unwrap();
        PatchInfo::from_items(section, &items, Some(2)).unwrap();
        PatchInfo::from_items(section, &items, Some(3)).unwrap();
        PatchInfo::from_items(section, &items, Some(4)).unwrap();
        PatchInfo::from_items(section, &items, Some(5)).unwrap();
    }
}
