use anyhow::{anyhow, Result};
use eth_trie_utils::partial_trie::PartialTrie;
use ethereum_types::{BigEndianHash, H256, U256};

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::tests::mpt::{extension_to_leaf, test_account_1_rlp, test_account_2_rlp};
use crate::generation::mpt::{
    all_mpt_prover_inputs_reversed, state_smt_prover_inputs, state_smt_prover_inputs_reversed,
};
use crate::generation::TrieInputs;
use crate::Node;

// TODO: Test with short leaf. Might need to be a storage trie.

#[test]
fn smt_hash() -> Result<()> {
    // let state_smt = [
    //     "1",
    //     "3",
    //     "5",
    //     "0",
    //     "0",
    //     "1",
    //     "8",
    //     "16",
    //     "2",
    //     "57896044618658097711785492504343953926634992332820282019728792003956564819968",
    //     "15450786757123453439",
    //     "22643746980206239462929324928381526295855492643195463681584304106277748792610",
    //     "14",
    //     "82556071676249696196436722777043875263842381384143312748198043956457558240436",
    //     "0",
    //     "0",
    //     "2",
    //     "86844066927987146567678238756515930889952488499230423029593188005934847229952",
    //     "18161530422407358598",
    //     "76787957748290171668506414736037771547459283808788837043402835113099504989954",
    //     "22",
    //     "25198543450711007348123120128462564350095946700804425833122107489109791993828",
    //     "0",
    //     "0",
    // ]
    //     .iter()
    //     .map(|s| U256::from_dec_str(s).unwrap())
    //     .collect::<Vec<_>>();
    let state_smt = [
        "1",
        "3",
        "5",
        "0",
        "0",
        "1",
        "8",
        "81",
        "2",
        "57896044618658097711785492504343953926634992332820282019728792003956564819968",
        "6031852226157986198",
        "106161476088327142152199525654738271207912606538307655305305561965385251525584",
        "14",
        "79589599609422202436001745250656009311442985310891377317806242244372303146666",
        "1",
        "17",
        "49",
        "1",
        "20",
        "40",
        "1",
        "23",
        "26",
        "2",
        "8784066359843413222562719269502695216087082165461564857235599220692660059844",
        "14752201973882333226639855884093784080982181310578764857468159466151449243914",
        "1",
        "29",
        "31",
        "0",
        "0",
        "1",
        "34",
        "37",
        "2",
        "24956434737841505466404572165879230536358542714850060774280931995222676533028",
        "59896359980073666099593271162724524764233434171370608879788611049218369111515",
        "2",
        "27719226894305605598473046489666257609449029984637312601517554290291466335686",
        "16869961137380263667352284318039769904445139113829715669809703805324543413369",
        "1",
        "43",
        "46",
        "2",
        "34052007344979184294078920184956633671523553208897636379459478394767689315487",
        "48008647881420978713727636017449648954135925776050877303721380603954871777967",
        "2",
        "55493451065639205757466023745582263442615924987104390196717319702780500944361",
        "5597729208789506764647082947121649859476739507495050676206557497671131062289",
        "1",
        "52",
        "55",
        "2",
        "66714677558511460537626853239145316712809451733595601385220831653375532362120",
        "9393151685607880348775233015220358802770107175423213973437757920226939747886",
        "1",
        "58",
        "60",
        "0",
        "0",
        "1",
        "63",
        "72",
        "1",
        "66",
        "69",
        "2",
        "102676986019655677204024860288340192492961819369873672973778807047932606810272",
        "102607813865825797704071723659960646490051538205496884298918291340161403783624",
        "2",
        "105094293107613626606593407422658452641533592534324505955140305499956483760903",
        "62631812672161490176166064891586689930224950592637005369182832494869704692882",
        "1",
        "75",
        "78",
        "2",
        "110299999246524980979366736984934966675231066659966403159447545667257652979326",
        "73017408750839319813787883821892533908118248145059451621276757802353952888004",
        "2",
        "115358829827687622803956708279650814786266413031368646344198365671737415070087",
        "97872099957547359766929945867439505023831330961013678329945402122122052792048",
        "2",
        "86844066927987146567678238756515930889952488499230423029593188005934847229952",
        "17096971921000269521",
        "50054366972152758308100666172971774406949987337129256794756384161311994025447",
        "87",
        "55698387663419864470477787514445259853752887187180632782871674337362998551070",
        "1",
        "90",
        "121",
        "1",
        "93",
        "112",
        "1",
        "96",
        "110",
        "1",
        "99",
        "108",
        "1",
        "102",
        "105",
        "2",
        "1950847846470958671630278293558950702649677513000820177044273585221609118023",
        "54716902097863563858107969371976748283977555728357787048692322938233046065712",
        "2",
        "6172861765036294763966320073211214142507888523759956991776311847564483346648",
        "99032953083805273537895066109044626224369189434867741118987275629998335262481",
        "0",
        "0",
        "0",
        "0",
        "1",
        "115",
        "118",
        "2",
        "42361344357375562236305638901138095615193772581011446168092182265823776349808",
        "101566274663492624164813331377068382078169496787858591098055658525273490323643",
        "2",
        "46225445316097543235261612057518541181062166367238759705737605219634148583184",
        "11872067949477936576329310863867085426400668933777286622081219542576440017295",
        "1",
        "124",
        "159",
        "1",
        "127",
        "130",
        "2",
        "64336303410135220081121645137228808559097362806134097769882626771265621713459",
        "30445486738471861740960559751148011315615700133731063886276513082922629536890",
        "1",
        "133",
        "135",
        "0",
        "0",
        "1",
        "138",
        "140",
        "0",
        "0",
        "1",
        "143",
        "145",
        "0",
        "0",
        "1",
        "148",
        "157",
        "1",
        "151",
        "154",
        "2",
        "85381807930110493970396584845957439840957574378718377576974857846776587809421",
        "113125231910544763593457054604509014080658982002978905999384704688400566533441",
        "2",
        "85696828948375489328108108027068727412181703874419587196833914182188505374057",
        "71672949300524048365867221682169754596500916413927290731148611803542354881357",
        "0",
        "0",
        "1",
        "162",
        "165",
        "2",
        "95930572437351211930941108557392494617500413264995237669951542888727555180891",
        "33681404674969435699120330564034304173147208509956361926799536440183490803746",
        "1",
        "168",
        "171",
        "2",
        "104649864616011403976906111861203040285501519155296767114852256086487142889125",
        "55892064691547849471512132971087565293912387083067587235046496857669284420787",
        "2",
        "114780416924077579789210823977441004364709218580313423551902619577719965514301",
        "112942588471396163239544937881392699678368752867042283258697992254345474009510",
    ]
    .iter()
    .map(|s| U256::from_dec_str(s).unwrap())
    .collect::<Vec<_>>();

    test_state_smt(state_smt)
}

fn test_state_smt(state_smt: Vec<U256>) -> Result<()> {
    let trie_inputs = TrieInputs {
        state_trie: state_smt,
        transactions_trie: Default::default(),
        receipts_trie: Default::default(),
        storage_tries: vec![],
    };
    let load_all_mpts = KERNEL.global_labels["load_all_mpts"];
    let smt_hash_state = KERNEL.global_labels["smt_hash_state"];

    let initial_stack = vec![0xDEADBEEFu32.into()];
    let mut interpreter = Interpreter::new_with_kernel(load_all_mpts, initial_stack);
    interpreter.generation_state.mpt_prover_inputs =
        all_mpt_prover_inputs_reversed(&trie_inputs).map_err(|_| anyhow!("Invalid MPT data"))?;
    interpreter.generation_state.state_smt_prover_inputs =
        state_smt_prover_inputs_reversed(&trie_inputs);
    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![]);

    // Now, execute mpt_hash_state_trie.
    interpreter.generation_state.registers.program_counter = smt_hash_state;
    interpreter.push(0xDEADBEEFu32.into());
    interpreter.run()?;

    assert_eq!(
        interpreter.stack().len(),
        1,
        "Expected 1 item on stack, found {:?}",
        interpreter.stack()
    );
    let hash = H256::from_uint(&interpreter.stack()[0]);
    dbg!(hash);
    // let expected_state_trie_hash = trie_inputs.state_trie.hash();
    // assert_eq!(hash, expected_state_trie_hash);

    Ok(())
}

// #[test]
// fn smt_hash_empty() -> Result<()> {
//     let trie_inputs = TrieInputs {
//         state_trie: Default::default(),
//         transactions_trie: Default::default(),
//         receipts_trie: Default::default(),
//         storage_tries: vec![],
//     };
//
//     test_state_trie(trie_inputs)
// }
//
// #[test]
// fn mpt_hash_empty_branch() -> Result<()> {
//     let children = core::array::from_fn(|_| Node::Empty.into());
//     let state_trie = Node::Branch {
//         children,
//         value: vec![],
//     }
//     .into();
//     let trie_inputs = TrieInputs {
//         state_trie,
//         transactions_trie: Default::default(),
//         receipts_trie: Default::default(),
//         storage_tries: vec![],
//     };
//     test_state_trie(trie_inputs)
// }
//
// #[test]
// fn mpt_hash_hash() -> Result<()> {
//     let hash = H256::random();
//     let trie_inputs = TrieInputs {
//         state_trie: Node::Hash(hash).into(),
//         transactions_trie: Default::default(),
//         receipts_trie: Default::default(),
//         storage_tries: vec![],
//     };
//
//     test_state_trie(trie_inputs)
// }
//
// #[test]
// fn mpt_hash_leaf() -> Result<()> {
//     let state_trie = Node::Leaf {
//         nibbles: 0xABC_u64.into(),
//         value: test_account_1_rlp(),
//     }
//     .into();
//     let trie_inputs = TrieInputs {
//         state_trie,
//         transactions_trie: Default::default(),
//         receipts_trie: Default::default(),
//         storage_tries: vec![],
//     };
//     test_state_trie(trie_inputs)
// }
//
// #[test]
// fn mpt_hash_extension_to_leaf() -> Result<()> {
//     let state_trie = extension_to_leaf(test_account_1_rlp());
//     let trie_inputs = TrieInputs {
//         state_trie,
//         transactions_trie: Default::default(),
//         receipts_trie: Default::default(),
//         storage_tries: vec![],
//     };
//     test_state_trie(trie_inputs)
// }
//
// #[test]
// fn mpt_hash_branch_to_leaf() -> Result<()> {
//     let leaf = Node::Leaf {
//         nibbles: 0xABC_u64.into(),
//         value: test_account_2_rlp(),
//     }
//     .into();
//
//     let mut children = core::array::from_fn(|_| Node::Empty.into());
//     children[3] = leaf;
//     let state_trie = Node::Branch {
//         children,
//         value: vec![],
//     }
//     .into();
//
//     let trie_inputs = TrieInputs {
//         state_trie,
//         transactions_trie: Default::default(),
//         receipts_trie: Default::default(),
//         storage_tries: vec![],
//     };
//
//     test_state_trie(trie_inputs)
// }
//
// fn test_state_trie(trie_inputs: TrieInputs) -> Result<()> {
//     let load_all_mpts = KERNEL.global_labels["load_all_mpts"];
//     let mpt_hash_state_trie = KERNEL.global_labels["mpt_hash_state_trie"];
//
//     let initial_stack = vec![0xDEADBEEFu32.into()];
//     let mut interpreter = Interpreter::new_with_kernel(load_all_mpts, initial_stack);
//     interpreter.generation_state.mpt_prover_inputs =
//         all_mpt_prover_inputs_reversed(&trie_inputs).map_err(|_| anyhow!("Invalid MPT data"))?;
//     interpreter.run()?;
//     assert_eq!(interpreter.stack(), vec![]);
//
//     // Now, execute mpt_hash_state_trie.
//     interpreter.generation_state.registers.program_counter = mpt_hash_state_trie;
//     interpreter.push(0xDEADBEEFu32.into());
//     interpreter.run()?;
//
//     assert_eq!(
//         interpreter.stack().len(),
//         1,
//         "Expected 1 item on stack, found {:?}",
//         interpreter.stack()
//     );
//     let hash = H256::from_uint(&interpreter.stack()[0]);
//     let expected_state_trie_hash = trie_inputs.state_trie.hash();
//     assert_eq!(hash, expected_state_trie_hash);
//
//     Ok(())
// }
