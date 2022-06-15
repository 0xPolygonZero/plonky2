use pest::iterators::Pair;
use pest::Parser;

use crate::cpu::kernel::ast::{Function, HexStr, Item, Literal, PushTarget};

/// Parses EVM assembly code.
#[derive(pest_derive::Parser)]
#[grammar = "cpu/kernel/evm_asm.pest"]
pub struct AsmParser;

pub(crate) fn parse(s: &str) -> Vec<Function> {
    let file = AsmParser::parse(Rule::file, s)
        .expect("Parsing failed")
        .next()
        .unwrap();
    file.into_inner().map(parse_function).collect()
}

fn parse_function(function: Pair<Rule>) -> Function {
    let mut function = function.into_inner();
    let name = function.next().unwrap().as_str().into();
    let body = function
        .next()
        .unwrap()
        .into_inner()
        .map(parse_item)
        .collect();
    Function { name, body }
}

fn parse_item(item: Pair<Rule>) -> Item {
    let item = item.into_inner().next().unwrap();
    match item.as_rule() {
        Rule::label => Item::LabelDeclaration(item.into_inner().next().unwrap().as_str().into()),
        Rule::literal_item => Item::Literal(parse_hex(item)),
        Rule::push_instruction => Item::Push(parse_push_target(item.into_inner().next().unwrap())),
        Rule::nullary_instruction => Item::StandardOp(item.as_str().into()),
        _ => panic!("Unexpected {:?}", item.as_rule()),
    }
}

fn parse_push_target(target: Pair<Rule>) -> PushTarget {
    match target.as_rule() {
        Rule::identifier => PushTarget::Label(target.as_str().into()),
        Rule::literal => PushTarget::Literal(parse_literal(target.into_inner().next().unwrap())),
        _ => panic!("Unexpected {:?}", target.as_rule()),
    }
}

fn parse_literal(literal: Pair<Rule>) -> Literal {
    match literal.as_rule() {
        Rule::literal_decimal => Literal::Decimal(literal.as_str().into()),
        Rule::literal_hex => Literal::Hex(parse_hex(literal)),
        _ => panic!("Unexpected {:?}", literal.as_rule()),
    }
}

fn parse_hex(hex: Pair<Rule>) -> HexStr {
    debug_assert_eq!(&hex.as_str()[..2], "0x");
    let nibbles = hex.as_str()[2..].to_string();
    HexStr { nibbles }
}
