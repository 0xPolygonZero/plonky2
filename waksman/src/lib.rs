#![allow(clippy::new_without_default)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::len_without_is_empty)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::return_self_not_must_use)]

pub mod bimap;
pub mod gates;
pub mod permutation;
pub mod sorting;

#[cfg(test)]
mod tests {
    // Set up Jemalloc for testing
    #[cfg(not(target_env = "msvc"))]
    use jemallocator::Jemalloc;

    #[cfg(not(target_env = "msvc"))]
    #[global_allocator]
    static GLOBAL: Jemalloc = Jemalloc;
}
