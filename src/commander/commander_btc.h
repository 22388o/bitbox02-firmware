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

#ifndef _COMMANDER_BTC_H_
#define _COMMANDER_BTC_H_

#include "commander.h"

#include <hww.pb.h>

commander_error_t commander_btc_pub(const BTCPubRequest* request, PubResponse* response);

commander_error_t commander_btc_sign(const Request* request, Response* response);
commander_error_t commander_btc_sign_init(const BTCSignInitRequest* request, Response* response);
commander_error_t commander_btc_sign_input(const BTCSignInputRequest* request, Response* response);
commander_error_t commander_btc_sign_output(
    const BTCSignOutputRequest* request,
    Response* response);

commander_error_t commander_btc(const BTCRequest* request, BTCResponse* response);
#endif
