fn main() {
    for deg in (61..=64).rev() {
        for adic in (28..=32).rev() {
            for i in 1u128..100000 {
                if i.count_ones() == 1 && i != 1 {
                    continue;
                }
                let epsilon = i * (1u128 << adic) - 1;
                if epsilon > 1 << 32 {
                    break;
                }
                let n = ((1u128) << deg) - epsilon;
                let n = n as u64;
                let prime = is_prime(n);
                if prime {
                    println!("2^{} - ({} * 2**{} - 1) = {}", deg, i, adic, n);
                    let perm3 = (n - 1) % 3 != 0;
                    println!("  x^3 {}", perm3);
                    if perm3 {
                        let mut exp = n as u128;
                        while exp % 3 != 0 {
                            exp += (n - 1) as u128;
                        }
                        exp /= 3;
                        println!("  exp weight {}", exp.count_ones());
                    }
                    let perm5 = (n - 1) % 5 != 0;
                    println!("  x^5 {}", perm5);
                    if perm5 {
                        let mut exp = n as u128;
                        while exp % 5 != 0 {
                            exp += (n - 1) as u128;
                        }
                        exp /= 5;
                        println!("  exp weight {}", exp.count_ones());
                    }
                }
            }
        }
    }
}

fn is_prime(n: u64) -> bool {
    if (n & 1) == 0 {
        return false;
    }

    let mut d = 3;
    while d * d <= n {
        if n % d == 0 {
            return false;
        }
        d += 2;
    }

    true
}
