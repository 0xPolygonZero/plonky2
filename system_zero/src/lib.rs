mod alu;
mod core_registers;
pub mod lookup;
mod memory;
mod permutation_unit;
mod public_input_layout;
mod registers;
pub mod system_zero;

#[cfg(test)]
mod tests {
    // Set up Jemalloc for testing
    #[cfg(not(target_env = "msvc"))]
    use jemallocator::Jemalloc;

    #[cfg(not(target_env = "msvc"))]
    #[global_allocator]
    static GLOBAL: Jemalloc = Jemalloc;
}
