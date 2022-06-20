use pest::iterators::Pair;
use pest::Parser;

use crate::cpu::kernel::ast::{File, Item, Literal, PushTarget};

/// Parses EVM assembly code.
#[derive(pest_derive::Parser)]
#[grammar = "cpu/kernel/evm_asm.pest"]
pub struct AsmParser;

pub(crate) fn parse(s: &str) -> File {
    let file = AsmParser::parse(Rule::file, s)
        .expect("Parsing failed")
        .next()
        .unwrap();
    let body = file.into_inner().map(parse_item).collect();
    File { body }
}

fn parse_item(item: Pair<Rule>) -> Item {
    let item = item.into_inner().next().unwrap();
    match item.as_rule() {
        Rule::global_label => {
            Item::GlobalLabelDeclaration(item.into_inner().next().unwrap().as_str().into())
        }
        Rule::local_label => {
            Item::LocalLabelDeclaration(item.into_inner().next().unwrap().as_str().into())
        }
        Rule::bytes_item => Item::Bytes(item.into_inner().map(parse_literal).collect()),
        Rule::push_instruction => Item::Push(parse_push_target(item.into_inner().next().unwrap())),
        Rule::nullary_instruction => Item::StandardOp(item.as_str().into()),
        _ => panic!("Unexpected {:?}", item.as_rule()),
    }
}

fn parse_push_target(target: Pair<Rule>) -> PushTarget {
    match target.as_rule() {
        Rule::identifier => PushTarget::Label(target.as_str().into()),
        Rule::literal => PushTarget::Literal(parse_literal(target)),
        _ => panic!("Unexpected {:?}", target.as_rule()),
    }
}

fn parse_literal(literal: Pair<Rule>) -> Literal {
    let literal = literal.into_inner().next().unwrap();
    match literal.as_rule() {
        Rule::literal_decimal => Literal::Decimal(literal.as_str().into()),
        Rule::literal_hex => Literal::Hex(parse_hex(literal)),
        _ => panic!("Unexpected {:?}", literal.as_rule()),
    }
}

fn parse_hex(hex: Pair<Rule>) -> String {
    let prefix = &hex.as_str()[..2];
    debug_assert!(prefix == "0x" || prefix == "0X");
    hex.as_str()[2..].to_string()
}
