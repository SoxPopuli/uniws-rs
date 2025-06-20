use crate::error::{ConfigError, Error};
use std::collections::HashMap;
use winnow::{
    ascii::{alphanumeric1, line_ending, multispace1, space0, till_line_ending},
    combinator::{alt, delimited, opt, repeat, terminated},
    prelude::*,
    token::{one_of, take_until},
};

fn header(input: &mut &str) -> ModalResult<String> {
    '['.parse_next(input)?;
    let res = take_until(1.., ']').parse_next(input)?;
    ']'.parse_next(input)?;

    Ok(res.to_string())
}

fn comment(input: &mut &str) -> ModalResult<()> {
    one_of((';', '#')).void().parse_next(input)?;
    till_line_ending.void().parse_next(input)?;
    line_ending.void().parse_next(input)
}

fn whitespace_and_comments(input: &mut &str) -> ModalResult<()> {
    fn ws(input: &mut &str) -> ModalResult<()> {
        alt((comment, multispace1.void())).void().parse_next(input)
    }

    fn repeat_ws(input: &mut &str) -> ModalResult<()> {
        repeat(1.., ws).map(|()| ()).parse_next(input)
    }

    opt(repeat_ws).void().parse_next(input)
}

pub fn kv_pair(input: &mut &str) -> ModalResult<(String, String)> {
    let key = alphanumeric1.parse_next(input)?;

    (space0, '=', space0).void().parse_next(input)?;

    let quoted = (delimited('"', take_until(0.., '"'), '"'), line_ending).map(|(a, _)| a);

    let value = alt((quoted, till_line_ending.map(|s: &str| s.trim_end()))).parse_next(input)?;

    Ok((key.to_string(), value.to_string()))
}

#[derive(Debug, PartialEq, Eq)]
pub struct Section {
    pub name: String,
    pub items: HashMap<String, String>,
}

type Items = HashMap<String, String>;
type RawConfig = HashMap<String, Items>;

fn parse(mut input: &str) -> ModalResult<RawConfig> {
    let input = &mut input;

    take_until(0.., '[').void().parse_next(input)?;

    fn parse_section(input: &mut &str) -> ModalResult<(String, Items)> {
        let header = header.parse_next(input)?;
        whitespace_and_comments.parse_next(input)?;

        let items: HashMap<String, String> =
            repeat(1.., terminated(kv_pair, whitespace_and_comments)).parse_next(input)?;

        Ok((header, items))
    }

    repeat(1.., parse_section).parse_next(input)
}

#[derive(Debug)]
pub struct Apps {
    pub version: String,
    pub apps: Vec<String>,
}

#[derive(Debug)]
pub struct PatchInfo {
    pub modfile: String,
    pub undofile: Option<String>,
    pub sig: Vec<u8>,
    pub sigwild: Vec<bool>,
    pub xoffset: Option<u64>,
    pub yoffset: Option<u64>,
    pub occur: u32,

    pub setx: Option<u16>,
    pub sety: Option<u16>,
}
impl PatchInfo {
    fn from_items(section: &str, items: &Items, index: Option<u8>) -> Result<Self, Error> {
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
                    '0' => Ok(false),
                    '1' => Ok(true),
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
}

#[cfg(windows)]
const LINE_ENDING: &str = "\r\n";
#[cfg(not(windows))]
const LINE_ENDING: &str = "\n";

#[derive(Debug)]
pub struct AppSection {
    pub name: String,
    pub details: String,
    pub checkfile: String,
    pub patches: Vec<PatchInfo>,
}
impl AppSection {
    fn from_items(name: impl Into<String>, items: &Items) -> Result<Self, Error> {
        let name: String = name.into();
        let details = items
            .get("details")
            .cloned()
            .map(|details| details.replace(r#"\013\010"#, LINE_ENDING))
            .ok_or(Error::config_missing_field(name.clone(), "details"))?;
        let checkfile = items
            .get("checkfile")
            .cloned()
            .ok_or(Error::config_missing_field(name.clone(), "checkfile"))?;

        let first = PatchInfo::from_items(&name, items, None)?;
        let mut patches = vec![first];

        let mut idx = 1;
        loop {
            match PatchInfo::from_items(&name, items, Some(idx)) {
                Ok(next) => {
                    patches.push(next);
                }
                Err(Error::ConfigError(ConfigError::MissingRequiredField { .. })) => break,
                Err(e) => return Err(e),
            }
            idx += 1;
        }

        Ok(Self {
            name,
            details,
            checkfile,
            patches,
        })
    }
}

#[derive(Debug)]
pub struct Config {
    pub apps: Apps,
    pub sections: Vec<AppSection>,
}
impl Config {
    fn get_apps(raw_config: &RawConfig) -> Result<Apps, Error> {
        let apps = raw_config
            .get("Apps")
            .ok_or(Error::config_error("Missing 'Apps' Section"))?;

        let version = apps
            .get("version")
            .ok_or(Error::config_error("Missing 'Apps.version'"))?;

        let mut apps = apps
            .iter()
            .filter_map(|(k, v)| {
                let (first, rest) = k.split_at_checked(1)?;

                let rest = rest.parse::<u8>().ok()?;

                if first == "a" { Some((rest, v)) } else { None }
            })
            .collect::<Vec<_>>();

        apps.sort_by_key(|x| x.0);

        let apps = apps.into_iter().map(|x| x.1).cloned().collect::<Vec<_>>();

        Ok(Apps {
            version: version.clone(),
            apps,
        })
    }

    pub fn new(input: &str) -> Result<Self, Error> {
        let raw_config: RawConfig = parse(input)?;
        let apps = Self::get_apps(&raw_config)?;

        let mut sections = Vec::with_capacity(apps.apps.len());
        for header in &apps.apps {
            let section = raw_config
                .get(header)
                .ok_or(Error::config_error(format!("Missing section {header}")))?;

            let section = AppSection::from_items(header, section)?;
            sections.push(section);
        }

        Ok(Self { apps, sections })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn items_map<T, U>(items: T) -> HashMap<String, String>
    where
        T: IntoIterator<Item = (U, U)>,
        U: Into<String>,
    {
        items
            .into_iter()
            .map(|(a, b)| (a.into(), b.into()))
            .collect()
    }

    #[test]
    fn whitespace_test() {
        let mut file = r#"
            ; Comment

            ;Comment
            ;Comment
            ;Comment




            ; Comment      c
        "#;

        whitespace_and_comments
            .parse_next(&mut file)
            .expect("Whitespace parse failed");
        assert_eq!(file, "");
    }

    #[test]
    fn parse_test() {
        let file = r#"
            ; Comment A
            ; Comment B
            [Apps]
            version = 1.0

            ; Comment
            a0 = One
            a1=Two
            a2="Three"
        "#;

        let expected = HashMap::from_iter([(
            "Apps".to_string(),
            items_map([
                ("version", "1.0"),
                ("a0", "One"),
                ("a1", "Two"),
                ("a2", "Three"),
            ]),
        )]);

        assert_eq!(parse(file), Ok(expected))
    }

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
