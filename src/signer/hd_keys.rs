extern crate curv;

use core::slice::SlicePattern;

use curv::arithmetic::traits::Converter;

use curv::{
    BigInt,

    arithmetic::{BasicOps, One}
};
use hmac::{Hmac, Mac, NewMac};
use sha2::Sha512;
use crate::signer::secp256k1def::{FE, GE};
use zeroize::Zeroize;

pub fn get_hd_key(y_sum: &GE, path_vector: Vec<BigInt>) -> (GE, FE) {
    // generate a random but shared chain code, this will do
    let chain_code = GE::generator().as_point();
    //    println!("chain code {:?}", chain_code);
    // derive a new pubkey and LR sequence, y_sum becomes a new child pub key
    let (y_sum_child, f_l_new, _cc_new) = hd_key(
        path_vector,
        &y_sum,
        &BigInt::from_bytes(&chain_code.to_bytes(true).as_slice()),
    );
    let y_sum = y_sum_child.clone();
    //    println!("New public key: {:?}", &y_sum);
    //    println!("Public key X: {:?}", &y_sum.x_coord());
    //    println!("Public key Y: {:?}", &y_sum.y_coord());
    (y_sum, f_l_new)
}

pub fn hd_key(
    mut location_in_hir: Vec<BigInt>,
    pubkey: &GE,
    chain_code_bi: &BigInt,
) -> (GE, FE, GE) {
    let mask = BigInt::from(2).pow(256) - BigInt::one();
    // let public_key = self.public.q.clone();

    // calc first element:
    let first = location_in_hir.remove(0);
    let pub_key_bi = &BigInt::from_bytes(&pubkey.to_bytes(true).as_slice());
    let f = create_hmac(&chain_code_bi, &[&pub_key_bi, &first]);
    let f_l = &f >> 256;
    let f_r = &f & &mask;
    let f_l_fe: FE = FE::from(&f_l);
    let f_r_fe: FE = FE::from(&f_r);

    let bn_to_slice = BigInt::to_bytes(chain_code_bi);
    let chain_code = GE::from_bytes(&bn_to_slice[0..33]).unwrap() * &f_r_fe;
    let g: GE = GE::generator().to_point();
    let pub_key = pubkey + &g * &f_l_fe;

    let (public_key_new_child, f_l_new, cc_new) =
        location_in_hir
            .iter()
            .fold((pub_key, f_l_fe, chain_code), |acc, index| {
                let pub_key_bi = &BigInt::from_bytes(&acc.0.to_bytes(true).as_slice());
                let f = create_hmac(
                    &BigInt::from_bytes(&acc.2.to_bytes(true).as_slice()),
                    &[&pub_key_bi, index],
                );
                let f_l = &f >> 256;
                let f_r = &f & &mask;
                let f_l_fe: FE = FE::from(&f_l);
                let f_r_fe: FE = FE::from(&f_r);

                (acc.0 + &g * &f_l_fe, f_l_fe + &acc.1, &acc.2 * &f_r_fe)
            });
    (public_key_new_child, f_l_new, cc_new)
}

fn create_hmac(key: &BigInt, data: &[&BigInt]) -> BigInt {
    let mut key_bytes = key.to_bytes();

    let mut hmac = Hmac::<Sha512>::new_from_slice(&key_bytes).expect("");

    for value in data {
        hmac.update(&BigInt::to_bytes(value));
    }
    key_bytes.zeroize();
    let result = hmac.finalize();
    let code = result.into_bytes();

    BigInt::from_bytes(code.as_slice())
}
