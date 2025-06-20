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
use futures_util::stream::TryStreamExt;
use ordered_float::OrderedFloat;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_enum_str::{Deserialize_enum_str, Serialize_enum_str};
use time::OffsetDateTime;

use crate::{
    AddAdjustmentInterval,
    EditAdjustmentInterval,
    EditPriceInterval,
    QuantityOnlyPriceOverride,
    Price,
    RedeemedCoupon,
    SubscriptionAdjustmentInterval
};
use crate::client::customers::{Customer, CustomerId, CustomerResponse};
use crate::client::marketplaces::ExternalMarketplace;
use crate::client::plans::{Plan, PlanId};
use crate::client::Client;
use crate::config::ListParams;
use crate::error::Error;
use crate::util::StrIteratorExt;

use super::prices::PriceInterval;

const SUBSCRIPTIONS_PATH: [&str; 1] = ["subscriptions"];

/// An Orb subscription.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct CreateSubscriptionRequest<'a> {
    /// An optional user-defined ID for this customer resource, used throughout
    /// the system as an alias for this customer.
    #[serde(flatten)]
    pub customer_id: CustomerId<'a>,
    /// The plan that the customer should be subscribed to.
    ///
    /// The plan determines the pricing and the cadence of the subscription.
    #[serde(flatten)]
    pub plan_id: PlanId<'a>,
    /// The date at which Orb should start billing for the subscription,
    /// localized ot the customer's timezone.
    ///
    /// If `None`, defaults to the current date in the customer's timezone.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub start_date: Option<OffsetDateTime>,
    /// The name of the external marketplace that the subscription is attached
    /// to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_marketplace: Option<SubscriptionExternalMarketplaceRequest<'a>>,
    /// Whether to align billing periods with the subscription's start date.
    ///
    /// If `None`, the value is determined by the plan configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align_billing_with_subscription_start_date: Option<bool>,
    /// The subscription's override minimum amount for the plan.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_amount: Option<&'a str>,
    /// The subscription's override minimum amount for the plan.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_terms: Option<i64>,
    /// Determines whether issued invoices for this subscription will
    /// automatically be charged with the saved payment method on the due date.
    ///
    /// If `None`, the value is determined by the plan configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_collection: Option<bool>,
    /// Determines the default memo on this subscription's invoices.
    ///
    /// If `None`, the value is determined by the plan configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_invoice_memo: Option<&'a str>,
    /// An idempotency key can ensure that if the same request comes in
    /// multiple times in a 48-hour period, only one makes changes.
    // NOTE: this is passed in a request header, not the body
    #[serde(skip_serializing)]
    pub idempotency_key: Option<&'a str>,
    /// Optionally provide a list of overrides for prices on the plan
    /// TODO: this should really be a union of QuantityOnlyPriceOverride and PriceOverride
    /// but just using QuantityOnlyPriceOverride since that's the only one we need for now
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_overrides: Option<Vec<QuantityOnlyPriceOverride>>,
    /// Coupon to apply to this subscription
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coupon_redemption_code: Option<&'a str>,
    /// When this subscription's accrued usage reaches this threshold, an invoice
    /// will be issued for the subscription. If not specified, invoices will only
    /// be issued at the end of the billing period.
    pub invoicing_threshold: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct SubscriptionExternalMarketplaceRequest<'a> {
    /// The kind of the external marketplace.
    #[serde(rename = "external_marketplace")]
    pub kind: ExternalMarketplace,
    /// The ID of the subscription in the external marketplace.
    #[serde(rename = "external_marketplace_reporting_id")]
    pub reporting_id: &'a str,
}

/// Updates the quantity for a fixed fee
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct UpdatePriceQuantityRequest<'a> {
    /// Price for which the quantity should be updated. Must be a fixed fee.
    pub price_id: &'a str,
    /// New quantity for the fixed fee.
    pub quantity: serde_json::Number,
}

/// Options for billing cycle alignment during a plan change.
#[derive(Clone, Default, Debug, PartialEq, Eq, Hash, Deserialize_enum_str, Serialize_enum_str)]
#[serde(rename_all = "snake_case")]
pub enum BillingCycleAlignment {
    /// Keeps subscription's existing billing cycle alignment.
    #[default]
    Unchanged,
    /// Aligns billing periods with the plan change's effective date.
    PlanChangeDate,
    /// Aligns billing periods with the start of the month.
    StartOfMonth,
}

/// Changes the plan on an existing subscription
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct SchedulePlanChangeRequest<'a> {
    /// The plan to switch to.
    #[serde(flatten)]
    pub plan_id: PlanId<'a>,
    /// One of ["requested_date", "end_of_subscription_term", "immediate"]
    pub change_option: ChangeOption,
    /// The date that the plan change should take effect. This parameter
    /// can only be passed if the change_option is requested_date.
    pub change_date: Option<&'a str>,
    /// Optionally provide a list of overrides for prices on the plan
    /// TODO: this should really be a union of QuantityOnlyPriceOverride and PriceOverride
    /// but just using QuantityOnlyPriceOverride since that's the only one we need for now
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_overrides: Option<Vec<QuantityOnlyPriceOverride>>,
    /// Coupon to apply to this subscription
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coupon_redemption_code: Option<&'a str>,
    /// When this subscription's accrued usage reaches this threshold, an invoice
    /// will be issued for the subscription. If not specified, invoices will only
    /// be issued at the end of the billing period.
    pub invoicing_threshold: Option<&'a str>,
    /// Reset billing periods to be aligned with the plan change's effective date
    /// or start of the month. 
    pub billing_cycle_alignment: Option<BillingCycleAlignment>,
}

/// Options for when a plan transition should take place.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Deserialize_enum_str, Serialize_enum_str)]
#[serde(rename_all = "snake_case")]
pub enum ChangeOption {
    /// Changes the plan on a requested date
    RequestedDate,
    /// Changes the plan at the end of the existing plan's term.
    EndOfSubscriptionTerm,
    /// Changes the plan immediately.
    #[default]
    Immediate,
}

/// A request to update the price intervals on a subscription.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct PriceIntervalsRequest<'a> {
    /// Edit existing price intervals.
    pub edit: Vec<EditPriceInterval>,
    /// Add adjustment intervals.
    pub add_adjustments: Vec<AddAdjustmentInterval>,
    /// Edit adjustment intervals.
    pub edit_adjustments: Vec<EditAdjustmentInterval>,
    /// An idempotency key can ensure that if the same request comes in
    /// multiple times in a 48-hour period, only one makes changes.
    // NOTE: this is passed in a request header, not the body
    #[serde(skip_serializing)]
    pub idempotency_key: Option<&'a str>,
}

/// A request to cancel a subscription.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct CancelSubscriptionRequest {
    /// Possible values: [end_of_subscription_term, immediate, requested_date]
    pub cancel_option: ChangeOption,
    /// The date that the cancellation should take effect. This parameter can only be passed if the cancel_option is requested_date.
    #[serde(with = "time::serde::rfc3339::option")]
    pub cancellation_date: Option<OffsetDateTime>
}

/// A request to update a subscription.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct UpdateSubscriptionRequest<'a> {
    /// When this subscription's accrued usage reaches this threshold, an invoice
    /// will be issued for the subscription. If not specified, invoices will only
    /// be issued at the end of the billing period.
    pub invoicing_threshold: Option<&'a str>,
    // TODO: add more fields
}

/// A request to fetch the costs of a subscription.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct FetchSubscriptionCostsRequest {
    /// Costs returned are inclusive of timeframe_start.
    #[serde(with = "time::serde::rfc3339::option")]
    pub timeframe_start: Option<OffsetDateTime>,
    /// Costs returned are exclusive of timeframe_end.
    #[serde(with = "time::serde::rfc3339::option")]
    pub timeframe_end: Option<OffsetDateTime>,
}

/// The response from fetching the costs of a subscription.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct FetchSubscriptionCostsResponse {
    /// The data returned by the fetch subscription costs endpoint.
    pub data: Vec<SubscriptionCostsEntry>,
}

/// One of the entries in the data that is returned by fetch subscription costs endpoint
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SubscriptionCostsEntry {
    /// Costs returned are inclusive of timeframe_start.
    #[serde(with = "time::serde::rfc3339")]
    pub timeframe_start: OffsetDateTime,
    /// Costs returned are exclusive of timeframe_end.
    #[serde(with = "time::serde::rfc3339")]
    pub timeframe_end: OffsetDateTime,
    /// Total costs for the timeframe, excluding any minimums and discounts.
    pub subtotal: String,
    /// Total costs for the timeframe, including any minimums and discounts.
    pub total: String,
    /// Per price costs
    pub per_price_costs: Vec<PerPriceCostsEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct PerPriceCostsEntry {
    /// Price's contributions for the timeframe, excluding any minimums and discounts.
    pub subtotal: String,
    /// Price's contributions for the timeframe, including any minimums and discounts.
    pub total: String,
    pub price: Price,
}

/// An Orb subscription.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Subscription<C = Customer> {
    /// The Orb-assigned unique identifier for the subscription.
    pub id: String,
    /// The customer associated with this subscription.
    pub customer: C,
    /// The plan associated with this subscription.
    pub plan: Plan,
    /// The date at which Orb starts billing for this subscription.
    #[serde(with = "time::serde::rfc3339")]
    pub start_date: OffsetDateTime,
    /// The date at which Orb stops billing for this subscription.
    #[serde(with = "time::serde::rfc3339::option")]
    pub end_date: Option<OffsetDateTime>,
    /// The status of the subscription.
    pub status: SubscriptionStatus,
    /// The start of the current billing period if the subscription is currently
    /// active.
    #[serde(with = "time::serde::rfc3339::option")]
    pub current_billing_period_start_date: Option<OffsetDateTime>,
    /// The end of the current billing period if the subscription is currently
    /// active.
    #[serde(with = "time::serde::rfc3339::option")]
    pub current_billing_period_end_date: Option<OffsetDateTime>,
    /// The current plan phase that is active, if the subscription's plan has
    /// phases.
    pub active_plan_phase_order: Option<i64>,
    /// List of all fixed fee quantities associated with this subscription.
    pub fixed_fee_quantity_schedule: Vec<SubscriptionFixedFee>,
    /// Determines the difference between the invoice issue date and the
    /// date that they are due.
    ///
    /// A value of zero indicates that the invoice is due on issue, whereas a
    /// value of 30 represents that the customer has a month to pay the invoice.
    pub net_terms: i64,
    /// Determines whether issued invoices for this subscription will
    /// automatically be charged with the saved payment method on the due date.
    ///
    /// If `None`, the value is determined by the plan configuration.
    pub auto_collection: Option<bool>,
    /// Determines the default memo on this subscription's invoices.
    ///
    /// If `None`, the value is determined by the plan configuration.
    pub default_invoice_memo: Option<String>,
    /// The time at which the subscription was created.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// Coupon that was redeemed for the subscription.
    pub redeemed_coupon: Option<RedeemedCoupon>,
    /// The price intervals for this subscription.
    pub price_intervals: Vec<PriceInterval>,
    /// The adjustment intervals for this subscription.
    pub adjustment_intervals: Vec<SubscriptionAdjustmentInterval>,
    /// When this subscription's accrued usage reaches this threshold, an invoice
    /// will be issued for the subscription. If not specified, invoices will only
    /// be issued at the end of the billing period.
    pub invoicing_threshold: Option<String>,
}

/// The status of an Orb subscription.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize_enum_str, Serialize_enum_str)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    /// An active subscription.
    Active,
    /// A subscription that has ended.
    Ended,
    /// A subscription that has not yet started.
    Upcoming,
    /// An unknown subscription status.
    #[serde(other)]
    Other(String),
}

/// An entry in [`Subscription::fixed_fee_quantity_schedule`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct SubscriptionFixedFee {
    /// The date at which the fixed fee starts.
    #[serde(with = "time::serde::rfc3339")]
    pub start_date: OffsetDateTime,
    /// The date at which the fixed fee ends.
    #[serde(with = "time::serde::rfc3339::option")]
    pub end_date: Option<OffsetDateTime>,
    /// The price ID for the fixed fee.
    pub price_id: String,
    /// The quantity of the fixed fee.
    pub quantity: OrderedFloat<f64>,
}

/// Parameters for a subscription list operation.
#[derive(Debug, Clone)]
pub struct SubscriptionListParams<'a> {
    inner: ListParams,
    customer_id_filter: Option<CustomerId<'a>>,
    status_filter: Option<&'a str>,
}

impl<'a> Default for SubscriptionListParams<'a> {
    fn default() -> SubscriptionListParams<'a> {
        SubscriptionListParams::DEFAULT
    }
}

impl<'a> SubscriptionListParams<'a> {
    /// The default subscription list parameters.
    ///
    /// Exposed as a constant for use in constant evaluation contexts.
    pub const DEFAULT: SubscriptionListParams<'static> = SubscriptionListParams {
        inner: ListParams::DEFAULT,
        customer_id_filter: None,
        status_filter: None,
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
        self.customer_id_filter = Some(filter);
        self
    }

    /// Filters the listing by status
    pub const fn status(mut self, filter: &'a str) -> Self {
        self.status_filter = Some(filter);
        self
    }
}

impl Client {
    /// Lists subscriptions as configured by `params`.
    ///
    /// The underlying API call is paginated. The returned stream will fetch
    /// additional pages as it is consumed.
    pub fn list_subscriptions(
        &self,
        params: &SubscriptionListParams,
    ) -> impl Stream<Item = Result<Subscription, Error>> + '_ {
        let req = self.build_request(Method::GET, SUBSCRIPTIONS_PATH);
        let req = match params.customer_id_filter {
            None => req,
            Some(CustomerId::Orb(id)) => req.query(&[("customer_id", id)]),
            Some(CustomerId::External(id)) => req.query(&[("external_customer_id", id)]),
        };
        let req = match params.status_filter {
            None => req,
            Some(status) => req.query(&[("status", status)]),
        };
        self.stream_paginated_request(&params.inner, req)
            .try_filter_map(|subscription: Subscription<CustomerResponse>| async move {
                match subscription.customer {
                    CustomerResponse::Normal(customer) => Ok(Some(Subscription {
                        id: subscription.id,
                        customer,
                        plan: subscription.plan,
                        start_date: subscription.start_date,
                        end_date: subscription.end_date,
                        status: subscription.status,
                        current_billing_period_start_date: subscription
                            .current_billing_period_start_date,
                        current_billing_period_end_date: subscription
                            .current_billing_period_end_date,
                        active_plan_phase_order: subscription.active_plan_phase_order,
                        fixed_fee_quantity_schedule: subscription.fixed_fee_quantity_schedule,
                        net_terms: subscription.net_terms,
                        auto_collection: subscription.auto_collection,
                        default_invoice_memo: subscription.default_invoice_memo,
                        created_at: subscription.created_at,
                        redeemed_coupon: subscription.redeemed_coupon,
                        price_intervals: subscription.price_intervals,
                        adjustment_intervals: subscription.adjustment_intervals,
                        invoicing_threshold: subscription.invoicing_threshold,
                    })),
                    CustomerResponse::Deleted {
                        id: _,
                        deleted: true,
                    } => Ok(None),
                    CustomerResponse::Deleted { id, deleted: false } => {
                        Err(Error::UnexpectedResponse {
                            detail: format!(
                                "customer {id} used deleted response shape \
                                but deleted field was `false`"
                            ),
                        })
                    }
                }
            })
    }

    /// Creates a new subscription.
    pub async fn create_subscription(
        &self,
        subscription: &CreateSubscriptionRequest<'_>,
    ) -> Result<Subscription, Error> {
        let mut req = self.build_request(Method::POST, SUBSCRIPTIONS_PATH);
        if let Some(key) = subscription.idempotency_key {
            req = req.header("Idempotency-Key", key);
        }

        let req = req.json(subscription);
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// Gets a subscription by ID.
    pub async fn get_subscription(&self, id: &str) -> Result<Subscription, Error> {
        let req = self.build_request(Method::GET, SUBSCRIPTIONS_PATH.chain_one(id));
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// Updates the quantity for a fixed fee
    pub async fn update_price_quantity(&self, id: &str, params: &UpdatePriceQuantityRequest<'_>) -> Result<Subscription, Error> {
        let req = self.build_request(
            Method::POST,
            SUBSCRIPTIONS_PATH
            .chain_one(id)
            .chain_one("update_fixed_fee_quantity")
        );
        let req = req.json(params);
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// Changes the plan on an existing subscription
    pub async fn schedule_plan_change(&self, id: &str, params: &SchedulePlanChangeRequest<'_>) -> Result<Subscription, Error> {
        let req = self.build_request(
            Method::POST,
            SUBSCRIPTIONS_PATH
            .chain_one(id)
            .chain_one("schedule_plan_change")
        );
        let req = req.json(params);
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// Add and edit price intervals on a subscription.
    pub async fn price_intervals(&self, id: &str, params: &PriceIntervalsRequest<'_>) -> Result<Subscription, Error> {
        let mut req = self.build_request(
            Method::POST,
            SUBSCRIPTIONS_PATH
            .chain_one(id)
            .chain_one("price_intervals")
        );
        if let Some(key) = params.idempotency_key {
            req = req.header("Idempotency-Key", key);
        }

        let req = req.json(params);
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// Cancel a subscription
    pub async fn cancel_subscription(&self, id: &str, params: &CancelSubscriptionRequest) -> Result<Subscription, Error> {
        let req = self.build_request(
            Method::POST,
            SUBSCRIPTIONS_PATH
            .chain_one(id)
            .chain_one("cancel")
        );
        let req = req.json(params);
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// Unschedules any pending cancellations for a subscription
   pub async fn unschedule_cancellation(&self, id: &str) -> Result<Subscription, Error> {
        let req = self.build_request(
            Method::POST,
            SUBSCRIPTIONS_PATH
            .chain_one(id)
            .chain_one("unschedule_cancellation")
        );
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// Updates a subscription
    pub async fn update_subscription(&self, id: &str, params: &UpdateSubscriptionRequest<'_>) -> Result<Subscription, Error> {
        let req = self.build_request(
            Method::PUT,
            SUBSCRIPTIONS_PATH
            .chain_one(id)
        );
        let req = req.json(params);
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// Fetches the costs of a subscription
    pub async fn fetch_subscription_costs(&self, id: &str, params: &FetchSubscriptionCostsRequest) -> Result<FetchSubscriptionCostsResponse, Error> {
        let req = self.build_request(
            Method::GET,
            SUBSCRIPTIONS_PATH
            .chain_one(id)
            .chain_one("costs")
        );
        let req = req.json(params);
        let res = self.send_request(req).await?;
        Ok(res)
    }
}
