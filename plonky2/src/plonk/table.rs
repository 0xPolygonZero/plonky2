use plonky2_field::extension_field::Extendable;
use plonky2_field::field_types::Field;

use crate::hash::hash_types::RichField;
use crate::iop::target::Target;
use crate::iop::wire::Wire;
use crate::plonk::circuit_builder::CircuitBuilder;

pub enum Table<F: Field> {
    Inductive { initial_value: F, f: fn(F) -> F },
    VectorWithPadding { v: Vec<F>, padding_value: F },
    Vector(Vec<F>),
}

impl<F: Field> Table<F> {
    pub fn to_vec(&self, len: usize) -> Vec<F> {
        match self {
            Table::Inductive { initial_value, f } => {
                let mut cur = *initial_value;
                std::iter::repeat_with(|| {
                    let tmp = cur;
                    cur = f(cur);
                    tmp
                })
                .take(len)
                .collect()
            }
            Table::VectorWithPadding { v, padding_value } => {
                assert!(v.len() <= len);
                let mut ans = v.to_vec();
                ans.resize(len, *padding_value);
                ans
            }
            Table::Vector(v) => {
                assert_eq!(v.len(), len);
                v.to_vec()
            }
        }
    }
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn add_table(&mut self, table: Table<F>) -> usize {
        let table_index = self.tables.len();
        self.tables.push(table);
        table_index
    }

    pub fn lookup_in_table(&mut self, wire: Wire, table: usize) {
        self.lookups.push((wire.input, table));
    }
}

#[cfg(test)]
mod tests {
    use plonky2_field::goldilocks_field::GoldilocksField;

    use crate::field::field_types::Field;
    use crate::plonk::table::Table;

    #[test]
    fn test_yo() {
        type F = GoldilocksField;
        let t = Table::VectorWithPadding {
            v: vec![],
            padding_value: F::ZERO,
        };
        let tt = Table::Inductive {
            initial_value: F::ZERO,
            f: |x| x + F::ONE,
        };
        dbg!(t.to_vec(4));
        dbg!(tt.to_vec(4));
    }
}
