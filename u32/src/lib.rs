#![allow(clippy::needless_range_loop)]

pub mod gadgets;
pub mod gates;
pub mod witness;

#[cfg(test)]
mod tests {
    // Set up Jemalloc for testing
    #[cfg(not(target_env = "msvc"))]
    use jemallocator::Jemalloc;

    #[cfg(not(target_env = "msvc"))]
    #[global_allocator]
    static GLOBAL: Jemalloc = Jemalloc;
}
