use anyhow::{Context, Result};
use pest::error::Error;
use pest::iterators::Pair;
use pest::Parser;
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Parser)]
#[grammar = "grammar/plist.pest"]
struct PlistParser;

pub enum Root {
    Dict,
    Array,
}

#[derive(Debug)]
pub struct Slice<'a> {
    pub value: Value<'a>,
    pub code: &'a str,
}

#[derive(Debug)]
pub enum Value<'a> {
    Dict(Vec<(&'a str, Slice<'a>, &'a str)>),
    Array(Vec<Slice<'a>>),
    String(&'a str),
}

pub fn parse(root: Root, code: &str) -> Result<Slice, Error<Rule>> {
    fn parse_string(pair: Pair<Rule>) -> &str {
        match pair.as_rule() {
            //  TODO: handle escape sequences
            Rule::string_quoted => pair.into_inner().next().unwrap().as_str(),
            Rule::string_unquoted => pair.as_str(),
            _ => unreachable!(),
        }
    }

    fn parse_slice(pair: Pair<Rule>) -> Slice {
        let rule = pair.as_rule();
        let mut pairs = pair.into_inner();

        match rule {
            Rule::dict => Slice {
                code: pairs.as_str(),
                value: Value::Dict({
                    pairs
                        .map(|pair| {
                            let code = pair.as_str();
                            let mut inner_rules = pair.into_inner();
                            let key = parse_string(
                                inner_rules.next().unwrap().into_inner().next().unwrap(),
                            );
                            let value = parse_slice(inner_rules.next().unwrap());
                            (key, value, code)
                        })
                        .collect()
                }),
            },
            Rule::array => Slice {
                code: pairs.as_str(),
                value: Value::Array(pairs.map(parse_slice).collect()),
            },
            Rule::string => Slice {
                code: pairs.as_str(),
                value: Value::String(parse_string(pairs.next().unwrap())),
            },
            Rule::value => parse_slice(pairs.next().unwrap()),
            _ => unreachable!(),
        }
    }

    let rule = match root {
        Root::Dict => Rule::dict,
        Root::Array => Rule::array,
    };
    let plist = PlistParser::parse(rule, code)?.next().unwrap();

    Ok(parse_slice(plist))
}

pub fn write_dict_file(path: &Path, codes: &Vec<&str>) -> Result<()> {
    let mut file =
        fs::File::create(&path).with_context(|| format!("cannot create {}", path.display()))?;

    write!(file, "{{\n")?;

    for code in codes {
        write!(file, "{}\n", code)?;
    }

    write!(file, "}}\n")?;

    Ok(())
}

pub fn write_array_file(path: &Path, codes: &Vec<&str>) -> Result<()> {
    let mut file =
        fs::File::create(&path).with_context(|| format!("cannot create {}", path.display()))?;

    write!(file, "(\n")?;

    let mut iter = codes.iter().peekable();

    while let Some(code) = iter.next() {
        if iter.peek().is_some() {
            write!(file, "{},\n", code)?;
        } else {
            write!(file, "{}\n", code)?;
        }
    }

    write!(file, ")")?;

    Ok(())
}
