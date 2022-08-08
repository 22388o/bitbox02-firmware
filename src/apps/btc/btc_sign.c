// Copyright 2019 Shift Cryptosecurity AG
// Copyright 2020 Shift Crypto AG
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

#include "btc_sign.h"
#include "btc_common.h"

#include <rust/rust.h>

#include <hardfault.h>
#include <keystore.h>
#include <ui/screen_stack.h>
#include <util.h>

#include <wally_script.h>
#include <wally_transaction.h>

#include <pb_decode.h>

// Inputs and changes must be of a type defined in _init_request.script_configs.
// Inputs and changes keypaths must have the prefix as defined in the referenced script_config..
static BTCSignInitRequest _init_request = {0};

// Must be called in any code path that exits the signing process (error or regular finish).
static void _reset(void)
{
    util_zero(&_init_request, sizeof(_init_request));
}

static app_btc_result_t _error(app_btc_result_t err)
{
    _reset();
    return err;
}

app_btc_result_t app_btc_sign_init(const BTCSignInitRequest* request)
{
    _reset();
    _init_request = *request;
    return APP_BTC_OK;
}

app_btc_result_t app_btc_sign_sighash_script(
    const BTCSignInputRequest* request,
    uint8_t* sighash_script,
    size_t* sighash_script_size)
{
    uint8_t pubkey_hash160[HASH160_LEN];
    UTIL_CLEANUP_20(pubkey_hash160);
    if (!keystore_secp256k1_pubkey_hash160(
            request->keypath, request->keypath_count, pubkey_hash160)) {
        return APP_BTC_ERR_UNKNOWN;
    }

    const BTCScriptConfig* script_config_account =
        &_init_request.script_configs[request->script_config_index].script_config;

    switch (script_config_account->which_config) {
    case BTCScriptConfig_simple_type_tag:
        if (!btc_common_sighash_script_from_pubkeyhash(
                script_config_account->config.simple_type,
                pubkey_hash160,
                sighash_script,
                sighash_script_size)) {
            return APP_BTC_ERR_INVALID_INPUT;
        }
        break;
    case BTCScriptConfig_multisig_tag: {
        uint8_t sighash_script_tmp[MAX_PK_SCRIPT_SIZE] = {0};
        size_t sighash_script_size_tmp = sizeof(sighash_script_tmp);
        const BTCScriptConfig_Multisig* ms = &script_config_account->config.multisig;
        multisig_t multisig = {0};
        if (!btc_common_convert_multisig(ms, &multisig)) {
            return APP_BTC_ERR_INVALID_INPUT;
        }
        if (!btc_common_pkscript_from_multisig(
                &multisig,
                request->keypath[request->keypath_count - 2],
                request->keypath[request->keypath_count - 1],
                sighash_script_tmp,
                &sighash_script_size_tmp)) {
            return APP_BTC_ERR_INVALID_INPUT;
        }
        if (wally_varbuff_to_bytes(
                sighash_script_tmp,
                sighash_script_size_tmp,
                sighash_script,
                *sighash_script_size,
                sighash_script_size) != WALLY_OK) {
            return APP_BTC_ERR_UNKNOWN;
        }
        break;
    }
    default:
        return APP_BTC_ERR_INVALID_INPUT;
    }
    return APP_BTC_OK;
}

app_btc_result_t app_btc_sign_init_wrapper(in_buffer_t request_buf)
{
    pb_istream_t in_stream = pb_istream_from_buffer(request_buf.data, request_buf.len);
    BTCSignInitRequest request = {0};
    if (!pb_decode(&in_stream, BTCSignInitRequest_fields, &request)) {
        return _error(APP_BTC_ERR_UNKNOWN);
    }
    return app_btc_sign_init(&request);
}

app_btc_result_t app_btc_sign_sighash_script_wrapper(
    in_buffer_t request_buf,
    uint8_t* sighash_script,
    size_t* sighash_script_size)
{
    pb_istream_t in_stream = pb_istream_from_buffer(request_buf.data, request_buf.len);
    BTCSignInputRequest request = {0};
    if (!pb_decode(&in_stream, BTCSignInputRequest_fields, &request)) {
        return _error(APP_BTC_ERR_UNKNOWN);
    }
    return app_btc_sign_sighash_script(&request, sighash_script, sighash_script_size);
}

void app_btc_sign_reset(void)
{
    _reset();
}
