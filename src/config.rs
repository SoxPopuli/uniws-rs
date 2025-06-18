use std::collections::HashMap;
use winnow::{
    ascii::{alphanumeric1, line_ending, multispace1, space0, till_line_ending},
    combinator::{alt, delimited, opt, repeat, terminated},
    prelude::*,
    token::{one_of, take_until},
};

fn header(input: &mut &str) -> ModalResult<String> {
    // let allowed_chars = one_of(('a'..='z', 'A'..='Z', ' ', '.', '_', '-', '(', ')'));

    '['.parse_next(input)?;
    // let res = repeat(1.., allowed_chars).parse_next(input)?;
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

pub fn parse(mut input: &str) -> ModalResult<Vec<Section>> {
    let input = &mut input;

    take_until(0.., '[').void().parse_next(input)?;

    fn parse_section(input: &mut &str) -> ModalResult<Section> {
        let header = header.parse_next(input)?;
        whitespace_and_comments.parse_next(input)?;

        let items: HashMap<String, String> =
            repeat(1.., terminated(kv_pair, whitespace_and_comments)).parse_next(input)?;

        Ok(Section {
            name: header,
            items,
        })
    }

    repeat(1.., parse_section).parse_next(input)
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

        let expected = vec![Section {
            name: "Apps".to_string(),
            items: items_map([
                ("version", "1.0"),
                ("a0", "One"),
                ("a1", "Two"),
                ("a2", "Three"),
            ]),
        }];

        assert_eq!(parse(file), Ok(expected))
    }
}
