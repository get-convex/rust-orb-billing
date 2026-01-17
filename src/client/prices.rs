use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// An Orb price
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(tag = "model_type")]
pub enum Price {
    /// Used to represent unit prices
    #[serde(rename = "unit")]
    Unit(UnitPrice),
    /// Used to represent tiered prices
    #[serde(rename = "tiered")]
    Tiered(TieredPrice),
    // TODO: Add support for additional prices
}

/// With unit pricing, each unit costs a fixed amount.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct UnitPrice {
    /// Id of the price
    pub id: String,
    /// Name of the price
    pub name: String,
    /// Config with rates per unit
    pub unit_config: UnitConfig,
    /// Which phase of the plan this price is associated with
    pub plan_phase_order: Option<i64>,
    // TODO: many missing fields.
}

/// In tiered pricing, the cost of a given unit depends on the tier range that it
/// falls into, where each tier range is defined by an upper and lower bound.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct TieredPrice {
    /// Id of the price
    pub id: String,
    /// Name of the price
    pub name: String,
    /// Config with rates per tier
    pub tiered_config: TieredConfig,
    /// Which phase of the plan this price is associated with
    pub plan_phase_order: Option<i64>,
    // TODO: many missing fields.
}

/// An Orb price interval
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct PriceInterval {
    /// The id of the price interval.
    pub id: String,
    /// The price of the interval.
    pub price: Price,
    /// The start date of the price interval.
    /// This is the date that Orb starts billing for this price.
    #[serde(with = "time::serde::rfc3339")]
    pub start_date: OffsetDateTime,
    /// The end date of the price interval.
    /// This is the date that Orb stops billing for this price.
    #[serde(with = "time::serde::rfc3339::option")]
    pub end_date: Option<OffsetDateTime>,
    /// The start date of the current billing period.
    /// Set to null if this price interval is not currently active.
    #[serde(with = "time::serde::rfc3339::option")]
    pub current_billing_period_start_date: Option<OffsetDateTime>,
    /// Fixed fee transitions for this price interval.
    pub fixed_fee_quantity_transitions: Option<Vec<FixedFeeQuantityTransition>>,
}

/// A list of price intervals to add to the subscription.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct AddPriceInterval {
    /// This is the date that the price will start billing on the subscription.
    #[serde(with = "time::serde::rfc3339")]
    pub start_date: OffsetDateTime,
    /// The external price id of the price to add to the subscription.
    pub external_price_id: Option<String>,
}

/// A list of price intervals to edit on the subscription.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct EditPriceInterval {
    /// The id of the price interval to edit.
    pub price_interval_id: String,
    /// A list of fixed fee quantity transitions to use for this price interval.
    /// Note that this list will overwrite all existing fixed fee quantity transitions on the price interval.
    pub fixed_fee_quantity_transitions: Option<Vec<FixedFeeQuantityTransition>>,
}

/// A list of adjustment intervals on a subscription.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct SubscriptionAdjustmentInterval {
    /// The id of the adjustment interval.
    pub id: String,
    /// The start date of the adjustment interval.
    #[serde(with = "time::serde::rfc3339")]
    pub start_date: OffsetDateTime,
    /// The end date of the adjustment interval.
    #[serde(with = "time::serde::rfc3339::option")]
    pub end_date: Option<OffsetDateTime>,
    /// The adjustment details of the adjustment interval.
    pub adjustment: Adjustment,
}

/// The adjustment details of the subscription adjustment interval.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(tag = "adjustment_type")]
pub enum Adjustment {
    /// A maximum adjustment on a subscription.
    #[serde(rename = "maximum")]
    Maximum(MaximumAdjustment),
    /// A percentage discount adjustment on a subscription.
    #[serde(rename = "percentage_discount")]
    PercentageDiscount,
    /// A minimum adjustment on a subscription.
    #[serde(rename = "minimum")]
    Minimum,
}

/// A maximum adjustment on a subscription.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct MaximumAdjustment {
    /// The maximum amount to apply to the price IDs.
    pub maximum_amount: String,
    /// The filters that determine which prices to apply this adjustment to.
    pub filters: Vec<TransformPriceFilter>,
}

/// Filters for specifying which prices an adjustment applies to.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct TransformPriceFilter {
    /// The property of the price to filter on.
    pub field: TransformPriceFilterField,
    /// Should prices that match the filter be included or excluded.
    pub operator: TransformPriceFilterOperator,
    /// The IDs or values that match this filter.
    pub values: Vec<String>,
}

/// The field to filter prices on.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransformPriceFilterField {
    /// Filter by price ID.
    PriceId,
    /// Filter by price type (e.g., usage-based).
    PriceType,
    /// Filter by currency.
    Currency,
}

/// The operator to apply to the price filter.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransformPriceFilterOperator {
    /// Include prices that match the filter.
    Includes,
    /// Exclude prices that match the filter.
    Excludes,
}

/// A list of adjustments to add to the subscription.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct AddAdjustmentInterval {
    /// The start date of the adjustment interval. This is the date that the adjustment will start affecting prices on the subscription.
    #[serde(with = "time::serde::rfc3339")]
    pub start_date: OffsetDateTime,
    /// The end date of the adjustment interval. This is the date that the adjustment will stop affecting prices on the subscription.
    #[serde(with = "time::serde::rfc3339::option")]
    pub end_date: Option<OffsetDateTime>,
    /// The definition of a new adjustment to create and add to the subscription.
    pub adjustment: NewAdjustment,
}

/// The definition of a new adjustment to create and add to the subscription.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(tag = "adjustment_type")]
pub enum NewAdjustment {
    /// A maximum adjustment to create and add to the subscription.
    #[serde(rename = "maximum")]
    NewMaximum(NewMaximumAdjustment),
}

/// A new maximum adjustment to create and add to the subscription.
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct NewMaximumAdjustment {
    /// The set of price IDs to which this adjustment applies.
    pub applies_to_price_ids: Option<Vec<String>>,
    /// If set, the adjustment will apply to every price on the subscription.
    pub applies_to_all: Option<bool>,
    /// If set, only prices of the specified type will have the adjustment applied.
    pub price_type: Option<PriceType>,
    /// If set, only prices in the specified currency will have the adjustment applied.
    pub currency: Option<String>,
    /// The maximum amount to apply to the price IDs.
    pub maximum_amount: String,
}

/// Price type-scoped filters (e.g., all usage-based prices)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PriceType {
    /// Usage-based prices
    Usage,
    // More options not listed here can be added in the future.
}

/// A list of adjustments to edit on the subscription.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct EditAdjustmentInterval {
    /// The id of the adjustment interval to edit.
    pub adjustment_interval_id: String,
    /// The updated end date of this adjustment interval. If not specified, the end date will not be updated.
    #[serde(with = "time::serde::rfc3339::option")]
    pub end_date: Option<OffsetDateTime>,
}

/// A fixed fee quantity transition is used to update the quantity for a price interval.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct FixedFeeQuantityTransition {
    /// The quantity of the fixed fee quantity transition.
    pub quantity: serde_json::Number,
    /// The date that the fixed fee quantity transition should take effect.
    pub effective_date: String,
}

/// Quantity-only price overrides, which do not create child plans unlike normal price overrides.
/// Price override for a unit price
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct QuantityOnlyPriceOverride {
    /// Id of the price
    pub id: String,
    /// The quantity of the price
    pub fixed_price_quantity: serde_json::Number,
}

/// Price overrides are used to update some or all prices in a plan for the specific subscription being created.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(tag = "model_type")]
pub enum PriceOverride {
    /// Used to override unit prices
    #[serde(rename = "unit")]
    Unit(OverrideUnitPrice),
    // TODO: Add support for additional price overrides
}

/// Price override for a unit price
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct OverrideUnitPrice {
    /// Id of the price
    pub id: String,
    /// Will be "unit" for this type of price override
    pub model_type: String,
    /// The starting quantity of the price
    pub fixed_price_quantity: Option<serde_json::Number>,
    /// Configuration for a unit price
    pub unit_config: UnitConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct UnitConfig {
    /// Rate per unit of usage
    pub unit_amount: String,
    /// Multiplier to scale rated quantity by
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scaling_factor: Option<serde_json::Number>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct TieredConfig {
    /// Tiers for rating based on total usage quantities into the specified tier
    pub tiers: Vec<Tier>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Tier {
    /// Inclusive tier starting value
    pub first_unit: serde_json::Number,
    /// Exclusive tier ending value. If null, this is treated as the last tier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_unit: Option<serde_json::Number>,
    /// Rate per unit of usage
    pub unit_amount: String,
}
