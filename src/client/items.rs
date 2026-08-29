// Copyright Materialize, Inc. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE file at the
// root of this repository, or online at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use futures_core::Stream;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::config::ListParams;
use crate::error::Error;
use crate::util::StrIteratorExt;

const ITEMS_PATH: [&str; 1] = ["items"];

/// An Orb item: the product or service a price or credit allocation is for.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Item {
    /// The Orb-assigned unique identifier for the item.
    pub id: String,
    /// The name of the item, e.g. "Business Plan Minimum".
    pub name: String,
}

impl Client {
    /// Gets an item by ID.
    pub async fn get_item(&self, id: &str) -> Result<Item, Error> {
        let req = self.build_request(Method::GET, ITEMS_PATH.chain_one(id));
        self.send_request(req).await
    }

    /// Lists all items.
    ///
    /// The underlying API call is paginated. The returned stream will fetch
    /// additional pages as it is consumed.
    pub fn list_items(&self, params: &ListParams) -> impl Stream<Item = Result<Item, Error>> + '_ {
        let req = self.build_request(Method::GET, ITEMS_PATH);
        self.stream_paginated_request(params, req)
    }
}
