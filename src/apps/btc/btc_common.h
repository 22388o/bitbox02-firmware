// Copyright 2019 Shift Cryptosecurity AG
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#ifndef _APPS_BTC_COMMON_H
#define _APPS_BTC_COMMON_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include <compiler_util.h>
#include <keystore.h>

#include <hww.pb.h>

#include <wally_bip32.h>
#include <wally_crypto.h>
#include <wally_script.h>

#define MULTISIG_P2WSH_MAX_SIGNERS 15

typedef struct {
    size_t xpubs_count;
    uint8_t xpubs[MULTISIG_P2WSH_MAX_SIGNERS][BIP32_SERIALIZED_LEN];
    uint32_t threshold;
} multisig_t;

// see https://en.bitcoin.it/wiki/Protocol_documentation#Variable_length_integer
#define MAX_VARINT_SIZE (9)
// current expected max pk script size is a m-of-15 multisig. 700 is also enough for m-of-20, which
// is technically possible to extend to if needed.
#define MAX_PK_SCRIPT_SIZE (700)

/**
 * Generate the payload used in an output script, e.g. pubkeyhash or script hash or pubkey.
 * @param[in] keypath address-level keypath, e.g. m/84'/0'/0'/0/0
 * @param[in] keypath_len number of elements in keypath
 * @param[in] script_type script type defining the payload.
 * @param[out] output_payload will have the resulting payload. Must be of size 32.
 * @param[out] output_payload_size will be 32 for p2wsh scripts, HASH160_LEN for
 * all others.
 * return true on succes, false on failure.
 */
USE_RESULT bool btc_common_payload_at_keypath(
    const uint32_t* keypath,
    size_t keypath_len,
    BTCScriptConfig_SimpleType script_type,
    uint8_t* output_payload,
    size_t* output_payload_size);

/**
 * Computes the pkScript from a pubkey hash or script hash or pubkey, depending on the output
 * type.
 * @param[in] output_type type of pkScript.
 * @param[in] payload pubkey hash or script hash or pubkey.
 * @param[inout] pk_script_len: size of pk_script IN, size of the resulting pk_script OUT.
 */
USE_RESULT bool btc_common_pkscript_from_payload(
    bool taproot_support,
    BTCOutputType output_type,
    const uint8_t* payload,
    size_t payload_size,
    uint8_t* pk_script,
    size_t* pk_script_len);

/**
 * Creates a n-of-m multisig script based on OP_CHECKMULTISIG. 0<n<=m<=15.
 * Note that the multisig config and keypaths are *not* validated, this must be done before calling.
 * @param[in] multisig Multisig configuration (threshold, signers). The xpubs are account-level
 * xpubs.
 * @param[in] keypath_change 0 for receive addresses, 1 for change addresses.
 * @param[in] keypath_address receive address index.
 * @param[out] script_out script to be created. Must be at least 517 bytes.
 * @param[out] script_out_size The size of the generated script.
 * @return true on success, false on failure.
 */
USE_RESULT bool btc_common_pkscript_from_multisig(
    const multisig_t* multisig,
    uint32_t keypath_change,
    uint32_t keypath_address,
    uint8_t* script_out,
    size_t* script_out_size);

/**
 * Constructs sha256(<multisig pkScript>) from the provided multisig.
 * Note that the multisig config and keypaths are *not* validated, this must be done before calling.
 * @param[in] multisig Multisig configuration (threshold, signers). The xpubs are account-level
 * xpubs.
 * @param[in] keypath_change 0 for receive addresses, 1 for change addresses.
 * @param[in] keypath_address receive address index.
 * @param[out] output_payload result, must be at least `SHA256_LEN` bytes.
 * @param[out] output_payload_size of the output hash. Will be `SHA256_LEN` for P2WSH and
 * `HASH160_LEN` for P2WSH-P2SH.
 */
USE_RESULT bool btc_common_payload_from_multisig(
    const multisig_t* multisig,
    BTCScriptConfig_Multisig_ScriptType script_type,
    uint32_t keypath_change,
    uint32_t keypath_address,
    uint8_t* output_payload,
    size_t* output_payload_size);

#endif
