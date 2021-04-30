use crate::field::field::Field;
use crate::merkle_tree::MerkleTree;
use crate::polynomial::polynomial::PolynomialValues;
use crate::util::transpose;

struct ListPolynomialCommitment<F: Field> {
    pub lde_values: Vec<Vec<F>>,
    pub rate_bits: usize,
    pub merkle_tree: MerkleTree<F>,
}

impl<F: Field> ListPolynomialCommitment<F> {
    pub fn new(values: Vec<PolynomialValues<F>>, rate_bits: usize) -> Self {
        let lde_values = values
            .into_iter()
            .map(|p| p.lde(rate_bits).values)
            .collect::<Vec<_>>();
        let merkle_tree = MerkleTree::new(transpose(&lde_values), false);

        Self {
            lde_values,
            rate_bits,
            merkle_tree,
        }
    }
}
