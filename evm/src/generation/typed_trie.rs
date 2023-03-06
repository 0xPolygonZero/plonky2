use std::fmt::Debug;
use std::sync::Arc;

use eth_trie_utils::partial_trie::{Nibbles, PartialTrie};
use ethereum_types::H256;
use rlp::Rlp;

pub(crate) type TypedWrappedNode<T> = Arc<Box<TypedPartialTrie<T>>>;

impl<T> From<TypedPartialTrie<T>> for TypedWrappedNode<T> {
    fn from(v: TypedPartialTrie<T>) -> Self {
        Arc::new(Box::new(v))
    }
}

/// A variant of `PartialTrie` where values are typed; intended for debugging.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) enum TypedPartialTrie<T> {
    Empty,
    Hash(H256),
    Branch {
        children: [TypedWrappedNode<T>; 16],
        value: Option<T>,
    },
    Extension {
        nibbles: Nibbles,
        child: TypedWrappedNode<T>,
    },
    Leaf {
        nibbles: Nibbles,
        value: T,
    },
}

impl<T: rlp::Decodable + Debug> From<&PartialTrie> for TypedPartialTrie<T> {
    fn from(value: &PartialTrie) -> Self {
        let convert_value = |value: &[u8]| {
            if value.is_empty() {
                None
            } else {
                Some(T::decode(&Rlp::new(value)).expect("Failed to decode"))
            }
        };
        match value {
            PartialTrie::Empty => TypedPartialTrie::Empty,
            PartialTrie::Hash(h) => TypedPartialTrie::Hash(*h),
            PartialTrie::Branch { children, value } => {
                let children = children
                    .clone()
                    .map(|c| Self::from(c.as_ref().as_ref()).into());
                let value = convert_value(value);
                TypedPartialTrie::Branch { children, value }
            }
            PartialTrie::Extension { nibbles, child } => TypedPartialTrie::Extension {
                nibbles: *nibbles,
                child: Self::from(child.as_ref().as_ref()).into(),
            },
            PartialTrie::Leaf { nibbles, value } => TypedPartialTrie::Leaf {
                nibbles: *nibbles,
                value: convert_value(value).expect("Leaf should have a value"),
            },
        }
    }
}
