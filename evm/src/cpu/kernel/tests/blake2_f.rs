use anyhow::Result;

use crate::cpu::kernel::interpreter::{
    run_interpreter_with_memory, InterpreterMemoryInitialization,
};
use crate::memory::segments::Segment::KernelGeneral;

type ConvertedBlakeInputs = (u32, [u64; 8], [u64; 16], u64, u64, bool);

fn reverse_bytes_u64(input: u64) -> u64 {
    let mut result = 0;
    for i in 0..8 {
        result |= ((input >> (i * 8)) & 0xff) << ((7 - i) * 8);
    }
    result
}

fn convert_input(input: &str) -> Result<ConvertedBlakeInputs> {
    let rounds = u32::from_str_radix(&input[..8], 16).unwrap();

    let mut h = [0u64; 8];
    for i in 0..8 {
        h[i] = reverse_bytes_u64(
            u64::from_str_radix(&input[8 + i * 16..8 + (i + 1) * 16], 16).unwrap(),
        );
    }

    let mut m = [0u64; 16];
    for i in 0..16 {
        m[i] = reverse_bytes_u64(
            u64::from_str_radix(&input[136 + i * 16..136 + (i + 1) * 16], 16).unwrap(),
        );
    }

    let t_0 = reverse_bytes_u64(u64::from_str_radix(&input[392..408], 16).unwrap());
    let t_1 = reverse_bytes_u64(u64::from_str_radix(&input[408..424], 16).unwrap());
    let flag = u8::from_str_radix(&input[424..426], 16).unwrap() != 0;

    Ok((rounds, h, m, t_0, t_1, flag))
}

fn convert_output(output: [u64; 8]) -> String {
    output
        .iter()
        .map(|&x| format!("{:016x}", reverse_bytes_u64(x)))
        .collect::<Vec<_>>()
        .join("")
}

fn run_blake2_f(
    rounds: u32,
    h: [u64; 8],
    m: [u64; 16],
    t_0: u64,
    t_1: u64,
    flag: bool,
) -> Result<[u64; 8]> {
    let mut stack = vec![];
    stack.push(rounds.into());
    stack.append(&mut h.iter().map(|&x| x.into()).collect());
    stack.append(&mut m.iter().map(|&x| x.into()).collect());
    stack.push(t_0.into());
    stack.push(t_1.into());
    stack.push(u8::from(flag).into());
    stack.push(0xDEADBEEFu32.into());

    let interpreter_setup = InterpreterMemoryInitialization {
        label: "blake2_f".to_string(),
        stack,
        segment: KernelGeneral,
        memory: vec![],
    };

    let result = run_interpreter_with_memory(interpreter_setup).unwrap();
    let mut hash = result.stack().to_vec();
    hash.reverse();

    Ok(hash
        .iter()
        .map(|&x| x.low_u64())
        .collect::<Vec<_>>()
        .try_into()
        .unwrap())
}

// Test data from EIP-152.

fn test_blake2_f_eip(input: &str, output: &str) -> Result<()> {
    let (rounds, h, m, t_0, t_1, flag) = convert_input(input).unwrap();
    let result = run_blake2_f(rounds, h, m, t_0, t_1, flag).unwrap();
    assert_eq!(convert_output(result), output);
    Ok(())
}

#[test]
fn test_blake2_f_4() -> Result<()> {
    test_blake2_f_eip(
        "0000000048c9bdf267e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3af54fa5d182e6ad7f520e511f6c3e2b8c68059b6bbd41fbabd9831f79217e1319cde05b61626300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000001",
        "08c9bcf367e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3af54fa5d282e6ad7f520e511f6c3e2b8c68059b9442be0454267ce079217e1319cde05b",
    )
}

#[test]
fn test_blake2_f_5() -> Result<()> {
    test_blake2_f_eip(
        "0000000c48c9bdf267e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3af54fa5d182e6ad7f520e511f6c3e2b8c68059b6bbd41fbabd9831f79217e1319cde05b61626300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000001",
        "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d17d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923",
    )
}

#[test]
fn test_blake2_f_6() -> Result<()> {
    test_blake2_f_eip(
        "0000000c48c9bdf267e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3af54fa5d182e6ad7f520e511f6c3e2b8c68059b6bbd41fbabd9831f79217e1319cde05b61626300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000000",
        "75ab69d3190a562c51aef8d88f1c2775876944407270c42c9844252c26d2875298743e7f6d5ea2f2d3e8d226039cd31b4e426ac4f2d3d666a610c2116fde4735",
    )
}

#[test]
fn test_blake2_f_7() -> Result<()> {
    test_blake2_f_eip(
        "0000000148c9bdf267e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3af54fa5d182e6ad7f520e511f6c3e2b8c68059b6bbd41fbabd9831f79217e1319cde05b61626300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000001",
        "b63a380cb2897d521994a85234ee2c181b5f844d2c624c002677e9703449d2fba551b3a8333bcdf5f2f7e08993d53923de3d64fcc68c034e717b9293fed7a421",
    )
}

#[ignore]
#[test]
fn test_blake2_f_8() -> Result<()> {
    test_blake2_f_eip(
        "ffffffff48c9bdf267e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3af54fa5d182e6ad7f520e511f6c3e2b8c68059b6bbd41fbabd9831f79217e1319cde05b61626300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000001",
        "fc59093aafa9ab43daae0e914c57635c5402d8e3d2130eb9b3cc181de7f0ecf9b22bf99a7815ce16419e200e01846e6b5df8cc7703041bbceb571de6631d2615",
    )
}
