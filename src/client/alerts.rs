use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_enum_str::{Deserialize_enum_str, Serialize_enum_str};

use crate::client::Client;
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

}