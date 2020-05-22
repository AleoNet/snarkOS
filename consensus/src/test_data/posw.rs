use once_cell::sync::Lazy;
use rand::SeedableRng;
use rand_xorshift::XorShiftRng;
use snarkos_algorithms::snark::generate_random_parameters;
use snarkos_objects::pedersen_merkle_tree::PARAMS;
use std::marker::PhantomData;

use snarkos_models::storage::Storage;
use snarkos_posw::{ProvingKey, VerifyingKey, POSW};
use snarkos_profiler::{end_timer, start_timer};

// Public parameters for the POSW SNARK
pub static POSW_PP: Lazy<(ProvingKey, VerifyingKey)> = Lazy::new(|| {
    let mut path = std::env::current_dir().unwrap();
    path.push("../consensus/src/test_data/");
    let test_pk_path = path.clone().join("test_posw.params");
    let test_vk_path = path.clone().join("test_posw_vk.params");
    let generation_timer = start_timer!(|| "POSW setup");

    let (params, vk) = if test_pk_path.exists() {
        let vk = VerifyingKey::load(&test_vk_path).unwrap();
        let pk = ProvingKey::load(&test_pk_path).unwrap();

        (pk, vk)
    } else {
        let params = generate_random_parameters(
            POSW {
                leaves: vec![None; 0],
                merkle_parameters: PARAMS.clone(),
                mask: None,
                root: None,
                field_type: PhantomData,
                crh_gadget_type: PhantomData,
                circuit_parameters_type: PhantomData,
            },
            &mut XorShiftRng::seed_from_u64(1234567),
        )
        .unwrap();

        let vk = params.vk.clone();

        params.store(&test_pk_path).unwrap();
        vk.store(&test_vk_path).unwrap();

        (params, vk)
    };

    end_timer!(generation_timer);
    (params, vk)
});
