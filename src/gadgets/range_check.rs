use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::gates::base_sum::BaseSumGate;
use crate::generator::SimpleGenerator;
use crate::target::Target;
use crate::witness::PartialWitness;

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Checks that `x < 2^n_log` using a `BaseSumGate`.
    pub fn range_check(&mut self, x: Target, n_log: usize) {
        let gate = self.add_gate(BaseSumGate::<2>::new(n_log), vec![]);
        let sum = Target::wire(gate, BaseSumGate::<2>::WIRE_SUM);
        self.route(x, sum);
    }

    /// Returns `(a,b)` such that `x = a + 2^n_log * b` with `a < 2^n_log`.
    pub fn split_low_high(&mut self, x: Target, n_log: usize) -> (Target, Target) {
        let low_gate = self.add_gate(BaseSumGate::<2>::new(n_log), vec![]);
        let high_gate = self.add_gate(BaseSumGate::<2>::new(64 - n_log), vec![]);
        let low = Target::wire(low_gate, BaseSumGate::<2>::WIRE_SUM);
        let high = Target::wire(high_gate, BaseSumGate::<2>::WIRE_SUM);
        self.add_generator(LowHighGenerator {
            integer: x,
            n_log,
            low,
            high,
        });

        let pow2 = self.constant(F::from_canonical_u64(1 << n_log));
        let comp_x = self.mul_add(high, pow2, low);
        self.assert_equal(x, comp_x);

        (low, high)
    }
}

#[derive(Debug)]
struct LowHighGenerator {
    integer: Target,
    n_log: usize,
    low: Target,
    high: Target,
}

impl<F: Field> SimpleGenerator<F> for LowHighGenerator {
    fn dependencies(&self) -> Vec<Target> {
        vec![self.integer]
    }

    fn run_once(&self, witness: &PartialWitness<F>) -> PartialWitness<F> {
        let mut integer_value = witness.get_target(self.integer).to_canonical_u64();
        let low = integer_value & ((1 << self.n_log) - 1);
        let high = integer_value >> self.n_log;

        let mut result = PartialWitness::new();
        result.set_target(self.low, F::from_canonical_u64(low));
        result.set_target(self.high, F::from_canonical_u64(high));

        result
    }
}
