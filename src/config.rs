use crate::{
    error::{ConfigError, Error},
    patch_info::PatchInfo,
};
use std::collections::HashMap;
use winnow::{
    ascii::{alphanumeric1, line_ending, multispace1, space0, till_line_ending},
    combinator::{alt, delimited, opt, repeat, terminated},
    prelude::*,
    token::{one_of, take_till, take_until},
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

    let unquoted = take_till(1.., (';', '#', '\r', '\n')).map(|x: &str| x.trim_end());

    let value = alt((quoted, unquoted)).parse_next(input)?;

    Ok((key.to_string(), value.to_string()))
}

#[derive(Debug, PartialEq, Eq)]
pub struct Section {
    pub name: String,
    pub items: HashMap<String, String>,
}

pub type Items = HashMap<String, String>;
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
    use super::*;
    use pretty_assertions::assert_eq;
    use std::collections::HashMap;

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
            version = 1.0 ; End of line comment

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
}
