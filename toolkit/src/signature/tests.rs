// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use std::str::FromStr;

// Test the signature scheme derived from an Account Private Key `sk_sig` and `pk_sig`
pub mod private {
    use super::*;
    use crate::{
        account::PrivateKey,
        signature::private::{Signature, SignaturePublicKey},
    };

    #[test]
    pub fn test_signature_public_key() {
        let private_key = PrivateKey::from_str("APrivateKey1tvv5YV1dipNiku2My8jMkqpqCyYKvR5Jq4y2mtjw7s77Zpn").unwrap();
        let public_key = SignaturePublicKey::from(&private_key);
        assert!(public_key.is_ok());

        let expected_public_key = "17e858cfba9f42335bd7d4751f9284671f913d841325ce548f98ae46d480211038530919083215e5376a472a61eefad25b545d3b75d43c8e2f8f821a17500103";
        let candidate_public_key = public_key.unwrap().to_string();

        println!("{} == {}", expected_public_key, candidate_public_key);
        assert_eq!(expected_public_key, candidate_public_key);
    }

    #[test]
    pub fn test_signature() {
        let rng = &mut ChaChaRng::seed_from_u64(1231275789u64);
        let private_key = PrivateKey::new(rng);
        assert!(private_key.is_ok());

        let message: [u8; 32] = rng.gen();

        let signature = Signature::sign(&private_key.unwrap(), &message, rng);
        assert!(signature.is_ok());

        let expected_signature = "41fdc76a826b157b895012fc0bd840b65eaec5b69e9d33141960ee61b0ccdd00d0f3be67419c660afed7cd807a94396ff93864fb149c0a39148036da8c9eaa02";
        let candidate_signature = signature.unwrap().to_string();

        println!("{} == {}", expected_signature, candidate_signature);
        assert_eq!(expected_signature, candidate_signature);
    }

    #[test]
    pub fn test_signature_verification() {
        let rng = &mut ChaChaRng::seed_from_u64(1231275789u64);

        let message: [u8; 32] = rng.gen();

        let private_key = PrivateKey::from_str("APrivateKey1tvv5YV1dipNiku2My8jMkqpqCyYKvR5Jq4y2mtjw7s77Zpn").unwrap();
        let public_key = SignaturePublicKey::from(&private_key);
        assert!(public_key.is_ok());

        let signature = Signature::sign(&private_key, &message, rng);
        assert!(signature.is_ok());

        let verification = signature.unwrap().verify(&public_key.unwrap(), &message);
        assert!(verification.is_ok());
        assert!(verification.unwrap())
    }

    #[test]
    pub fn test_failed_signature_verification() {
        let rng = &mut ChaChaRng::seed_from_u64(1231275789u64);

        let message: [u8; 32] = rng.gen();

        let private_key = PrivateKey::from_str("APrivateKey1tvv5YV1dipNiku2My8jMkqpqCyYKvR5Jq4y2mtjw7s77Zpn").unwrap();
        let public_key = SignaturePublicKey::from(&private_key);
        assert!(public_key.is_ok());

        let signature = Signature::sign(&private_key, &message, rng);
        assert!(signature.is_ok());

        let bad_message: [u8; 32] = rng.gen();

        let verification = signature.unwrap().verify(&public_key.unwrap(), &bad_message);
        assert!(verification.is_ok());
        assert!(!verification.unwrap())
    }
}

// Test the signature scheme derived from the Account View Key and Account Address
pub mod public {
    use super::*;
    use crate::{
        account::{Address, PrivateKey, ViewKey},
        signature::public::Signature,
    };

    #[test]
    pub fn test_signature() {
        let rng = &mut ChaChaRng::seed_from_u64(1231275789u64);
        let private_key = PrivateKey::new(rng);
        assert!(private_key.is_ok());

        let view_key = ViewKey::from(&private_key.unwrap());
        assert!(view_key.is_ok());

        let message: [u8; 32] = rng.gen();

        let signature = Signature::sign(&view_key.unwrap(), &message, rng);
        assert!(signature.is_ok());

        let expected_signature = "672cfc66d9d2c018fbac1a9d8245d3f1ed5ab2485031e64eaeeaf0c09c8cab03a4474f5d8f9f7108cce355c8c4509e3c625b78ccc63a0f203bf81a3493ce7c03";
        let candidate_signature = signature.unwrap().to_string();

        println!("{} == {}", expected_signature, candidate_signature);
        assert_eq!(expected_signature, candidate_signature);
    }

    #[test]
    pub fn test_signature_verification() {
        let rng = &mut ChaChaRng::seed_from_u64(1231275789u64);
        let private_key = PrivateKey::new(rng);
        assert!(private_key.is_ok());

        let view_key = ViewKey::from(&private_key.unwrap()).unwrap();
        let address = Address::from_view_key(&view_key).unwrap();

        let message: [u8; 32] = rng.gen();

        let signature = Signature::sign(&view_key, &message, rng);
        assert!(signature.is_ok());

        let verification = signature.unwrap().verify(&address, &message);
        assert!(verification.is_ok());
        assert!(verification.unwrap())
    }

    #[test]
    pub fn test_failed_signature_verification() {
        let rng = &mut ChaChaRng::seed_from_u64(1231275789u64);
        let private_key = PrivateKey::new(rng);
        assert!(private_key.is_ok());

        let view_key = ViewKey::from(&private_key.unwrap()).unwrap();
        let address = Address::from_view_key(&view_key).unwrap();

        let message: [u8; 32] = rng.gen();

        let signature = Signature::sign(&view_key, &message, rng);
        assert!(signature.is_ok());

        let bad_message: [u8; 32] = rng.gen();

        let verification = signature.unwrap().verify(&address, &bad_message);
        assert!(verification.is_ok());
        assert!(!verification.unwrap())
    }
}
