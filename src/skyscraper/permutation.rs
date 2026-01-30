use ark_bn254::{Fr as F};
use ark_ff::{BigInteger256, Field, One, PrimeField};
use core::str::FromStr;
use crate::skyscraper::constants::{FunctionBlock, RF1, RC, RC_RAW};
use crate::state::State;

pub fn bars_inplace_mont(x: &mut F) {
    // x → two 128‐bit chunks.
    let bi = x.0;
    let limbs = bi.0;
    let mut data = [0u128; 2];
    data[0] = (limbs[0] as u128) | ((limbs[1] as u128) << 64);
    data[1] = (limbs[2] as u128) | ((limbs[3] as u128) << 64);

    let t_function = |value: u128|  {
        let t1 = ((value & 0x80808080808080808080808080808080) >> 7) | ((value & 0x7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F) << 1); //circular left rot by 1
        let t2 = ((value & 0xC0C0C0C0C0C0C0C0C0C0C0C0C0C0C0C0) >> 6) | ((value & 0x3F3F3F3F3F3F3F3F3F3F3F3F3F3F3F3F) << 2); //circular left rot by 2
        let t3 = ((value & 0xE0E0E0E0E0E0E0E0E0E0E0E0E0E0E0E0) >> 5) | ((value & 0x1F1F1F1F1F1F1F1F1F1F1F1F1F1F1F1F) << 3); //circular left rot by 3
        let tmp = (!t1 & t2 & t3) ^ value;
        ((tmp & 0x80808080808080808080808080808080) >> 7) | ((tmp & 0x7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F) << 1) // Final left rot by 1
    };

    // T‐function to each chunk.
    let tmp_lo = t_function(data[1]);
    let tmp_hi = t_function(data[0]);
    data[0] = tmp_lo;
    data[1] = tmp_hi;

    // reduce
    reduce_small::<F>(&mut data);

    // put back into x
    let (lo, hi) = (data[0], data[1]);
    let mut out = BigInteger256::new([0, 0, 0, 0]);
    out.0[0] = (lo & 0xFFFFFFFFFFFFFFFF) as u64;
    out.0[1] = (lo >> 64) as u64;
    out.0[2] = (hi & 0xFFFFFFFFFFFFFFFF) as u64;
    out.0[3] = (hi >> 64) as u64;

    *x = F::new_unchecked(out)
}

// same reduction strategy as in the zkfriendlyhashzoo
fn reduce_small<F: PrimeField>(lhs: &mut [u128; 2]) {
    let p = F::characteristic(); // same as Modulus
    let pa = p.as_ref();
    // prime in two 128‐bit limbs
    let mut prime = [0u128; 2];
    prime[0] = (pa[0] as u128) | ((pa[1] as u128) << 64);
    prime[1] = (pa[2] as u128) | ((pa[3] as u128) << 64);

    loop {
        for idx in (0..2).rev() {
            if lhs[idx] < prime[idx] {
                return;
            }
            if lhs[idx] > prime[idx] {
                sub_full(lhs, &prime);
                break;
            }
            if idx == 0 && lhs[idx] == prime[idx] {
                lhs[0] = 0;
                lhs[1] = 0;
                return;
            }
        }
    }
}

pub fn sub_full(lhs: &mut [u128], rhs: &[u128]) {
    let mut overflow: bool;
    let mut overflow_part: u128;
    (lhs[0], overflow) = lhs[0].overflowing_sub(rhs[0]);
    overflow_part = if overflow {1} else {0};

    for index in 1..rhs.len(){
        (lhs[index], overflow) = lhs[index].overflowing_sub(overflow_part);
        overflow_part = if overflow {1} else {0};
        (lhs[index], overflow) = lhs[index].overflowing_sub(rhs[index]);
        incr(&mut overflow_part, overflow);
    }
}
#[inline(always)]
pub fn incr(left: &mut u128, right: bool){
    if right {
        *left += 1;
    }
}


fn square_inplace(x: &mut F) {
    *x *= *x;
}

pub fn permute(input: [F; 2]) -> [F; 2] {
    let mut current_state = input;
    let mut left  = current_state[0];
    let mut right = current_state[1];

    for (i, &fun) in RF1.iter().enumerate() {
        // s‐box on `left`
        match fun {
            FunctionBlock::Square => square_inplace(&mut left),
            FunctionBlock::Bar => bars_inplace_mont(&mut left),
        }

        if i > 0 && i < (RF1.len() - 1) {
            right += RC_RAW[i - 1];
        }

        // combine
        left += right;

        // the feistel rotation
        right = current_state[0];
        current_state[0] = left;
    }
    current_state[1] = right;
    current_state
}

/// WARNING: this ignores the z element of the state
/// TODO: extension field
pub(crate) fn permute_state_inplace(u: &mut State) {
    let ns = permute([u.x,u.y]);
    u.x = ns[0];
    u.y = ns[1];
}

/// WARNING: this ignores the z element of the state
/// TODO: extension field
pub(crate) fn permute_state(mut u: State) -> State{
    permute_state_inplace(&mut u);
    u
}

pub fn compress(x: F, y: F) -> F {
    let p_out = permute([x, y]);
    let out = x + p_out[0];
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permutation() {
        let init = [F::from(1234u64), F::from(5678u64)];
        let init_mont = [F::new_unchecked(init[0].into_bigint()), F::new_unchecked(init[1].into_bigint())];
        let out = permute(init_mont);
        println!("Permutation on (1234,5678) => ({},{})", out[0].0, out[1].0);
        let expected = [
            BigInteger256::from_str("10398388528337208913702213361515546865573572771332206462397283188708690721181").unwrap(),
            BigInteger256::from_str("21827939006013637437091304277086947125806852726479468249428384625000968262245").unwrap()
        ];

        assert_eq!(out[0].0, expected[0]);
        assert_eq!(out[1].0, expected[1]);
    }
}