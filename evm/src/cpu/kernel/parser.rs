use std::str::FromStr;

use ethereum_types::U256;
use pest::iterators::Pair;
use pest::Parser;

use super::ast::StackPlaceholder;
use crate::cpu::kernel::ast::{File, Item, PushTarget, StackReplacement};

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
        Rule::repeat => parse_repeat(item),
        Rule::stack => parse_stack(item),
        Rule::global_label_decl => {
            Item::GlobalLabelDeclaration(item.into_inner().next().unwrap().as_str().into())
        }
        Rule::local_label_decl => {
            Item::LocalLabelDeclaration(item.into_inner().next().unwrap().as_str().into())
        }
        Rule::macro_label_decl => {
            Item::MacroLabelDeclaration(item.into_inner().next().unwrap().as_str().into())
        }
        Rule::bytes_item => Item::Bytes(item.into_inner().map(parse_literal_u8).collect()),
        Rule::push_instruction => Item::Push(parse_push_target(item.into_inner().next().unwrap())),
        Rule::prover_input_instruction => Item::ProverInput(
            item.into_inner()
                .next()
                .unwrap()
                .into_inner()
                .map(|x| x.as_str().into())
                .collect::<Vec<_>>()
                .into(),
        ),
        Rule::nullary_instruction => Item::StandardOp(item.as_str().to_uppercase()),
        _ => panic!("Unexpected {:?}", item.as_rule()),
    }
}

fn parse_macro_def(item: Pair<Rule>) -> Item {
    assert_eq!(item.as_rule(), Rule::macro_def);
    let mut inner = item.into_inner().peekable();

    let name = inner.next().unwrap().as_str().into();

    // The parameter list is optional.
    let params = if let Some(Rule::paramlist) = inner.peek().map(|pair| pair.as_rule()) {
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

fn parse_repeat(item: Pair<Rule>) -> Item {
    assert_eq!(item.as_rule(), Rule::repeat);
    let mut inner = item.into_inner();
    let count = parse_literal_u256(inner.next().unwrap());
    Item::Repeat(count, inner.map(parse_item).collect())
}

fn parse_stack(item: Pair<Rule>) -> Item {
    assert_eq!(item.as_rule(), Rule::stack);
    let mut inner = item.into_inner();

    let placeholders = inner.next().unwrap();
    assert_eq!(placeholders.as_rule(), Rule::stack_placeholders);
    let replacements = inner.next().unwrap();
    assert_eq!(replacements.as_rule(), Rule::stack_replacements);

    let placeholders = placeholders
        .into_inner()
        .map(parse_stack_placeholder)
        .collect();
    let replacements = replacements
        .into_inner()
        .map(parse_stack_replacement)
        .collect();
    Item::StackManipulation(placeholders, replacements)
}

fn parse_stack_placeholder(target: Pair<Rule>) -> StackPlaceholder {
    assert_eq!(target.as_rule(), Rule::stack_placeholder);
    let inner = target.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::identifier => StackPlaceholder(inner.as_str().into(), 1),
        Rule::stack_block => {
            let mut block = inner.into_inner();
            let identifier = block.next().unwrap().as_str();
            let length = block.next().unwrap().as_str().parse().unwrap();
            StackPlaceholder(identifier.to_string(), length)
        }
        _ => panic!("Unexpected {:?}", inner.as_rule()),
    }
}

fn parse_stack_replacement(target: Pair<Rule>) -> StackReplacement {
    assert_eq!(target.as_rule(), Rule::stack_replacement);
    let inner = target.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::identifier => StackReplacement::Identifier(inner.as_str().into()),
        Rule::literal => StackReplacement::Literal(parse_literal_u256(inner)),
        Rule::macro_label => {
            StackReplacement::MacroLabel(inner.into_inner().next().unwrap().as_str().into())
        }
        Rule::variable => {
            StackReplacement::MacroVar(inner.into_inner().next().unwrap().as_str().into())
        }
        Rule::constant => {
            StackReplacement::Constant(inner.into_inner().next().unwrap().as_str().into())
        }
        _ => panic!("Unexpected {:?}", inner.as_rule()),
    }
}

fn parse_push_target(target: Pair<Rule>) -> PushTarget {
    assert_eq!(target.as_rule(), Rule::push_target);
    let inner = target.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::literal => PushTarget::Literal(parse_literal_u256(inner)),
        Rule::identifier => PushTarget::Label(inner.as_str().into()),
        Rule::macro_label => {
            PushTarget::MacroLabel(inner.into_inner().next().unwrap().as_str().into())
        }
        Rule::variable => PushTarget::MacroVar(inner.into_inner().next().unwrap().as_str().into()),
        Rule::constant => PushTarget::Constant(inner.into_inner().next().unwrap().as_str().into()),
        _ => panic!("Unexpected {:?}", inner.as_rule()),
    }
}

fn parse_literal_u8(literal: Pair<Rule>) -> u8 {
    let literal = literal.into_inner().next().unwrap();
    match literal.as_rule() {
        Rule::literal_decimal => {
            u8::from_str(literal.as_str()).expect("Failed to parse literal decimal byte")
        }
        Rule::literal_hex => {
            u8::from_str_radix(&parse_hex(literal), 16).expect("Failed to parse literal hex byte")
        }
        _ => panic!("Unexpected {:?}", literal.as_rule()),
    }
}

fn parse_literal_u256(literal: Pair<Rule>) -> U256 {
    let literal = literal.into_inner().next().unwrap();
    match literal.as_rule() {
        Rule::literal_decimal => {
            U256::from_dec_str(literal.as_str()).expect("Failed to parse literal decimal")
        }
        Rule::literal_hex => {
            U256::from_str_radix(&parse_hex(literal), 16).expect("Failed to parse literal hex")
        }
        _ => panic!("Unexpected {:?}", literal.as_rule()),
    }
}

fn parse_hex(hex: Pair<Rule>) -> String {
    let prefix = &hex.as_str()[..2];
    debug_assert!(prefix == "0x" || prefix == "0X");
    hex.as_str()[2..].to_string()
}
