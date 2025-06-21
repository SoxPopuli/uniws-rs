use crate::{
    config::Items,
    error::Error,
    signature::{MatchType, Signature},
};
use std::io::{Read, Seek, Write};

#[derive(Debug, Default)]
pub struct PatchInfo {
    pub modfile: String,
    pub undofile: Option<String>,
    pub signature: Signature,
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

        let sig = field_name("sig");
        let sigwild = field_name("sigwild");

        let signature = Signature::from_string(section, sig.get()?, sigwild.get()?)?;

        Ok(Self {
            signature,
            modfile: field_name("modfile").get().cloned()?,
            undofile: field_name("undofile").get().cloned().ok(),
            xoffset: field_name("xoffset").parse().ok(),
            yoffset: field_name("yoffset").parse().ok(),
            occur: field_name("occur").parse()?,
            setx: field_name("setx").parse().ok(),
            sety: field_name("sety").parse().ok(),
        })
    }

    /// Returns `true` if applied successfully
    #[must_use = "Should handle failure case"]
    pub fn apply_patch(&self, data: &mut [u8], x_res: u16, y_res: u16) -> bool {
        let mut data = data;

        for _ in 0..self.occur {
            match self.signature.try_find(data) {
                Some(index) => {
                    let x_bytes = x_res.to_le_bytes();
                    let y_bytes = y_res.to_le_bytes();
                    println!(
                        "x: [{:0x}, {:0x}], y: [{:0x}, {:0x}]",
                        x_bytes[0], x_bytes[1], y_bytes[0], y_bytes[1],
                    );

                    if let Some(xoffset) = self.xoffset {
                        let x_offset = index + xoffset as usize;

                        data[x_offset] = x_bytes[0];
                        data[x_offset + 1] = x_bytes[1];
                    }

                    if let Some(yoffset) = self.yoffset {
                        let y_offset = index + yoffset as usize;

                        data[y_offset] = y_bytes[0];
                        data[y_offset + 1] = y_bytes[1];
                    }

                    data = &mut data[index + self.signature.pattern.len()..]
                }
                None => return false,
            }
        }

        true
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

    #[test]
    fn apply_test() {
        let info = PatchInfo {
            signature: Signature::from_string("test", "80020000C701E0010000", "0000110000")
                .unwrap(),
            xoffset: Some(0),
            yoffset: Some(6),
            occur: 2,
            ..Default::default()
        };

        #[rustfmt::skip]
        let mut data = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,

            0x80, 0x02, 0x00, 0x00, 0xC7, 0x01, 0xE0, 0x01, 0x00, 0x00,

            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,

            0x80, 0x02, 0x00, 0x00, 0xC7, 0x01, 0xE0, 0x01, 0x00, 0x00,
        ];

        assert!(info.apply_patch(&mut data, 1920, 1080));

        #[rustfmt::skip]
        assert_eq!(data.as_slice(), [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,

            0x80, 0x07, 0x00, 0x00, 0xC7, 0x01, 0x38, 0x04, 0x00, 0x00,

            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,

            0x80, 0x07, 0x00, 0x00, 0xC7, 0x01, 0x38, 0x04, 0x00, 0x00,
        ]);
    }
}
