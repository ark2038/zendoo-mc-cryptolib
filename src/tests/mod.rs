use algebra::{
    bytes::{FromBytes, ToBytes},
    curves::mnt4753::MNT4 as PairingCurve,
    fields::mnt4753::Fr,
    to_bytes, UniformRand,
};

use proof_systems::groth16::Proof;
use rand::rngs::OsRng;

use crate::{
    zendoo_deserialize_field, zendoo_deserialize_sc_proof, zendoo_verify_sc_proof, zendoo_serialize_field,
    ginger_mt_new, ginger_mt_get_root, ginger_mt_get_merkle_path, ginger_mt_verify_merkle_path,
    GingerMerkleTree, ginger_mt_free, ginger_mt_path_free, zendoo_sc_proof_free, zendoo_field_free,
    BackwardTransfer, zendoo_compute_poseidon_hash, zendoo_field_assert_eq,
    zendoo_deserialize_sc_vk_from_file, zendoo_sc_vk_free, zendoo_serialize_sc_proof,
};

use std::{fmt::Debug, fs::File, ptr::null};

fn assert_slice_equals<T: Eq + Debug>(s1: &[T], s2: &[T]) {
    for (i1, i2) in s1.iter().zip(s2.iter()) {
        assert_eq!(i1, i2);
    }
}

#[cfg(target_os = "windows")]
use std::ffi::OsString;
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;

#[cfg(not(target_os = "windows"))]
fn path_as_ptr(path: &str) -> *const u8 { path.as_ptr() }

#[cfg(target_os = "windows")]
fn path_as_ptr(path: &str) -> *const u16 {
    let tmp: Vec<u16> = OsString::from(path).encode_wide().collect();
    tmp.as_ptr()
}

#[test]
fn verify_zkproof_test() {

    let mut file = File::open("./test_files/sample_proof").unwrap();
    let proof = Proof::<PairingCurve>::read(&mut file).unwrap();

    //Create inputs for Rust FFI function
    //Positive case
    let mut zkp = [0u8; 771];

    //Get zkp raw pointer
    proof.write(&mut zkp[..]).unwrap();
    let zkp_ptr = zendoo_deserialize_sc_proof(&zkp);
    let mut zkp_serialized = [0u8; 771];

    //Test proof serialization/deserialization
    zendoo_serialize_sc_proof(zkp_ptr, &mut zkp_serialized);
    assert_slice_equals(&zkp, &zkp_serialized);
    drop(zkp_serialized);

    //Inputs
    let end_epoch_mc_b_hash: [u8; 32] = [
        78, 85, 161, 67, 167, 192, 185, 56, 133, 49, 134, 253, 133, 165, 182, 80, 152, 93, 203, 77,
        165, 13, 67, 0, 64, 200, 185, 46, 93, 135, 238, 70
    ];

    let prev_end_epoch_mc_b_hash: [u8; 32] = [
        68, 214, 34, 70, 20, 109, 48, 39, 210, 156, 109, 60, 139, 15, 102, 79, 79, 2, 87, 190, 118,
        38, 54, 18, 170, 67, 212, 205, 183, 115, 182, 198
    ];

    let constant_bytes: [u8; 96] = [
        170, 190, 140, 27, 234, 135, 240, 226, 158, 16, 29, 161, 178, 36, 69, 34, 29, 75, 195, 247,
        29, 93, 92, 48, 214, 102, 70, 134, 68, 165, 170, 201, 119, 162, 19, 254, 229, 115, 80, 248,
        106, 182, 164, 40, 21, 154, 15, 177, 158, 16, 172, 169, 189, 253, 206, 182, 72, 183, 128,
        160, 182, 39, 98, 76, 95, 198, 62, 39, 87, 213, 251, 12, 154, 180, 125, 231, 222, 73, 129,
        120, 144, 197, 116, 248, 95, 206, 147, 108, 252, 125, 79, 118, 57, 26, 0, 0
    ];

    let constant = zendoo_deserialize_field(&constant_bytes);
    drop(constant_bytes);

    let quality = 2;

    //Create dummy bt
    let bt_num = 10;
    let mut bt_list = vec![];
    for _ in 0..bt_num {
        bt_list.push(BackwardTransfer {
            pk_dest: [0u8; 20],
            amount: 0,
        });
    }

    //Get vk
    let vk = zendoo_deserialize_sc_vk_from_file(
        path_as_ptr("./test_files/sample_vk"),
        22,
    );

    assert!(zendoo_verify_sc_proof(
        &end_epoch_mc_b_hash,
        &prev_end_epoch_mc_b_hash,
        bt_list.as_ptr(),
        bt_num,
        quality,
        constant,
        null(),
        zkp_ptr,
        vk
    ));

    //Negative test: change one of the inputs and assert verification failure

    assert!(!zendoo_verify_sc_proof(
        &end_epoch_mc_b_hash,
        &prev_end_epoch_mc_b_hash,
        bt_list.as_ptr(),
        bt_num,
        quality - 1,
        constant,
        null(),
        zkp_ptr,
        vk
    ));

    //Free memory
    zendoo_sc_proof_free(zkp_ptr);
    zendoo_sc_vk_free(vk);
    zendoo_field_free(constant);
}

#[test]
fn merkle_tree_test() {
    let mut rng = OsRng::default();

    //Generate random field elements
    let mut fes = vec![];
    for _ in 0..16 {
        fes.push(Fr::rand(&mut rng));
    }

    //Get native Merkle Tree
    let native_tree = GingerMerkleTree::new(fes.as_slice()).unwrap();

    //Get Merkle Tree from lib
    let mut fes_ptr = vec![];
    let fes_b = to_bytes!(fes).unwrap();
    for i in 0..16 {
        let mut fe = [0u8; 96];
        fes_b[(i * 96)..((i + 1) * 96)]
            .to_vec()
            .write(&mut fe[..])
            .unwrap();
        fes_ptr.push(zendoo_deserialize_field(&fe) as *const Fr)
    }
    let tree = ginger_mt_new(fes_ptr.as_ptr(), 16);

    //Get root and compare the two trees
    let root = ginger_mt_get_root(tree);

    assert_eq!(unsafe { *root }, native_tree.root());

    for i in 0..16 {
        //Get native Merkle Path for a leaf
        let native_mp = native_tree.generate_proof(i, &fes[i]).unwrap();

        //Get Merkle Path from lib
        let path = ginger_mt_get_merkle_path(fes_ptr[i], i, tree);

        for (native_path, path) in native_mp.path.iter().zip(unsafe { &*path }.path.iter()) {
            assert_eq!(native_path, path);
        }

        //Verify that both merkle paths are correct
        assert!(native_mp.verify(&native_tree.root(), &fes[i]).unwrap());
        assert!(ginger_mt_verify_merkle_path(fes_ptr[i], root, path));

        //Free path
        ginger_mt_path_free(path);
    }

    //Free memory
    ginger_mt_free(tree);
    zendoo_field_free(root);
    for i in 0..16 {
        zendoo_field_free(fes_ptr[i] as *mut Fr);
    }
}

#[test]
fn poseidon_hash_test() {
    let lhs: [u8; 96] = [
        138, 206, 199, 243, 195, 254, 25, 94, 236, 155, 232, 182, 89, 123, 162, 207, 102, 52, 178,
        128, 55, 248, 234, 95, 33, 196, 170, 12, 118, 16, 124, 96, 47, 203, 160, 167, 144, 153,
        161, 86, 213, 126, 95, 76, 27, 98, 34, 111, 144, 36, 205, 124, 200, 168, 29, 196, 67, 210,
        100, 154, 38, 79, 178, 191, 246, 115, 84, 232, 87, 12, 34, 72, 88, 23, 236, 142, 237, 45,
        11, 148, 91, 112, 156, 47, 68, 229, 216, 56, 238, 98, 41, 243, 225, 192, 0, 0,
    ];

    let rhs: [u8; 96] = [
        199, 130, 235, 52, 44, 219, 5, 195, 71, 154, 54, 121, 3, 11, 111, 160, 86, 212, 189, 66,
        235, 236, 240, 242, 126, 248, 116, 0, 48, 95, 133, 85, 73, 150, 110, 169, 16, 88, 136, 34,
        106, 7, 38, 176, 46, 89, 163, 49, 162, 222, 182, 42, 200, 240, 149, 226, 173, 203, 148,
        194, 207, 59, 44, 185, 67, 134, 107, 221, 188, 208, 122, 212, 200, 42, 227, 3, 23, 59, 31,
        37, 91, 64, 69, 196, 74, 195, 24, 5, 165, 25, 101, 215, 45, 92, 1, 0,
    ];

    let hash: [u8; 96] = [
        53, 2, 235, 12, 255, 18, 125, 167, 223, 32, 245, 103, 38, 74, 43, 73, 254, 189, 174, 137,
        20, 90, 195, 107, 202, 24, 151, 136, 85, 23, 9, 93, 207, 33, 229, 200, 178, 225, 221, 127,
        18, 250, 108, 56, 86, 94, 171, 1, 76, 21, 237, 254, 26, 235, 196, 14, 18, 129, 101, 158,
        136, 103, 147, 147, 239, 140, 163, 94, 245, 147, 110, 28, 93, 231, 66, 7, 111, 11, 202, 99,
        146, 211, 117, 143, 224, 99, 183, 108, 157, 200, 119, 169, 180, 148, 0, 0,
    ];

    let lhs_field = zendoo_deserialize_field(&lhs);
    let rhs_field = zendoo_deserialize_field(&rhs);
    let expected_hash = zendoo_deserialize_field(&hash);

    //Test field serialization/deserialization
    let mut lhs_serialized = [0u8; 96];
    zendoo_serialize_field(lhs_field, &mut lhs_serialized);
    assert_slice_equals(&lhs, &lhs_serialized);
    drop(lhs_serialized);

    let hash_input = &[lhs_field as *const Fr, rhs_field as *const Fr];
    let actual_hash = zendoo_compute_poseidon_hash(hash_input.as_ptr(), 2);

    assert!(zendoo_field_assert_eq(expected_hash, actual_hash));

    zendoo_field_free(lhs_field);
    zendoo_field_free(rhs_field);
    zendoo_field_free(expected_hash);
    zendoo_field_free(actual_hash);
}
