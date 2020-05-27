#include "zendoo_mc.h"
#include "hex_utils.h"
#include <iostream>
#include <cassert>
#include <string>

/*
 *  Usage: ./mcTest <"-v"> "end_epoch_mc_b_hash" "prev_end_epoch_mc_b_hash" "quality" "<constant>" "<proofdata>"
                    "pk_dest_0" "amount_0" "pk_dest_1" "amount_1" ... "pk_dest_n" "amount_n"
 *  "constant" and "proofdata" can be null but the "" must be specified anyway
 */


int main(int argc, char** argv)
{
    int arg = 1;
    bool verify = false;
    if (std::string(argv[1]) == "-v"){
        arg++;
        verify = true;
    }
    // Parse inputs
    assert(IsHex(argv[arg]));
    auto end_epoch_mc_b_hash = ParseHex(argv[arg++]);
    assert(end_epoch_mc_b_hash.size() == 32);

    assert(IsHex(argv[arg]));
    auto prev_end_epoch_mc_b_hash = ParseHex(argv[arg++]);
    assert(prev_end_epoch_mc_b_hash.size() == 32);

    uint64_t quality = strtoull(argv[arg++], NULL, 0);
    assert(quality >= 0);

    auto constant = std::string(argv[arg++]);
    auto proofdata = std::string(argv[arg++]);

    // Deserialize constant and proofdata
    field_t* constant_f = NULL;
    if(constant.size() != 0){
        assert(IsHex(constant));
        auto constant_decoded = ParseHex(constant.c_str());
        assert(constant_decoded.size() == 96);
        constant_f = zendoo_deserialize_field((unsigned char*)constant_decoded.data());
    } else {
        unsigned char zeros[96] = {0};
        constant_f = zendoo_deserialize_field(zeros);
    }
    assert(constant_f != NULL);

    field_t* proofdata_f = NULL;
    if(proofdata.size() != 0){
        assert(IsHex(proofdata));
        auto proofdata_decoded = ParseHex(proofdata.c_str());
        assert(proofdata_decoded.size() == 96);
        proofdata_f = zendoo_deserialize_field((unsigned char*)proofdata_decoded.data());
        assert(proofdata_f != NULL);
    } else {
        unsigned char zeros[96] = {0};
        proofdata_f = zendoo_deserialize_field(zeros);
    }
    assert(proofdata_f != NULL);

    // Create bt_list
    // Inputs must be (pk_dest, amount) pairs from which construct backward_transfer objects
    assert((argc - arg) % 2 == 0);
    int bt_list_length = (argc - arg)/2;
    assert(bt_list_length > 0);

    // Parse backward transfer list
    std::vector<backward_transfer_t> bt_list;
    bt_list.reserve(bt_list_length);
    for(int i = 0; i < bt_list_length; i ++){
        backward_transfer_t bt;

        assert(IsHex(argv[arg]));
        auto pk_dest = ParseHex(argv[arg++]);
        assert(pk_dest.size() == 20);

        uint64_t amount = strtoull(argv[arg++], NULL, 0);
        assert(amount >= 0);
        bt.amount = amount;

        bt_list.push_back(bt);
    }

    // Generate proof and vk
    assert(zendoo_create_mc_test_proof(
        end_epoch_mc_b_hash.data(),
        prev_end_epoch_mc_b_hash.data(),
        bt_list.data(),
        bt_list_length,
        quality,
        constant_f,
        proofdata_f
    ));

    // If -v was specified we verify the proof just created
    if(verify) {

        // Deserialize proof
        sc_proof_t* proof = zendoo_deserialize_sc_proof_from_file(
            (path_char_t*)"./test_mc_proof",
            15
        );

        // Deserialize vk
        sc_vk_t* vk = zendoo_deserialize_sc_vk_from_file(
            (path_char_t*)"./test_mc_vk",
            12
        );

        // Verify proof
        assert(zendoo_verify_sc_proof(
            end_epoch_mc_b_hash.data(),
            prev_end_epoch_mc_b_hash.data(),
            bt_list.data(),
            bt_list_length,
            quality,
            constant_f,
            proofdata_f,
            proof,
            vk
        ));

        zendoo_sc_proof_free(proof);
        zendoo_sc_vk_free(vk);
    }

    if(constant_f != NULL)
        zendoo_field_free(constant_f);

    if(proofdata_f != NULL)
        zendoo_field_free(proofdata_f);
}