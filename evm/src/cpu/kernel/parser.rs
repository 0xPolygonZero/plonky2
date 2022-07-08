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
    assert_eq!(item.as_rule(), Rule::item);
    let item = item.into_inner().next().unwrap();
    match item.as_rule() {
        Rule::macro_def => parse_macro_def(item),
        Rule::macro_call => parse_macro_call(item),
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

fn parse_macro_def(item: Pair<Rule>) -> Item {
    assert_eq!(item.as_rule(), Rule::macro_def);
    let mut inner = item.into_inner().peekable();

    let name = inner.next().unwrap().as_str().into();

    // The parameter list is optional.
    let params = if let Some(Rule::macro_paramlist) = inner.peek().map(|pair| pair.as_rule()) {
        let params = inner.next().unwrap().into_inner();
        params.map(|param| param.as_str().to_string()).collect()
    } else {
        vec![]
    };

    Item::MacroDef(name, params, inner.map(parse_item).collect())
}

fn parse_macro_call(item: Pair<Rule>) -> Item {
    assert_eq!(item.as_rule(), Rule::macro_call);
    let mut inner = item.into_inner();

    let name = inner.next().unwrap().as_str().into();

    // The arg list is optional.
    let args = if let Some(arglist) = inner.next() {
        assert_eq!(arglist.as_rule(), Rule::macro_arglist);
        arglist.into_inner().map(parse_push_target).collect()
    } else {
        vec![]
    };

    Item::MacroCall(name, args)
}

fn parse_push_target(target: Pair<Rule>) -> PushTarget {
    assert_eq!(target.as_rule(), Rule::push_target);
    let inner = target.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::literal => PushTarget::Literal(parse_literal(inner)),
        Rule::identifier => PushTarget::Label(inner.as_str().into()),
        Rule::variable => PushTarget::MacroVar(inner.into_inner().next().unwrap().as_str().into()),
        Rule::constant => PushTarget::Constant(inner.into_inner().next().unwrap().as_str().into()),
        _ => panic!("Unexpected {:?}", inner.as_rule()),
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
