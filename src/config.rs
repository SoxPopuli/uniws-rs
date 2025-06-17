use crate::error::Error;
use winnow::{
    ascii::{alphanumeric1, line_ending, multispace0, space0, till_line_ending},
    combinator::{alt, delimited, not, opt, repeat, terminated},
    prelude::*,
    token::{one_of, take_until, take_while},
};

fn header(input: &mut &str) -> ModalResult<String> {
    let allowed_chars = one_of(('a'..='z', 'A'..='Z', '.', '_', '-', '(', ')'));

    '['.parse_next(input)?;
    let res = repeat(1.., allowed_chars).parse_next(input)?;
    ']'.parse_next(input)?;

    Ok(res)
}

fn comment(input: &mut &str) -> ModalResult<()> {
    one_of((';', '#')).void().parse_next(input)?;
    till_line_ending.void().parse_next(input)?;
    line_ending.void().parse_next(input)
}

fn whitespace_or_comment(input: &mut &str) -> ModalResult<()> {
    (opt(multispace0).void(), comment, opt(multispace0).void())
        .void()
        .parse_next(input)
}

#[derive(Debug)]
struct Item {
    key: String,
    value: String,
}

fn kv_pair(input: &mut &str) -> ModalResult<Item> {
    let key = alphanumeric1.parse_next(input)?;

    (space0, '=', space0).void().parse_next(input)?;

    let value = alt((
        delimited('"', not(line_ending).with_taken().map(|(_, a)| a), '"'),
        till_line_ending,
    ))
    .parse_next(input)?;

    Ok(Item {
        key: key.to_string(),
        value: value.to_string(),
    })
}

pub fn parse(mut input: &str) -> ModalResult<()> {
    let input = &mut input;

    take_until(0.., '[').void().parse_next(input)?;

    let header = header.parse_next(input)?;
    whitespace_or_comment.parse_next(input)?;

    let items: Vec<Item> =
        repeat(1.., terminated(kv_pair, whitespace_or_comment)).parse_next(input)?;

    println!("{header}");
    println!("{items:#?}");

    todo!()
}
