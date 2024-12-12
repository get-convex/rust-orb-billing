use futures_core::Stream;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_enum_str::{Deserialize_enum_str, Serialize_enum_str};

use crate::client::Client;
use crate::client::ListParams;
use crate::error::Error;
use crate::util::StrIteratorExt;

const ALERTS_PATH: [&str; 1] = ["alerts"];

/// Creates a subscription alert
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct CreateSubscriptionAlertRequest {
    /// The type of alert to create
    pub r#type: AlertType,
    /// The thresholds that define the values at which the alert will be triggered
    pub thresholds: Option<Vec<AlertThreshold>>,
}

/// Updates an alert
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct UpdateAlertRequest {
    /// The thresholds that define the values at which the alert will be triggered
    pub thresholds: Option<Vec<AlertThreshold>>,
}

/// Parameters for a alert list operation.
#[derive(Debug, Clone)]
pub struct AlertListParams<'a> {
    inner: ListParams,
    subscription_id_filter: Option<&'a str>,
}

impl<'a> Default for AlertListParams<'a> {
    fn default() -> AlertListParams<'a> {
        AlertListParams::DEFAULT
    }
}

impl<'a> AlertListParams<'a> {
    /// The default alert list parameters.
    ///
    /// Exposed as a constant for use in constant evaluation contexts.
    pub const DEFAULT: AlertListParams<'static> = AlertListParams {
        inner: ListParams::DEFAULT,
        subscription_id_filter: None,
    };

    /// Sets the page size for the list operation.
    ///
    /// See [`ListParams::page_size`].
    pub const fn page_size(mut self, page_size: u64) -> Self {
        self.inner = self.inner.page_size(page_size);
        self
    }

    /// Filters the listing to the specified subscription ID.
    pub const fn subscription_id(mut self, filter: &'a str) -> Self {
        self.subscription_id_filter = Some(filter);
        self
    }
}

/// An Orb alert type
#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize_enum_str, Serialize_enum_str)]
#[serde(rename_all = "snake_case")]
pub enum AlertType {
    /// Cost exceeded alert
    CostExceeded,
    // TODO: Support other types of alerts
}

/// An Orb alert threshold
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct AlertThreshold {
    /// The threshold value
    pub value: serde_json::Number,
}

/// An Orb alert
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Alert {
    /// The Orb-assigned unique identifier for the alert.
    pub id: String,
    /// The type of alert
    pub r#type: AlertType,
    /// Whether the alert is enabled or disabled.
    pub enabled: bool,
    /// The thresholds that define the values at which the alert will be triggered
    pub thresholds: Option<Vec<AlertThreshold>>,
}

impl Client {
    /// This endpoint is used to create alerts at the subscription level.
    pub async fn create_subscription_alert(&self, subscription_id: &str, params: &CreateSubscriptionAlertRequest) -> Result<Alert, Error> {
        let req = self.build_request(
            Method::POST, 
            ALERTS_PATH
            .chain_one("subscription_id")
            .chain_one(subscription_id)
            );
        let req = req.json(params);
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// This endpoint retrieves an alert by its ID.
    pub async fn fetch_alert(&self, alert_id: &str) -> Result<Alert, Error> {
        let req = self.build_request(
            Method::GET, 
            ALERTS_PATH
            .chain_one(alert_id)
            );
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// This endpoint returns a list of alerts within Orb.
    pub fn list_alerts(&self, params: &AlertListParams) -> impl Stream<Item = Result<Alert, Error>> + '_ {
        let req = self.build_request(Method::GET, ALERTS_PATH);
        let req = match params.subscription_id_filter {
            None => req,
            Some(subscription_id) => req.query(&[("subscription_id", subscription_id)]),
        };
        self.stream_paginated_request(&params.inner, req)
    }

    /// This endpoint is used to disable an alert.
    pub async fn disable_alert(&self, alert_id: &str) -> Result<Alert, Error> {
        let req = self.build_request(
            Method::POST, 
            ALERTS_PATH
            .chain_one(alert_id)
            .chain_one("disable")
            );
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// This endpoint is used to enable an alert.
    pub async fn enable_alert(&self, alert_id: &str) -> Result<Alert, Error> {
        let req = self.build_request(
            Method::POST, 
            ALERTS_PATH
            .chain_one(alert_id)
            .chain_one("enable")
            );
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// This endpoint updates the thresholds of an alert.
    pub async fn update_alert(&self, alert_id: &str, params: &UpdateAlertRequest) -> Result<Alert, Error> {
        let req = self.build_request(
            Method::PUT, 
            ALERTS_PATH
            .chain_one(alert_id)
            );
        let req = req.json(params);
        let res = self.send_request(req).await?;
        Ok(res)
    }

}