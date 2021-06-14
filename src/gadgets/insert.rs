use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::target::Target;
use crate::util::log2_strict;

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Inserts a `Target` in a vector at a non-deterministic index. This is done by rotating to the
    /// left, inserting at 0 and then rotating to the right.
    /// Note: `index` is not range-checked.
    pub fn insert(
        &mut self,
        index: Target,
        element: ExtensionTarget<D>,
        mut v: Vec<ExtensionTarget<D>>,
    ) -> Vec<ExtensionTarget<D>> {
        let n = v.len();
        debug_assert!(n.is_power_of_two());
        let n_log = log2_strict(n);

        v.push(self.zero_extension());
        let mut v = self.rotate_left(index, &v, n_log);

        v.insert(0, element);
        v.pop().unwrap();

        self.rotate_right(index, &v, n_log)
    }
}
