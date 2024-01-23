/// Shifts a byte to the right and returns both the shifted byte and the bits that carried.
pub fn shr_carry(input: u8, rotation: u8) -> (u8, u8) {
    let c_mod = rotation & 0x7;
    if c_mod != 0 {
        let res = input >> c_mod;
        let carry = (input << (8 - c_mod)) >> (8 - c_mod);
        (res, carry)
    } else {
        (input, 0u8)
    }
}

/// Computes a < b and returns the result as a u8.
pub fn lt(b: u8, c: u8) -> u8 {
    if b < c {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests the `shr_carry` function.
    #[test]
    fn test_shr_carry() {
        println!("{:?}", shr_carry(0, 2));
    }
}
