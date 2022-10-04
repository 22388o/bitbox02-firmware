// Copyright 2022 Shift Crypto AG
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

use super::pb;
use super::Error;

use bitbox02::keystore;

use alloc::string::String;
use alloc::vec::Vec;

use bech32::{ToBase32, Variant};

use pb::btc_script_config::SimpleType;
pub use pb::{BtcCoin, BtcOutputType};

use super::{multisig, params::Params};

const HASH160_LEN: usize = 20;
const SHA256_LEN: usize = 32;

/// Converts a satoshi value to a string, suffixed with `unit`, e.g. 1234567890 -> "12.3456789 BTC".
pub fn format_amount(satoshi: u64, unit: &str) -> String {
    let mut s = util::decimal::format(satoshi, 8);
    s.push(' ');
    s.push_str(unit);
    s
}

pub fn payload_simple(
    params: &Params,
    simple_type: SimpleType,
    keypath: &[u32],
) -> Result<Vec<u8>, Error> {
    match simple_type {
        SimpleType::P2wpkh => Ok(keystore::secp256k1_pubkey_hash160(keypath)?.to_vec()),
        SimpleType::P2wpkhP2sh => {
            let payload_p2wpkh = keystore::secp256k1_pubkey_hash160(keypath)?;
            let pkscript_p2wpkh = bitbox02::app_btc::pkscript_from_payload(
                params.taproot_support,
                bitbox02::app_btc::OutputType::OUTPUT_TYPE_P2WPKH,
                &payload_p2wpkh,
            )?;
            Ok(bitbox02::app_btc::hash160(&pkscript_p2wpkh).to_vec())
        }
        SimpleType::P2tr => {
            if params.taproot_support {
                Ok(keystore::secp256k1_schnorr_bip86_pubkey(keypath)?.to_vec())
            } else {
                Err(Error::InvalidInput)
            }
        }
    }
}

pub fn payload_multisig(
    multisig: &pb::btc_script_config::Multisig,
    keypath: &[u32],
) -> Result<Vec<u8>, Error> {
    let script_type = pb::btc_script_config::multisig::ScriptType::from_i32(multisig.script_type)
        .ok_or(Error::InvalidInput)?;
    Ok(bitbox02::app_btc::payload_from_multisig(
        &multisig::convert_multisig(multisig)?,
        multisig::convert_multisig_script_type(script_type),
        keypath[keypath.len() - 2],
        keypath[keypath.len() - 1],
    )?)
}

/// Computes the payload data from a script config. The payload can then be used generate a pkScript
/// or an address.
pub fn payload(
    params: &Params,
    keypath: &[u32],
    script_config_account: &pb::BtcScriptConfigWithKeypath,
) -> Result<Vec<u8>, Error> {
    match script_config_account {
        pb::BtcScriptConfigWithKeypath {
            script_config:
                Some(pb::BtcScriptConfig {
                    config: Some(pb::btc_script_config::Config::SimpleType(simple_type)),
                }),
            ..
        } => {
            let simple_type = pb::btc_script_config::SimpleType::from_i32(*simple_type)
                .ok_or(Error::InvalidInput)?;
            payload_simple(params, simple_type, keypath)
        }
        pb::BtcScriptConfigWithKeypath {
            script_config:
                Some(pb::BtcScriptConfig {
                    config: Some(pb::btc_script_config::Config::Multisig(multisig)),
                }),
            ..
        } => payload_multisig(multisig, keypath),
        _ => Err(Error::InvalidInput),
    }
}

fn encode_segwit_addr(
    hrp: &str,
    witness_version: u8,
    witness_program: &[u8],
) -> Result<String, ()> {
    let variant = match witness_version {
        0 => Variant::Bech32,
        1 => Variant::Bech32m,
        _ => return Err(()),
    };
    let mut b32 = witness_program.to_base32();
    b32.insert(0, bech32::u5::try_from_u8(witness_version).or(Err(()))?);
    bech32::encode(hrp, &b32, variant).or(Err(()))
}

/// Converts a payload to an address. The payload can be obtained from `payload::compute()`.
pub fn address_from_payload(
    params: &Params,
    output_type: BtcOutputType,
    payload: &[u8],
) -> Result<String, ()> {
    match output_type {
        BtcOutputType::Unknown => Err(()),
        BtcOutputType::P2pkh => {
            if payload.len() != HASH160_LEN {
                return Err(());
            }
            Ok(bs58::encode(payload)
                .with_check_version(params.base58_version_p2pkh)
                .into_string())
        }
        BtcOutputType::P2sh => {
            if payload.len() != HASH160_LEN {
                return Err(());
            }
            Ok(bs58::encode(payload)
                .with_check_version(params.base58_version_p2sh)
                .into_string())
        }
        BtcOutputType::P2wpkh => {
            if payload.len() != HASH160_LEN {
                return Err(());
            }
            encode_segwit_addr(params.bech32_hrp, 0, payload)
        }
        BtcOutputType::P2wsh => {
            if payload.len() != SHA256_LEN {
                return Err(());
            }
            encode_segwit_addr(params.bech32_hrp, 0, payload)
        }
        BtcOutputType::P2tr => {
            if !params.taproot_support || payload.len() != 32 {
                return Err(());
            }
            encode_segwit_addr(params.bech32_hrp, 1, payload)
        }
    }
}

pub fn determine_output_type_from_simple_type(simple_type: SimpleType) -> BtcOutputType {
    match simple_type {
        SimpleType::P2wpkhP2sh => BtcOutputType::P2sh,
        SimpleType::P2wpkh => BtcOutputType::P2wpkh,
        SimpleType::P2tr => BtcOutputType::P2tr,
    }
}

pub fn determine_output_type_multisig(
    script_type: pb::btc_script_config::multisig::ScriptType,
) -> BtcOutputType {
    match script_type {
        pb::btc_script_config::multisig::ScriptType::P2wsh => BtcOutputType::P2wsh,
        pb::btc_script_config::multisig::ScriptType::P2wshP2sh => BtcOutputType::P2sh,
    }
}

/// Determine the output type from the given an input script config.
pub fn determine_output_type(script_config: &pb::BtcScriptConfig) -> Result<BtcOutputType, Error> {
    match script_config {
        pb::BtcScriptConfig {
            config: Some(pb::btc_script_config::Config::SimpleType(simple_type)),
        } => {
            let simple_type = SimpleType::from_i32(*simple_type).ok_or(Error::InvalidInput)?;
            Ok(determine_output_type_from_simple_type(simple_type))
        }
        pb::BtcScriptConfig {
            config: Some(pb::btc_script_config::Config::Multisig(multisig)),
        } => {
            let script_type =
                pb::btc_script_config::multisig::ScriptType::from_i32(multisig.script_type)
                    .ok_or(Error::InvalidInput)?;
            Ok(determine_output_type_multisig(script_type))
        }
        _ => Err(Error::InvalidInput),
    }
}

/// Converts a Rust protobuf OutputType to a representation suitable to be passed to C functions.
pub fn convert_output_type(simple_type: BtcOutputType) -> bitbox02::app_btc::OutputType {
    match simple_type {
        BtcOutputType::Unknown => bitbox02::app_btc::OutputType::OUTPUT_TYPE_UNKNOWN,
        BtcOutputType::P2pkh => bitbox02::app_btc::OutputType::OUTPUT_TYPE_P2PKH,
        BtcOutputType::P2sh => bitbox02::app_btc::OutputType::OUTPUT_TYPE_P2SH,
        BtcOutputType::P2wpkh => bitbox02::app_btc::OutputType::OUTPUT_TYPE_P2WPKH,
        BtcOutputType::P2wsh => bitbox02::app_btc::OutputType::OUTPUT_TYPE_P2WSH,
        BtcOutputType::P2tr => bitbox02::app_btc::OutputType::OUTPUT_TYPE_P2TR,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use bitbox02::testing::mock_unlocked_using_mnemonic;
    use util::bip32::HARDENED;

    #[test]
    fn test_address_from_payload() {
        let params_btc = super::super::params::get(pb::BtcCoin::Btc);
        let params_ltc = super::super::params::get(pb::BtcCoin::Ltc);
        {
            // BTC & LTC p2pkh

            let payload =
                b"\x67\xfe\x0b\xdd\xe7\x98\x46\x71\xf2\xed\x59\xbb\x68\xa9\x7d\x9c\xc6\x8a\x02\xe0";
            assert_eq!(
                address_from_payload(params_btc, BtcOutputType::P2pkh, payload),
                Ok("1AUrwD77AL5ax5zj2BhZQ1x43wA5NLsYg1".into())
            );
            assert_eq!(
                address_from_payload(params_ltc, BtcOutputType::P2pkh, payload),
                Ok("LUhpCRQwEzKeCtgtCKgrg31pG9XMZLm6qX".into())
            );
        }
        {
            // BTC & LTC p2wpkh

            let payload =
                b"\x3f\x0d\xc2\xe9\x14\x2d\x88\x39\xae\x9c\x90\xa1\x9c\xa8\x6c\x36\xd9\x23\xd8\xab";
            assert_eq!(
                address_from_payload(params_btc, BtcOutputType::P2wpkh, payload),
                Ok("bc1q8uxu96g59kyrnt5ujzsee2rvxmvj8k9trg5ltx".into())
            );
            assert_eq!(
                address_from_payload(params_ltc, BtcOutputType::P2wpkh, payload),
                Ok("ltc1q8uxu96g59kyrnt5ujzsee2rvxmvj8k9t85wmnk".into())
            );
        }

        {
            // BTC & LTC p2sh

            let payload =
                b"\x8d\xd0\x9c\x25\xc9\x28\xbe\x67\x66\xf4\x50\x73\x87\x0c\xe3\xbb\x93\x1f\x2f\x55";
            assert_eq!(
                address_from_payload(params_btc, BtcOutputType::P2sh, payload),
                Ok("3Ecs74kCeeAc6EKWMGe7RXupUoeeXPdyj7".into())
            );
            assert_eq!(
                address_from_payload(params_ltc, BtcOutputType::P2sh, payload),
                Ok("MLq1QxAAbm22tjbQT9dTFBADoWF6UwYB7R".into())
            );
        }

        {
            // BTC & LTC p2wsh

            let payload = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
            assert_eq!(
                address_from_payload(params_btc, BtcOutputType::P2wsh, payload),
                Ok("bc1qv9skzctpv9skzctpv9skzctpv9skzctpv9skzctpv9skzctpv9ss52vqes".into())
            );
            assert_eq!(
                address_from_payload(params_ltc, BtcOutputType::P2wsh, payload),
                Ok("ltc1qv9skzctpv9skzctpv9skzctpv9skzctpv9skzctpv9skzctpv9sshwzsr4".into())
            );
        }

        // Taproot addresses, test vectors from
        // https://github.com/bitcoin/bips/blob/fb5bd37d0cdec14b47c45fda7aba4f7e8f801690/bip-0086.mediawiki#Test_vectors
        {
            // First receiving address

            let payload = b"\xa6\x08\x69\xf0\xdb\xcf\x1d\xc6\x59\xc9\xce\xcb\xaf\x80\x50\x13\x5e\xa9\xe8\xcd\xc4\x87\x05\x3f\x1d\xc6\x88\x09\x49\xdc\x68\x4c";
            assert!(address_from_payload(params_ltc, BtcOutputType::P2tr, payload).is_err());
            assert_eq!(
                address_from_payload(params_btc, BtcOutputType::P2tr, payload),
                Ok("bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr".into())
            );
        }
        {
            // Second receiving address

            let payload = b"\xa8\x2f\x29\x94\x4d\x65\xb8\x6a\xe6\xb5\xe5\xcc\x75\xe2\x94\xea\xd6\xc5\x93\x91\xa1\xed\xc5\xe0\x16\xe3\x49\x8c\x67\xfc\x7b\xbb";
            assert!(address_from_payload(params_ltc, BtcOutputType::P2tr, payload).is_err());
            assert_eq!(
                address_from_payload(params_btc, BtcOutputType::P2tr, payload),
                Ok("bc1p4qhjn9zdvkux4e44uhx8tc55attvtyu358kutcqkudyccelu0was9fqzwh".into())
            );
        }
        {
            // First change address

            let payload = b"\x88\x2d\x74\xe5\xd0\x57\x2d\x5a\x81\x6c\xef\x00\x41\xa9\x6b\x6c\x1d\xe8\x32\xf6\xf9\x67\x6d\x96\x05\xc4\x4d\x5e\x9a\x97\xd3\xdc";
            assert!(address_from_payload(params_ltc, BtcOutputType::P2tr, payload).is_err());
            assert_eq!(
                address_from_payload(params_btc, BtcOutputType::P2tr, payload),
                Ok("bc1p3qkhfews2uk44qtvauqyr2ttdsw7svhkl9nkm9s9c3x4ax5h60wqwruhk7".into())
            );
        }
    }

    #[test]
    fn test_format_amount() {
        let tests: Vec<(u64, &str)> = vec![
            (1234567890, "12.3456789 LOL"),
            (0, "0 LOL"),
            (1, "0.00000001 LOL"),
            (2, "0.00000002 LOL"),
            (10, "0.0000001 LOL"),
            (15, "0.00000015 LOL"),
            (20, "0.0000002 LOL"),
            (300, "0.000003 LOL"),
            (370, "0.0000037 LOL"),
            (371, "0.00000371 LOL"),
            (40000000000, "400 LOL"),
            (4000000000, "40 LOL"),
            (400000000, "4 LOL"),
            (40000000, "0.4 LOL"),
            (4000000, "0.04 LOL"),
            (400000, "0.004 LOL"),
            (40000, "0.0004 LOL"),
            (4000, "0.00004 LOL"),
            (400, "0.000004 LOL"),
            (40, "0.0000004 LOL"),
            (4, "0.00000004 LOL"),
            (5432345, "0.05432345 LOL"),
            (54323452, "0.54323452 LOL"),
            (543234527, "5.43234527 LOL"),
            (5432345270, "54.3234527 LOL"),
            (54323452708, "543.23452708 LOL"),
            (100000000, "1 LOL"),
            (1234567800000001, "12345678.00000001 LOL"),
            (0xffffffffffffffff, "184467440737.09551615 LOL"),
            (0xffffffffffffffff - 5, "184467440737.0955161 LOL"),
        ];
        for (satoshi, expected) in tests {
            assert_eq!(format_amount(satoshi, "LOL"), expected);
        }
    }

    #[test]
    fn test_payload_simple() {
        mock_unlocked_using_mnemonic(
            "sudden tenant fault inject concert weather maid people chunk youth stumble grit",
        );
        let coin_params = super::super::params::get(pb::BtcCoin::Btc);
        // p2wpkh
        assert_eq!(
            payload_simple(
                coin_params,
                SimpleType::P2wpkh,
                &[84 + HARDENED, 0 + HARDENED, 0 + HARDENED, 0, 0]
            )
            .unwrap()
            .as_slice(),
            b"\x3f\x0d\xc2\xe9\x14\x2d\x88\x39\xae\x9c\x90\xa1\x9c\xa8\x6c\x36\xd9\x23\xd8\xab"
        );

        //  p2wpkh-p2sh
        assert_eq!(
            payload_simple(
                coin_params,
                SimpleType::P2wpkhP2sh,
                &[49 + HARDENED, 0 + HARDENED, 0 + HARDENED, 0, 0]
            )
            .unwrap()
            .as_slice(),
            b"\x8d\xd0\x9c\x25\xc9\x28\xbe\x67\x66\xf4\x50\x73\x87\x0c\xe3\xbb\x93\x1f\x2f\x55"
        );

        // p2tr
        assert_eq!(
            payload_simple(
                coin_params,
                SimpleType::P2tr,
                &[86 + HARDENED, 0 + HARDENED, 0 + HARDENED, 0, 0]
            )
            .unwrap()
            .as_slice(),
            b"\x25\x0e\xc8\x02\xb6\xd3\xdb\x98\x42\xd1\xbd\xbe\x0e\xe4\x8d\x52\xf9\xa4\xb4\x6e\x60\xcb\xbb\xab\x3b\xcc\x4e\xe9\x15\x73\xfc\xe8"
        );
    }
}
