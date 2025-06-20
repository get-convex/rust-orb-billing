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

//! [<img src="https://user-images.githubusercontent.com/23521087/168297221-5d346edc-3a55-4055-b355-281b4bd76963.png" width=180 align=right>](https://materialize.com)
//! An async API client for the [Orb] billing platform.
//!
//! # Maintainership
//!
//! This is not an official Orb product. This crate is developed by
//! [Materialize], the streaming data warehouse. Contributions are encouraged:
//!
//! * [View source code](https://github.com/MaterializeInc/rust-orb-billing)
//! * [Report an issue](https://github.com/MaterializeInc/rust-orb-billing/issues/new)
//! * [Submit a pull request](https://github.com/MaterializeInc/rust-orb-billing/compare)
//!
//! [Materialize]: https://materialize.com
//!
//! # See also
//!
//! Additional information is available in the [official Orb API
//! documentation][official-api-docs].
//!
//! [Orb]: https://withorb.com
//! [official-api-docs]: https://docs.withorb.com/docs/orb-docs/api-reference

#[warn(missing_debug_implementations, missing_docs)]
mod client;
mod config;
mod error;
mod serde;
mod util;

pub use client::alerts::{Alert, AlertThreshold, AlertListParams, AlertType, CreateSubscriptionAlertRequest, UpdateAlertRequest};
pub use client::backfill::{BackfillStatus, CreateBackfillParams, BackfillStatusResponse, ListBackfillsResponse};
pub use client::coupons::{Coupon, CouponListParams, Discount, RedeemedCoupon};
pub use client::customers::{
    AddIncrementCreditLedgerEntryRequestParams, AddVoidCreditLedgerEntryRequestParams, Address,
    AddressRequest, CostViewMode, CreateCustomerRequest, Customer, CustomerCostBucket,
    CustomerCostItem, CustomerCostParams, CustomerCostPriceBlock,
    CustomerCostPriceBlockMatrixPrice, CustomerCostPriceBlockMatrixPriceConfig,
    CustomerCostPriceBlockMatrixPriceValue, CustomerCostPriceBlockPrice,
    CustomerCostPriceBlockPriceGroup, CustomerCostPriceBlockUnitPrice,
    CustomerCostPriceBlockUnitPriceConfig, CustomerCreditBlock, CustomerId,
    CustomerPaymentProviderRequest, LedgerEntry, LedgerEntryRequest, PaymentProvider,
    UpdateCustomerRequest, VoidReason,
};
pub use client::events::{
    AmendEventRequest, Event, EventPropertyValue, EventSearchParams, IngestEventDebugResponse,
    IngestEventRequest, IngestEventResponse, IngestionMode,
};
pub use client::invoices::{
    Invoice, InvoiceCustomer, InvoiceListParams, InvoiceStatusFilter, InvoiceSubscription,
};
pub use client::marketplaces::ExternalMarketplace;
pub use client::plans::{Plan, PlanId, PlanListParams};
pub use client::prices::{AddAdjustmentInterval, Adjustment, EditAdjustmentInterval, EditPriceInterval, FixedFeeQuantityTransition, NewAdjustment, NewMaximumAdjustment, OverrideUnitPrice, Price, PriceInterval, PriceOverride, QuantityOnlyPriceOverride, SubscriptionAdjustmentInterval, TieredPrice, UnitPrice};
pub use client::subscriptions::{
    BillingCycleAlignment, ChangeOption, CancelSubscriptionRequest, CreateSubscriptionRequest, PriceIntervalsRequest, SchedulePlanChangeRequest, Subscription, SubscriptionListParams,
    SubscriptionStatus, UpdatePriceQuantityRequest, UpdateSubscriptionRequest, FetchSubscriptionCostsRequest, FetchSubscriptionCostsResponse, SubscriptionCostsEntry
};
pub use client::taxes::{TaxId, TaxIdRequest, TaxIdType};
pub use client::Client;
pub use config::{ClientBuilder, ClientConfig, ListParams};
pub use error::{ApiError, Error};
