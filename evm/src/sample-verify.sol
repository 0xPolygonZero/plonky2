pragma solidity >=0.7.0 <0.9.0;

type H256 is bytes32;

contract Plonky2Verification {

    struct TrieRoots {
        H256 stateRoot;
        H256 transactionsRoot;
        H256 receiptsRoot;
    }

    function serializeTrieRoots(trieRoots: trieRoots) -> bytes memory {
        abi.encodePacked(trieRoots.stateRoot, trieRoots.transactionsRoot, trieRoots.receiptsRoot);
    }

    struct BlockMetadata {
        address blockBeneficiary;
        uint256 blockTimestamp;
        uint256 blockNumber;
        uint256 blockDifficulty;
        H256 blockRandom;
        uint256 blockGaslimit;
        uint256 blockChainId;
        uint256 blockBaseFee;
        uint256 blockGasUsed;
        uint256[8] blockBloom;
    }

    function serializeBlockMetadata(blockMetadata: BlockMetadata) -> bytes memory {
        abi.encodePacked(
            blockMetadata.blockBeneficiary,
            blockMetadata.blockTimestamp,
            blockMetadata.blockNumber,
            blockMetadata.blockDifficulty,
            blockMetadata.blockRandom,
            blockMetadata.blockGaslimit,
            blockMetadata.blockChainId,
            blockMetadata.blockBaseFee,
            blockMetadata.blockGasUsed,
            blockMetadata.blockBloom,
        );
    }

    struct BlockHashes {
        H256[] prevHashes;
        H256 curHash;
    }

    function serializeBlockHashes(blockHashes: BlockHashes) -> bytes memory {
        abi.encodePacked(blockHashes.prevHashes, blockHashes.curHash);
    }

    struct ExtraBlockData {
        H256 genesisStateTrieRoot;
        uint256 txnNumberBefore;
        uint256 txnNumber_After;
        uint256 gasUsedBefore;
        uint256 gasUsed_After;
        uint256[8] blockBloomBefore;
        uint256[8] blockBloomAfter;
    }

    function serializeExtraBlockData(extraBlockData: ExtraBlockData) -> bytes memory {
        abi.encodePacked(
            extraBlockData.genesisStateTrieRoot,
            extraBlockData.txnNumberBefore,
            extraBlockData.txnNumber_After,
            extraBlockData.gasUsedBefore,
            extraBlockData.gasUsed_After,
            extraBlockData.blockBloomBefore,
            extraBlockData.blockBloomAfter,
        );
    }

    struct PublicValues {
        TrieRoots trieRootsBefore;
        TrieRoots trieRootsAfter;
        BlockMetadata blockMetadata;
        BlockHashes blockHashes;
        ExtraBlockData extraBlockData;
    }

    function serializePublicValues(publicValues: PublicValues) -> bytes memory {
        serializeTrieRoots(publicValues.trieRootsBefore);
        serializeTrieRoots(publicValues.trieRootsAfter);
        serializeBlockMetadata(publicValues.blockMetadata);
        serializeBlockHashes(publicValues.blockHashes);
        serializeExtraBlockData(publicValues.extraBlockData);
    }

    constructor() {
        TrieRoots dummyTrieRootsBefore = TrieRoots(
            abi.encodePacked(0x92648889955b1d41b36ea681a16ef94852e34e6011d029f278439adb4e9e30b4),
            abi.encodePacked(0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421),
            abi.encodePacked(0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421),
        );
        TrieRoots dummyTrieRootsAfter = TrieRoots(
            abi.encodePacked(0x049e45aef8dac161e0cec0edacd8af5b3399700affad6ede63b33c5d0ec796f5),
            abi.encodePacked(0xc523d7b87c0e49a24dae53b3e3be716e5a6808c1e05216497655c0ad84b12236),
            abi.encodePacked(0xfc047c9c96ea3d317bf5b0896e85c242ecc625efd3f7da721c439aff8331b2ab),
        );
        BlockMetadata dummyBlockMetadata = BlockMetadata(
            0x2adc25665018aa1fe0e6bc666dac8fc2697ff9ba,
            1000,
            0,
            131072,
            0x0000000000000000000000000000000000000000000000000000000000000000,
            4478310,
            1,
            10,
            43570,
            [
                0,
                0,
                55213970774324510299479508399853534522527075462195808724319849722937344,
                1361129467683753853853498429727072845824,
                33554432,
                9223372036854775808,
                3618502788666131106986593281521497120414687020801267626233049500247285563392,
                2722259584404615024560450425766186844160,
            ],
        );
        BlockHashes dummyBlockHashes = BlockHashes(
            [
                abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0), abi.encodePacked(0x0),
            ],
            abi.encodePacked(0x0),
        );
        ExtraBlockData dummyExtraBlockData = ExtraBlockData(
            abi.encodePacked(0x92648889955b1d41b36ea681a16ef94852e34e6011d029f278439adb4e9e30b4),
            0,
            2,
            0,
            43570,
            [ 0, 0, 0, 0, 0, 0, 0, 0 ],
            [
                0,
                0,
                55213970774324510299479508399853534522527075462195808724319849722937344,
                1361129467683753853853498429727072845824,
                33554432,
                9223372036854775808,
                3618502788666131106986593281521497120414687020801267626233049500247285563392,
                2722259584404615024560450425766186844160,
            ],
        );
        
        PublicValues dummyPublicValues = PublicValues(
            dummyTrieRoots,
            dummyTrieRoots,
            dummyBlockMetadata,
            dummyBlockHashes,
            dummyExtraBlockData,
        );
    }
}