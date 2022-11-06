use plonky2_field::extension::Extendable;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::target::{BoolTarget};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use std::time::{Instant, Duration};
use crate::helper::{uint32_to_bits, _right_rotate, _shr};
use crate::bit_operations::{not_arr, and_arr, xor2_arr, xor3_arr, add_arr, zip_add};

pub struct Sha256Target {
    pub message: Vec<BoolTarget>,
    pub digest: Vec<BoolTarget>,
}

fn get_initial_hash<F:RichField + Extendable<D>, const D:usize>(builder: &mut CircuitBuilder<F, D>) -> [[BoolTarget; 32]; 8] {
    let initial_hash = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19
    ];
    let mut res = [None; 8];
    for i in 0..8 {
        res[i] = Some(uint32_to_bits(initial_hash[i], builder));
    }
    res.map(|x| x.unwrap())
}

fn get_round_constants<F:RichField + Extendable<D>, const D:usize>(builder: &mut CircuitBuilder<F, D>) -> [[BoolTarget; 32]; 64] {
    let round_constants: [u32;64] = [
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
            0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
            0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
            0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
            0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
            0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
            0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
            0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
            0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
            0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
            0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
            0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
            0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
            0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
            0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
            0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2];
    let mut res = [None; 64];
    for i in 0..64 {
        res[i] = Some(uint32_to_bits(round_constants[i], builder));
    }
    res.map(|x| x.unwrap())
}

fn reshape(u: Vec<BoolTarget>) -> Vec<[BoolTarget; 32]>{
    let l = u.len()  / 32;
    let mut res = Vec::new();
    for i in 0..l {
        let mut arr = [None; 32];
        for j in 0..32 {
            arr[j] = Some(u[i*32 + j]);
        }
        res.push(arr.map(|x| x.unwrap()));
    }
    res
}

// reference: https://github.com/thomdixon/pysha2/blob/master/sha2/sha256.py
pub fn make_sha256_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    msg_bit_len: u128,
) -> Sha256Target 
{
    let mut msg_input = Vec::new();

    // Add signals for the input msg
    for _ in 0..msg_bit_len {
        msg_input.push(builder.add_virtual_bool_target_safe()); // Will verify that input is 0 or 1
    }

    let mdi = (msg_bit_len / 8) % 64;
    let length = ((msg_bit_len / 8) << 3); // length in bytes
    let padlen = if mdi < 56 { 55 - mdi } else { 119 - mdi };
    
    msg_input.push(builder.constant_bool(true));
    for _ in 0..7 {
        msg_input.push(builder.constant_bool(false));
    }

    for _ in 0..padlen*8 {
        msg_input.push(builder.constant_bool(false));
    }

    for i in (0..64).rev() {
        // big endian binary representation of length
        msg_input.push(builder.constant_bool((length >> i) & 1 == 1));
    }

    let mut sha256_hash = get_initial_hash(builder);
    let round_constants = get_round_constants(builder);

    // Process the input with 512 bit chunks aka 64 byte chunks
    for chunk_start in (0..msg_input.len()).step_by(512) {
        let chunk = msg_input[chunk_start..chunk_start+512].to_vec();
        let mut u = Vec::new(); 

        for i in 0..512 { // 0 .. 16 chunk size * 32 bits7
            u.push(chunk[i]);
        }
        for _ in 512..64*32 { // 16 * 8 ... 64 * 8 because of L
            u.push(builder.constant_bool(false));
        }

        let mut w = reshape(u);
        for i in 16..64 {
            let s0 = xor3_arr(
                _right_rotate(w[i-15], 7), 
                _right_rotate(w[i-15], 18), 
                _shr(w[i-15], 3, builder),
                builder,
            );
            let s1 = xor3_arr(
                _right_rotate(w[i-2], 17),
                _right_rotate(w[i-2], 19), 
                _shr(w[i-2], 10, builder),
                builder, 
            );
            let inter1 = add_arr(w[i-16], s0, builder);
            let inter2 = add_arr(inter1, w[i-7], builder);
            w[i] = add_arr(inter2, s1, builder);

        }
        let mut a = sha256_hash[0];
        let mut b = sha256_hash[1];
        let mut c = sha256_hash[2];
        let mut d = sha256_hash[3];
        let mut e = sha256_hash[4];
        let mut f = sha256_hash[5];
        let mut g = sha256_hash[6];
        let mut h = sha256_hash[7];

        for i in 0..64 {
            let sum1 = xor3_arr(
                _right_rotate(e, 6),
                _right_rotate(e, 11),
                _right_rotate(e, 25),
                builder, 
            );
            let ch = xor2_arr(
                and_arr(e, f, builder),
                and_arr(not_arr(e, builder), g, builder),
                builder,
            );
            let temp1 = add_arr(h, sum1, builder);
            let temp2 = add_arr(temp1, ch, builder);
            let temp3 = add_arr(temp2, round_constants[i], builder);
            let temp4 = add_arr(temp3, w[i], builder);
            let final_temp1 = temp4;

            let sum0 = xor3_arr(
                _right_rotate(a, 2),
                _right_rotate(a, 13),
                _right_rotate(a, 22),
                builder, 
            );

            let maj = xor3_arr(
                and_arr(a, b, builder),
                and_arr(a, c, builder),
                and_arr(b, c, builder),
                builder,
            );
            let final_temp2  = add_arr(sum0, maj, builder);
			
            h = g;
            g = f;
            f = e;
            e = add_arr(d, final_temp1, builder);
            d = c;
            c = b;
            b = a;
            a = add_arr(final_temp1, final_temp2, builder);

        }

        sha256_hash = zip_add(sha256_hash, [a, b, c, d, e, f, g, h], builder);
    }

    let mut digest = Vec::new();
    for word in sha256_hash.iter() {
        for i in 0..word.len() {
            digest.push(word[i]);
        }
    }

    return Sha256Target { message: msg_input, digest: digest}
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use hex::decode;
    use plonky2::iop::witness::{PartialWitness, Witness};
    use plonky2::plonk::circuit_builder::CircuitBuilder;
    use plonky2::plonk::circuit_data::CircuitConfig;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    use crate::sha256::make_sha256_circuit;

    fn to_bits(msg: Vec<u8>) -> Vec<bool> {
        let mut res = Vec::new();
        for i in 0..msg.len() {
            let char = msg[i];
            for j in 0..8 {
                if (char & (1 << 7 - j)) != 0 {
                    res.push(true);
                } else {
                    res.push(false);
                }
            }
        }
        res
    }

    #[test]
    fn test_sha256_bench() -> Result<()> {
        let mut msg = String::new();
        for _ in 0..8 {
            msg.push_str("abcdefghij");
        }
        let msg_bits = to_bits(msg.as_bytes().to_vec());
        let expected_digest = "d68d62c262c2ec08961c1104188cde86f51695878759666ad61490c8ec66745c";
        let digest_bits = to_bits(decode(expected_digest).unwrap());

        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let mut builder = CircuitBuilder::<F, D>::new(CircuitConfig::standard_recursion_config());
        let targets = make_sha256_circuit(&mut builder, msg_bits.len().try_into().unwrap());
       

        for i in 0..digest_bits.len() {
            if digest_bits[i] {
                builder.assert_one(targets.digest[i].target);
            } else {
                builder.assert_zero(targets.digest[i].target);
            }
        }


        let data = builder.build::<C>();

        
        for i in 0..10 {
            let mut pw = PartialWitness::new();

            for i in 0..msg_bits.len() {
                pw.set_bool_target(targets.message[i], msg_bits[i]);
            }
            let now = std::time::Instant::now();
            let proof = data.prove(pw).unwrap();
            println!("{} step, time elapsed {}", i, now.elapsed().as_millis());
        }

        Ok(())
    }

    #[test]
    fn test_sha256_empty() -> Result<()> {
        let msg = b"";
        let msg_bits = to_bits(msg.to_vec());
        let expected_digest = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let digest_bits = to_bits(decode(expected_digest).unwrap());

        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let mut builder = CircuitBuilder::<F, D>::new(CircuitConfig::standard_recursion_config());
        let targets = make_sha256_circuit(&mut builder, msg_bits.len().try_into().unwrap());
        let mut pw = PartialWitness::new();

        for i in 0..msg_bits.len() {
            pw.set_bool_target(targets.message[i], msg_bits[i]);
        }

        for i in 0..digest_bits.len() {
            if digest_bits[i] {
                builder.assert_one(targets.digest[i].target);
            } else {
                builder.assert_zero(targets.digest[i].target);
            }
        }

        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();

        data.verify(proof)
    }

    #[test]
    fn test_sha256_small_msg() -> Result<()> {
        let msg = b"plonky2";
        let msg_bits = to_bits(msg.to_vec());
        let expected_digest = "8943a85083f16e93dc92d6af455841daacdae5081aa3125b614a626df15461eb";
        let digest_bits = to_bits(decode(expected_digest).unwrap());

        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let mut builder = CircuitBuilder::<F, D>::new(CircuitConfig::standard_recursion_config());
        let targets = make_sha256_circuit(&mut builder, msg_bits.len().try_into().unwrap());
        let mut pw = PartialWitness::new();

        for i in 0..msg_bits.len() {
            pw.set_bool_target(targets.message[i], msg_bits[i]);
        }

        for i in 0..digest_bits.len() {
            if digest_bits[i] {
                builder.assert_one(targets.digest[i].target);
            } else {
                builder.assert_zero(targets.digest[i].target);
            }
        }

        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();

        data.verify(proof)
    }

    #[test]
    fn test_sha256_large_msg() -> Result<()> {
        let msg = decode("35c323757c20640a294345c89c0bfcebe3d554fdb0c7b7a0bdb72222c531b1ecf7ec1c43f4de9d49556de87b86b26a98942cb078486fdb44de38b80864c3973153756363696e6374204c616273").unwrap();
        let msg_bits = to_bits(msg.to_vec());
        let expected_digest = "8fcee6fbeadc123c38d5a97dbe58f8257b4906820d627425af668b94b795e74e";
        let digest_bits = to_bits(decode(expected_digest).unwrap());

        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let mut builder = CircuitBuilder::<F, D>::new(CircuitConfig::standard_ecc_config());
        let targets = make_sha256_circuit(&mut builder, msg_bits.len().try_into().unwrap());
        let mut pw = PartialWitness::new();

        for i in 0..msg_bits.len() {
            pw.set_bool_target(targets.message[i], msg_bits[i]);
        }

        for i in 0..digest_bits.len() {
            if digest_bits[i] {
                builder.assert_one(targets.digest[i].target);
            } else {
                builder.assert_zero(targets.digest[i].target);
            }
        }

        dbg!(builder.num_gates());
        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();

        data.verify(proof)
    }

    #[test]
    #[should_panic]
    fn test_sha512_failure() {
        let msg = decode("35c323757c20640a294345c89c0bfcebe3d554fdb0c7b7a0bdb72222c531b1ecf7ec1c43f4de9d49556de87b86b26a98942cb078486fdb44de38b80864c3973153756363696e6374204c616273").unwrap();
        let msg_bits = to_bits(msg.to_vec());
        let expected_digest = "9fcee6fbeadc123c38d5a97dbe58f8257b4906820d627425af668b94b795e74e";
        let digest_bits = to_bits(decode(expected_digest).unwrap());

        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let mut builder = CircuitBuilder::<F, D>::new(CircuitConfig::standard_recursion_config());
        let targets = make_sha256_circuit(&mut builder, msg_bits.len().try_into().unwrap());
        let mut pw = PartialWitness::new();

        for i in 0..msg_bits.len() {
            pw.set_bool_target(targets.message[i], msg_bits[i]);
        }

        for i in 0..digest_bits.len() {
            if digest_bits[i] {
                builder.assert_one(targets.digest[i].target);
            } else {
                builder.assert_zero(targets.digest[i].target);
            }
        }

        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();

        data.verify(proof).expect("sha256 error");
    }    

}