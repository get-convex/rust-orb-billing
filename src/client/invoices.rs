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

use std::collections::BTreeMap;

use futures_core::Stream;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::client::customers::CustomerId;
use crate::client::Client;
use crate::config::ListParams;
use crate::error::Error;
use crate::util::StrIteratorExt;

const INVOICES: [&str; 1] = ["invoices"];

/// An Orb invoice.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Invoice {
    /// The Orb-assigned unique identifier for the invoice.
    pub id: String,
    /// The customer to whom this invoice was issued.
    pub customer: InvoiceCustomer,
    /// The subscription associated with this invoice.
    pub subscription: Option<InvoiceSubscription>,
    /// The issue date of the invoice.
    #[serde(with = "time::serde::rfc3339")]
    pub invoice_date: OffsetDateTime,
    /// An automatically generated number to help track and reconcile invoices.
    pub invoice_number: String,
    /// The link to download the PDF representation of the invoice.
    pub invoice_pdf: Option<String>,
    /// An ISO 4217 currency string, or "credits"
    pub currency: String,
    /// The total after any minimums, discounts, and taxes have been applied.
    pub total: String,
    /// This is the final amount required to be charged to the
    /// customer and reflects the application of the customer balance
    /// to the total of the invoice.
    pub amount_due: String,
    /// The time at which the invoice was created.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// The time at which the invoice was issued.
    #[serde(with = "time::serde::rfc3339::option")]
    pub issued_at: Option<OffsetDateTime>,
    /// The link to the hosted invoice
    pub hosted_invoice_url: Option<String>,
    /// The status (see [`InvoiceStatusFilter`] for details)
    pub status: String,
    /// Arbitrary metadata that is attached to the invoice. Cannot be nested, must have string
    /// values.
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    /// If payment was attempted on this invoice but failed, this will be the time of the most recent attempt.
    #[serde(with = "time::serde::rfc3339::option")]
    pub payment_failed_at: Option<OffsetDateTime>,
    /// The auto-collection settings for this invoice.
    pub auto_collection: AutoCollection,
    /// The breakdown of prices in this invoice.
    pub line_items: Vec<InvoiceLineItem>,
    // TODO: many missing fields.
}

/// This is basically the same struct as the one above, but doesn't have the invoice_date field
/// because for some reason the fetch_upcoming_invoice API doesn't return it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct UpcomingInvoice {
        /// The Orb-assigned unique identifier for the invoice.
        pub id: String,
        /// The customer to whom this invoice was issued.
        pub customer: InvoiceCustomer,
        /// The subscription associated with this invoice.
        pub subscription: Option<InvoiceSubscription>,
        /// An automatically generated number to help track and reconcile invoices.
        pub invoice_number: String,
        /// The link to download the PDF representation of the invoice.
        pub invoice_pdf: Option<String>,
        /// An ISO 4217 currency string, or "credits"
        pub currency: String,
        /// The total after any minimums, discounts, and taxes have been applied.
        pub total: String,
        /// This is the final amount required to be charged to the
        /// customer and reflects the application of the customer balance
        /// to the total of the invoice.
        pub amount_due: String,
        /// The time at which the invoice was created.
        #[serde(with = "time::serde::rfc3339")]
        pub created_at: OffsetDateTime,
        /// The time at which the invoice was issued.
        #[serde(with = "time::serde::rfc3339::option")]
        pub issued_at: Option<OffsetDateTime>,
        /// The link to the hosted invoice
        pub hosted_invoice_url: Option<String>,
        /// The status (see [`InvoiceStatusFilter`] for details)
        pub status: String,
        /// Arbitrary metadata that is attached to the invoice. Cannot be nested, must have string
        /// values.
        #[serde(default)]
        pub metadata: BTreeMap<String, String>,
        /// If payment was attempted on this invoice but failed, this will be the time of the most recent attempt.
        #[serde(with = "time::serde::rfc3339::option")]
        pub payment_failed_at: Option<OffsetDateTime>,
        /// The auto-collection settings for this invoice.
        pub auto_collection: AutoCollection,
        /// The breakdown of prices in this invoice.
        pub line_items: Vec<InvoiceLineItem>,
        // TODO: many missing fields.
}

/// A line item on an [`Invoice`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct InvoiceLineItem {
    /// The line amount before before any adjustments.
    pub subtotal: String,
    /// The line amount after any adjustments and before overage conversion, credits and partial invoicing.
    pub adjusted_subtotal: String,
    /// Any amount applied from a partial invoice
    pub partially_invoiced_amount: String,
    /// The final amount for a line item after all adjustments and pre paid credits have been applied.
    pub amount: String,
    /// The name of the price associated with this line item.
    pub name: String,
}

/// Auto-collection settings for an [`Invoice`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct AutoCollection {
    #[serde(with = "time::serde::rfc3339::option")]
    pub next_attempt_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub previously_attempted_at: Option<OffsetDateTime>,
    pub enabled: Option<bool>,
    pub num_attempts: Option<u64>,
}

/// Identifies the customer associated with an [`Invoice`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct InvoiceCustomer {
    /// The Orb-assigned unique identifier for the customer.
    pub id: String,
    /// The external identifier for the customer, if any.
    #[serde(rename = "external_customer_id")]
    pub external_id: Option<String>,
}

/// Identifies the subscription associated with an [`Invoice`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct InvoiceSubscription {
    /// The Orb-assigned unique identifier for the subscription.
    pub id: String,
}

/// Identifies the statuses of which [`Invoice`]s should be returned.
#[derive(Debug, Clone, Copy)]
pub struct InvoiceStatusFilter {
    /// Draft -- invoices in their initial state
    pub draft: bool,
    /// Issued -- invoices after their billing period ends
    pub issued: bool,
    /// Paid -- invoices upon confirmation of successful automatic
    /// payment collection
    pub paid: bool,
    /// Void -- invoices that have been manually voided
    pub void: bool,
    /// Synced -- invoices that have been synced to an external
    /// billing provider
    pub synced: bool,
}

impl Default for InvoiceStatusFilter {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl InvoiceStatusFilter {
    /// The default invoice list status filter.
    ///
    /// Exposed as a constant for use in constant evaluation contexts.
    pub const DEFAULT: InvoiceStatusFilter = InvoiceStatusFilter {
        issued: true,
        paid: true,
        synced: true,
        draft: false,
        void: false,
    };
}

/// Parameters for a subscription list operation.
#[derive(Debug, Clone)]
pub struct InvoiceListParams<'a> {
    inner: ListParams,
    customer_filter: Option<CustomerId<'a>>,
    subscription_filter: Option<&'a str>,
    status_filter: InvoiceStatusFilter,
}

impl<'a> Default for InvoiceListParams<'a> {
    fn default() -> InvoiceListParams<'a> {
        InvoiceListParams::DEFAULT
    }
}

impl<'a> InvoiceListParams<'a> {
    /// The default invoice list parameters.
    ///
    /// Exposed as a constant for use in constant evaluation contexts.
    pub const DEFAULT: InvoiceListParams<'static> = InvoiceListParams {
        inner: ListParams::DEFAULT,
        customer_filter: None,
        subscription_filter: None,
        status_filter: InvoiceStatusFilter::DEFAULT,
    };

    /// Sets the page size for the list operation.
    ///
    /// See [`ListParams::page_size`].
    pub const fn page_size(mut self, page_size: u64) -> Self {
        self.inner = self.inner.page_size(page_size);
        self
    }

    /// Filters the listing to the specified customer ID.
    pub const fn customer_id(mut self, filter: CustomerId<'a>) -> Self {
        self.customer_filter = Some(filter);
        self
    }

    /// Filters the listing to the specified subscription ID.
    pub const fn subscription_id(mut self, filter: &'a str) -> Self {
        self.subscription_filter = Some(filter);
        self
    }

    /// Filters the listing to a specified set of statuses.
    pub const fn status_filter(mut self, filter: InvoiceStatusFilter) -> Self {
        self.status_filter = filter;
        self
    }
}

impl Client {
    /// Lists invoices as configured by `params`.
    ///
    /// The underlying API call is paginated. The returned stream will fetch
    /// additional pages as it is consumed.
    pub fn list_invoices(
        &self,
        params: &InvoiceListParams,
    ) -> impl Stream<Item = Result<Invoice, Error>> + '_ {
        let req = self.build_request(Method::GET, INVOICES);
        let req = match params.customer_filter {
            None => req,
            Some(CustomerId::Orb(id)) => req.query(&[("customer_id", id)]),
            Some(CustomerId::External(id)) => req.query(&[("external_customer_id", id)]),
        };
        let req = match params.subscription_filter {
            None => req,
            Some(id) => req.query(&[("subscription_id", id)]),
        };
        let InvoiceStatusFilter {
            draft,
            issued,
            paid,
            void,
            synced,
        } = params.status_filter;
        let mut req = req;
        for (name, value) in [
            ("draft", draft),
            ("issued", issued),
            ("paid", paid),
            ("void", void),
            ("synced", synced),
        ] {
            if value {
                req = req.query(&[("status[]", name)])
            }
        }
        self.stream_paginated_request(&params.inner, req)
    }

    /// Gets an invoice by ID.
    pub async fn get_invoice(&self, id: &str) -> Result<Invoice, Error> {
        let req = self.build_request(Method::GET, INVOICES.chain_one(id));
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// Void an invoice by ID.
    pub async fn void_invoice(&self, id: &str) -> Result<Invoice, Error> {
        let req = self.build_request(Method::POST, INVOICES.chain_one(id).chain_one("void"));
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// Fetch upcoming invoice
    pub async fn fetch_upcoming_invoice(&self, subscription_id: &str) -> Result<UpcomingInvoice, Error> {
        let req = self.build_request(Method::GET, INVOICES.chain_one("upcoming"));
        let req = req.query(&[("subscription_id", subscription_id)]);
        let res = self.send_request(req).await?;
        Ok(res)
    }
}
