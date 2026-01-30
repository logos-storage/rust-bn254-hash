
#![allow(unused_imports)]

use ark_ff::prelude::{Zero};
use ark_bn254::Fr as F;

use crate::state::*;
use crate::{poseidon2, skyscraper};
use crate::griffin;

//------------------------------------------------------------------------------

#[derive(Debug, Copy, Clone)]
pub enum Hash {
  Poseidon2,
  Griffin,
  // Skyscraper
}

//------------------------------------------------------------------------------

pub fn permute(h: Hash, s: State) -> State {
  match h {
    Hash::Poseidon2 => poseidon2::permutation::permute(s),
    Hash::Griffin   =>   griffin::permutation::permute(s),
    // Hash::Skyscraper   =>   skyscraper::permutation::permute_state(s),
  }
}

pub fn permute_inplace(h: Hash, s: &mut State){
  match h {
    Hash::Poseidon2 => poseidon2::permutation::permute_inplace(s),
    Hash::Griffin   =>   griffin::permutation::permute_inplace(s),
    // Hash::Skyscraper   =>   skyscraper::permutation::permute_state_inplace(s),
  };
}

//------------------------------------------------------------------------------

pub fn compress(h: Hash, x: F, y: F) -> F {
  let mut u = State { x: x, y: y, z: F::zero() };
  permute_inplace(h, &mut u);
  u.x
}

pub fn keyed_compress(h: Hash, key: u64, x: F, y: F) -> F {
  let mut u = State { x: x, y: y, z: F::from(key) };
  permute_inplace(h, &mut u);
  u.x
}

//------------------------------------------------------------------------------


