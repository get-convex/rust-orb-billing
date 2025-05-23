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

//! Integration tests.
//!
//! To run these tests, you must create an Orb account and provide an API key in
//! the `ORB_API_KEY` environment variable.
//!
//! These tests must be run serially, as via
//!
//!     $ cargo test -- --test-threads=1
//!
//! because each test competes for access to the same Orb account.

use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fmt;
use std::ops::{Add, Sub};

use ::time::{OffsetDateTime, Time};
use codes_iso_3166::part_1::CountryCode;
use futures::future;
use futures::stream::TryStreamExt;
use once_cell::sync::Lazy;
use rand::Rng;
use reqwest::StatusCode;
use test_log::test;
use tokio::time::{self, Duration};
use tracing::info;

use orb_billing::{AddIncrementCreditLedgerEntryRequestParams, AddVoidCreditLedgerEntryRequestParams, Address, AddressRequest, AmendEventRequest, Client, ClientConfig, CostViewMode, CreateCustomerRequest, CreateSubscriptionRequest, Customer, CustomerCostParams, CustomerCostPriceBlockPrice, CustomerId, CustomerPaymentProviderRequest, Error, Event, EventPropertyValue, EventSearchParams, IngestEventRequest, IngestionMode, InvoiceListParams, LedgerEntry, LedgerEntryRequest, ListParams, PaymentProvider, SubscriptionListParams, TaxId, TaxIdRequest, UpdateCustomerRequest, VoidReason, PlanListParams, CreateBackfillParams};

/// The API key to authenticate with.
static API_KEY: Lazy<String> = Lazy::new(|| env::var("ORB_API_KEY").expect("missing ORB_API_KEY"));

/// When performing parallel operations against the Orb API, the maximum
/// number of concurrent operations to run.
const CONCURRENCY_LIMIT: usize = 16;

/// A prefix to use in objects to make it possible to determine whether a given
/// object was created by this test script or not.
///
/// Required because we do not have exclusive access to the Orb account. Other
/// tests may be running against this account, so we do not want to blindly
/// delete all existing objects at the start of the test.
const TEST_PREFIX: &str = "$TEST-RUST-API$";

/// A `ListParams` that uses the maximum possible page size.
const MAX_PAGE_LIST_PARAMS: ListParams = ListParams::DEFAULT.page_size(500);

/// The number of retries to attempt for Orb endpoints with known latency
const MAX_LIST_RETRIES: usize = 8;

fn new_client() -> Client {
    Client::new(ClientConfig {
        api_key: API_KEY.clone(),
    })
}

async fn delete_all_test_customers(client: &Client) {
    client
        .list_customers(&MAX_PAGE_LIST_PARAMS)
        .try_filter(|customer| future::ready(customer.name.starts_with(TEST_PREFIX)))
        .try_for_each_concurrent(Some(CONCURRENCY_LIMIT), |customer| async move {
            info!(%customer.id, "deleting customer");
            client.delete_customer(&customer.id).await
        })
        .await
        .unwrap()
}

async fn create_test_customer(client: &Client, i: usize) -> Customer {
    client
        .create_customer(&CreateCustomerRequest {
            name: &format!("{TEST_PREFIX}-{i}"),
            email: &format!("orb-testing-{i}@materialize.com"),
            external_id: None,
            payment_provider: Some(CustomerPaymentProviderRequest {
                kind: PaymentProvider::Stripe,
                id: &format!("cus_fake_{i}"),
            }),
            ..Default::default()
        })
        .await
        .unwrap()
}

fn assert_error_with_status_code<T>(res: Result<T, Error>, status_code: StatusCode)
where
    T: fmt::Debug,
{
    match res.unwrap_err() {
        Error::Api(e) => assert_eq!(e.status_code, status_code),
        e => panic!("expected API error with code {status_code} but got: {e:?}"),
    }
}

#[test(tokio::test)]
async fn test_customers() {
    // Set up.
    let client = new_client();
    let nonce = rand::thread_rng().gen::<u32>();
    delete_all_test_customers(&client).await;

    // Test creating a customer.
    let name = format!("{TEST_PREFIX}-{nonce}");
    let email = "orb-testing@materialize.com";
    let external_id = format!("{TEST_PREFIX}-{nonce}");
    let customer = client
        .create_customer(&CreateCustomerRequest {
            name: &name,
            email,
            external_id: Some(&*external_id),
            timezone: Some("America/New_York"),
            idempotency_key: Some(&external_id),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(customer.name, name);
    assert_eq!(customer.email, email);
    assert_eq!(customer.external_id.as_ref(), Some(&external_id));
    assert_eq!(customer.timezone, "America/New_York");
    assert_eq!(customer.balance, "0.00");
    assert_eq!(customer.billing_address, None);
    assert_eq!(customer.shipping_address, None);
    assert_eq!(customer.tax_id, None);
    let empty_emails: Vec<String> = vec![];
    assert_eq!(customer.additional_emails, empty_emails);

    // Test fetching the customer by ID.
    let customer = client.get_customer(&customer.id).await.unwrap();
    assert_eq!(customer.name, name);
    assert_eq!(customer.email, email);

    // Test fetching the customer by external ID.
    let customer = client
        .get_customer_by_external_id(&external_id)
        .await
        .unwrap();
    assert_eq!(customer.name, name);
    assert_eq!(customer.email, email);

    // Test crediting customers and reading their balances back
    let ledger_res = client
        .create_ledger_entry(
            &customer.id,
            &LedgerEntryRequest::Increment(AddIncrementCreditLedgerEntryRequestParams {
                amount: serde_json::Number::from(42),
                description: Some("Test credit"),
                expiry_date: None,
                effective_date: None,
                per_unit_cost_basis: None,
                invoice_settings: None,
            }),
        )
        .await
        .unwrap();
    let inc_res = match ledger_res {
        LedgerEntry::Increment(inc_res) => inc_res,
        entry => panic!("Expected an Increment, received: {:?}", entry),
    };
    assert_eq!(inc_res.ledger.customer.id, customer.id);
    let balance: Vec<_> = client
        .get_customer_credit_balance(&customer.id, &ListParams::default().page_size(1))
        .try_collect()
        .await
        .unwrap();
    assert_eq!(balance.get(0).unwrap().balance, inc_res.ledger.amount);
    let ledger_res = client
        .create_ledger_entry(
            &customer.id,
            &LedgerEntryRequest::Void(AddVoidCreditLedgerEntryRequestParams {
                amount: inc_res.ledger.amount,
                block_id: &inc_res.ledger.credit_block.id,
                void_reason: Some(VoidReason::Refund),
                description: None,
            }),
        )
        .await
        .unwrap();
    let void_res = match ledger_res {
        LedgerEntry::VoidInitiated(void_res) => void_res,
        entry => panic!("Expected a VoidInitiated, received a {:?}", entry),
    };
    assert_eq!(void_res.ledger.customer.id, customer.id);
    let balance: Vec<_> = client
        .get_customer_credit_balance_by_external_id(
            &customer.external_id.unwrap(),
            &ListParams::default().page_size(1),
        )
        .try_collect()
        .await
        .unwrap();
    assert!(balance.is_empty());
    // Test a second creation request with the same idempotency key does
    // *not* create a new instance
    let res = client
        .create_customer(&CreateCustomerRequest {
            name: &name,
            email,
            external_id: Some(&format!("{external_id}-0")),
            timezone: Some("America/Chicago"),
            idempotency_key: Some(&external_id),
            ..Default::default()
        })
        .await;
    match res.expect_err("Request with idempotency key did not error") {
        Error::Api(e) if e.status_code == 409 => println!("Received expected conflict status"),
        x => panic!("Got unexpected error: {x:?}"),
    }

    // Test updating the customer by ID.
    let customer = client
        .update_customer(
            &customer.id,
            &UpdateCustomerRequest {
                email: Some("orb-testing+update-1@materialize.com"),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(customer.email, "orb-testing+update-1@materialize.com");
    let customer = client.get_customer(&customer.id).await.unwrap();
    assert_eq!(customer.email, "orb-testing+update-1@materialize.com");
    let empty_emails: Vec<String> = vec![];
    assert_eq!(customer.additional_emails, empty_emails);

    // Test updating additional_emails by ID
    let customer = client
        .update_customer(
            &customer.id,
            &UpdateCustomerRequest {
                additional_emails: Some(vec![
                    "orb-testing+update-2@materialize.com",
                    "orb-testing+update-3@materialize.com",
                ]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(customer
        .additional_emails
        .contains(&"orb-testing+update-2@materialize.com".to_string()));
    assert!(customer
        .additional_emails
        .contains(&"orb-testing+update-3@materialize.com".to_string()));
    let customer = client.get_customer(&customer.id).await.unwrap();
    assert!(customer
        .additional_emails
        .contains(&"orb-testing+update-2@materialize.com".to_string()));
    assert!(customer
        .additional_emails
        .contains(&"orb-testing+update-3@materialize.com".to_string()));
    assert_eq!(customer.email, "orb-testing+update-1@materialize.com");

    // Test updating the customer by external ID.
    let customer = client
        .update_customer_by_external_id(
            &external_id,
            &UpdateCustomerRequest {
                email: Some("orb-testing+update-2@materialize.com"),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(customer.email, "orb-testing+update-2@materialize.com");
    let customer = client.get_customer(&customer.id).await.unwrap();
    assert_eq!(customer.email, "orb-testing+update-2@materialize.com");

    // Test creating a second customer, and exercise addresses and tax IDs.
    let email2 = "orb-testing+2@materialize.com";
    let customer2 = client
        .create_customer(&CreateCustomerRequest {
            name: &format!("{TEST_PREFIX}-{nonce}-2"),
            email: email2,
            shipping_address: Some(AddressRequest {
                city: Some("New York"),
                country: Some(&CountryCode::US.to_string()),
                line1: Some("440 Lafayette St"),
                line2: Some("Floor 6"),
                postal_code: Some("10003"),
                state: Some("NY"),
            }),
            billing_address: Some(AddressRequest {
                city: Some("Boston"),
                country: Some(&CountryCode::US.to_string()),
                ..Default::default()
            }),
            tax_id: Some(TaxIdRequest {
                type_: orb_billing::TaxIdType::UsEin,
                value: "12-3456789",
                country: CountryCode::US.to_string(),
            }),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(
        customer2.shipping_address,
        Some(Address {
            city: Some("New York".into()),
            country: Some(CountryCode::US.to_string()),
            line1: Some("440 Lafayette St".into()),
            line2: Some("Floor 6".into()),
            postal_code: Some("10003".into()),
            state: Some("NY".into()),
        })
    );
    assert_eq!(
        customer2.billing_address,
        Some(Address {
            city: Some("Boston".into()),
            country: Some(CountryCode::US.to_string()),
            line1: None,
            line2: None,
            postal_code: None,
            state: None,
        })
    );
    assert_eq!(
        customer2.tax_id,
        Some(TaxId {
            type_: orb_billing::TaxIdType::UsEin,
            value: "12-3456789".into(),
            country: CountryCode::US.to_string(),
        })
    );

    // List customers, and ensure we see both customers that we created.
    // Do so with a page size of one to exercise the pagination logic.
    let customer_ids: HashSet<_> = client
        .list_customers(&ListParams::default().page_size(1))
        .map_ok(|customer| customer.id)
        .try_collect()
        .await
        .unwrap();
    assert!(customer_ids.contains(&customer.id));
    assert!(customer_ids.contains(&customer2.id));
}

#[test(tokio::test)]
async fn test_events() {
    // Set up.
    let client = new_client();
    let nonce = rand::thread_rng().gen::<u32>();
    delete_all_test_customers(&client).await;

    let customer_idx = 0;
    let customer = create_test_customer(&client, customer_idx).await;

    // Create data for three events.
    let mut ids = vec![];
    let mut timestamps = vec![];
    for i in 0..3 {
        let id = format!("event-{nonce}-{i}");
        let time = Time::from_hms(i, 0, 0).unwrap();
        let timestamp = OffsetDateTime::now_utc().replace_time(time);
        // Make all events happen tomorrow to avoid falling outside of the account's grace
        // period.
        let timestamp =
            timestamp.replace_date(timestamp.date().next_day().expect("Y10K problem detected"));
        ids.push(id);
        timestamps.push(timestamp);
    }
    // `timeframe_end` is an exclusive endpoint, so add a second to ensure all events are captured.
    let timeframe_end = timestamps.last().unwrap().add(Duration::from_secs(1));

    // Test that ingesting two new events results in Orb accepting both of them.
    let events = client
        .ingest_events(
            IngestionMode::Debug,
            None,
            &[
                IngestEventRequest {
                    customer_id: CustomerId::Orb(&customer.id),
                    idempotency_key: &ids[0],
                    event_name: "test",
                    properties: &BTreeMap::new(),
                    timestamp: timestamps[0],
                },
                IngestEventRequest {
                    customer_id: CustomerId::Orb(&customer.id),
                    idempotency_key: &ids[1],
                    event_name: "test",
                    properties: &BTreeMap::new(),
                    timestamp: timestamps[1],
                },
            ],
        )
        .await
        .unwrap();
    assert!(events.debug.as_ref().unwrap().duplicate.is_empty());
    // Ensure that the objects are sorted so that lists compare equal
    let mut ingested = events.debug.as_ref().unwrap().ingested.clone();
    ingested.sort();
    assert_eq!(ingested, vec![ids[0].clone(), ids[1].clone()]);

    // Test that ingesting one new event and one old event results in Orb
    // accepting only the new event.
    let events = client
        .ingest_events(
            IngestionMode::Debug,
            None,
            &[
                IngestEventRequest {
                    customer_id: CustomerId::Orb(&customer.id),
                    idempotency_key: &ids[1],
                    event_name: "test",
                    properties: &BTreeMap::new(),
                    timestamp: timestamps[1],
                },
                IngestEventRequest {
                    customer_id: CustomerId::Orb(&customer.id),
                    idempotency_key: &ids[2],
                    event_name: "test",
                    properties: &BTreeMap::new(),
                    timestamp: timestamps[2],
                },
            ],
        )
        .await
        .unwrap();
    assert_eq!(
        events.debug.as_ref().unwrap().duplicate,
        vec![ids[1].clone()]
    );
    assert_eq!(
        events.debug.as_ref().unwrap().ingested,
        vec![ids[2].clone()]
    );

    let events = client
        .ingest_events(
            IngestionMode::Production,
            None,
            &[IngestEventRequest {
                customer_id: CustomerId::Orb(&customer.id),
                idempotency_key: &ids[1],
                event_name: "test",
                properties: &BTreeMap::new(),
                timestamp: timestamps[1],
            }],
        )
        .await
        .unwrap();
    assert!(events.debug.is_none());

    // Extremely sketchy sleep seems to be required for search results to
    // reflect the ingestion
    time::sleep(Duration::from_secs(20)).await;

    // Test that all ingested events are reported in search results.
    let events: Vec<_> = client
        .search_events(
            &EventSearchParams::default()
                .event_ids(&[&ids[0], &ids[1], &ids[2]])
                .timeframe_end(timeframe_end),
        )
        .try_collect()
        .await
        .unwrap();
    assert_eq!(
        events,
        vec![
            Event {
                id: ids[0].clone(),
                customer_id: customer.id.clone(),
                external_customer_id: None,
                event_name: "test".into(),
                properties: BTreeMap::new(),
                timestamp: timestamps[0],
            },
            Event {
                id: ids[1].clone(),
                customer_id: customer.id.clone(),
                external_customer_id: None,
                event_name: "test".into(),
                properties: BTreeMap::new(),
                timestamp: timestamps[1],
            },
            Event {
                id: ids[2].clone(),
                customer_id: customer.id.clone(),
                external_customer_id: None,
                event_name: "test".into(),
                properties: BTreeMap::new(),
                timestamp: timestamps[2],
            },
        ]
    );

    // Test amending an event.
    let mut properties = BTreeMap::new();
    properties.insert("test".into(), EventPropertyValue::Bool(false));
    client
        .amend_event(
            &ids[0],
            &AmendEventRequest {
                customer_id: CustomerId::Orb(&customer.id),
                event_name: "new test",
                properties: &properties,
                timestamp: timestamps[0],
            },
        )
        .await
        .unwrap();

    // Orb takes its time registering the amendment in the search output. Let's try a few times
    // before giving up.
    for iteration in 1..=MAX_LIST_RETRIES {
        // Extremely sketchy sleep.
        time::sleep(Duration::from_secs(60)).await;

        let events: Vec<_> = client
            .search_events(
                &EventSearchParams::default()
                    .event_ids(&[&ids[0]])
                    .timeframe_end(timeframe_end),
            )
            .try_collect()
            .await
            .unwrap();
        if events.get(0).map(|e| e.event_name.clone()) != Some("new test".into()) {
            info!("  events list not updated after {iteration} attempts.");
            if iteration < MAX_LIST_RETRIES {
                continue;
            }
        }
        assert!(events.iter().any(|e| e.properties == properties));
        // Exit the loop
        break;
    }
}

#[test(tokio::test)]
async fn test_plans() {
    let client = new_client();

    let plans: Vec<_> = client
        .list_plans(&PlanListParams::default().status("active"))
        .try_collect()
        .await
        .unwrap();
    println!("plans = {:#?}", plans);

    // TODO: validate list results.
    // TODO: test get_plan.
    // TODO: test get_plan_by_external_id.
    // TODO: test get_plan w/nested plans?
    // Testing the above will be hard as there is no API to create plans.
}

#[test(tokio::test)]
async fn test_subscriptions() {
    let client = new_client();
    delete_all_test_customers(&client).await;

    let nonce = rand::thread_rng().gen::<u32>();
    let mut customers = vec![];
    let mut subscriptions = vec![];

    // Test creating and retrieving subscriptions.
    for i in 0..3 {
        let customer = create_test_customer(&client, i).await;
        let idempotency_key = format!("test-subscription-{nonce}-{i}");

        let subscription = client
            .create_subscription(&CreateSubscriptionRequest {
                customer_id: CustomerId::Orb(&customer.id),
                plan_id: orb_billing::PlanId::External("test"),
                net_terms: Some(3),
                auto_collection: Some(true),
                idempotency_key: Some(&idempotency_key),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(subscription.customer.id, customer.id);
        assert_eq!(subscription.plan.external_id.as_deref(), Some("test"));
        assert_eq!(
            subscription.plan.metadata.get("purpose"),
            Some(&"test".to_string())
        );
        assert_eq!(subscription.net_terms, 3);
        assert_eq!(subscription.auto_collection, Some(true));

        // A second creation request tests that the idempotency key is serving
        // its purpose!
        let res = client
            .create_subscription(&CreateSubscriptionRequest {
                customer_id: CustomerId::Orb(&customer.id),
                plan_id: orb_billing::PlanId::External("test"),
                net_terms: Some(11),
                auto_collection: Some(false),
                idempotency_key: Some(&idempotency_key),
                ..Default::default()
            })
            .await;
        match res.expect_err("Request with idempotency key did not error") {
            Error::Api(e) if e.status_code == 409 => println!("Received expected conflict status"),
            x => panic!("Got unexpected error: {x:?}"),
        }

        let fetched_subscription = client.get_subscription(&subscription.id).await.unwrap();
        assert_eq!(fetched_subscription, subscription);

        customers.push(customer);
        subscriptions.push(subscription);
    }

    // Test that listing subscriptions returns all subscriptions.
    let first_subscription = subscriptions[0].created_at;
    let mut fetched_subscriptions: Vec<_> = client
        .list_subscriptions(&SubscriptionListParams::default())
        .try_collect()
        .await
        .unwrap();
    fetched_subscriptions = fetched_subscriptions
        .iter()
        // List returns subscriptions most recent first. Reverse to match ordering
        // of subscriptions.
        .rev()
        // Exclude any subscriptions added as part of cost validation.
        .filter(|sub| sub.plan.external_id != Some("test-complex".into()))
        // Sometimes the tests don't clean up subscriptions from previous runs. Ensure we're only
        // querying subscriptions created within this run by constraining ourselves to those
        // falling on or after the first one was created.
        .filter(|sub| sub.created_at >= first_subscription)
        .cloned()
        .collect();
    assert_eq!(fetched_subscriptions, subscriptions);

    // Test that the list can be filtered to a single customer.
    let fetched_subscriptions: Vec<_> = client
        .list_subscriptions(
            &SubscriptionListParams::default().customer_id(CustomerId::Orb(&customers[0].id)),
        )
        .try_collect()
        .await
        .unwrap();
    assert_eq!(fetched_subscriptions, &[subscriptions.remove(0)]);
}

#[test(tokio::test)]
async fn test_create_backfill() {
    let client = new_client();
    let now = OffsetDateTime::now_utc();
    let response = client.create_backfill(&CreateBackfillParams {
        replace_existing_events: false,
        timeframe_start: now.replace_date(now.date().previous_day().expect("Y10K problem detected")),
        timeframe_end: now,
        close_time: None,
        customer_id: None,
        external_customer_id: None,
    }).await.unwrap();
    println!("{}", serde_json::to_string(&response).unwrap());
}

#[test(tokio::test)]
async fn test_close_backfill() {
    let client = new_client();
    let response = client.close_backfill("KuXfa6NqwM34DSCr".to_string()).await.unwrap();
    println!("{}", serde_json::to_string(&response).unwrap());
}

#[test(tokio::test)]
async fn test_fetch_backfill() {
    let client = new_client();
    let response = client.fetch_backfill("KuXfa6NqwM34DSCr".to_string()).await.unwrap();
    println!("{}", serde_json::to_string(&response).unwrap());
}

#[test(tokio::test)]
async fn test_revert_backfill() {
    let client = new_client();
    let response = client.revert_backfill("W5FLosAUD9KrQWVW".to_string()).await.unwrap();
    println!("{}", serde_json::to_string(&response).unwrap());
}
#[test(tokio::test)]
async fn test_list_backfill() {
    let client = new_client();
    let responses = client.list_backfills();
    for response in responses.try_collect::<Vec<_>>().await.unwrap() {
        println!("{}", serde_json::to_string(&response).unwrap());
    }
}

#[test(tokio::test)]
async fn test_invoices() {
    let client = new_client();

    let invoices: Vec<_> = client
        .list_invoices(&InvoiceListParams::default())
        .try_collect()
        .await
        .unwrap();
    println!("invoices = {:#?}", invoices);

    // TODO: validate list results.
    // TODO: test get_invoice.
}

#[test(tokio::test)]
async fn test_customer_costs() {
    let client = new_client();
    delete_all_test_customers(&client).await;
    let nonce = rand::thread_rng().gen::<u32>();
    let customer = create_test_customer(&client, 0).await;
    let idempotency_key = format!("test-subscription-{nonce}-0");
    let subscription = client
        .create_subscription(&CreateSubscriptionRequest {
            customer_id: CustomerId::Orb(&customer.id),
            plan_id: orb_billing::PlanId::External("test-complex"),
            net_terms: None,
            auto_collection: Some(true),
            idempotency_key: Some(&idempotency_key),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(subscription.customer.id, customer.id);
    assert_eq!(
        subscription.plan.external_id.as_deref(),
        Some("test-complex")
    );
    let costs = client
        .get_customer_costs(
            &customer.id,
            &CustomerCostParams::default().view_mode(CostViewMode::Periodic),
        )
        .await
        .unwrap();
    assert_ne!(costs.len(), 0);
    let cost_bucket = &costs[0];
    assert!(cost_bucket.timeframe_start < cost_bucket.timeframe_end);
    let (matrix_price, price_groups) = &cost_bucket
        .per_price_costs
        .iter()
        .filter_map(|block| match &block.price {
            CustomerCostPriceBlockPrice::Matrix(matrix_price) => {
                Some((matrix_price, block.price_groups.clone().unwrap()))
            }
            _ => None,
        })
        .next()
        .unwrap();
    assert_eq!(matrix_price.matrix_config.default_unit_amount, "1.00");
    assert_eq!(matrix_price.matrix_config.dimensions.len(), 2);
    assert_eq!(
        matrix_price.matrix_config.matrix_values[0].unit_amount,
        "2.00"
    );
    assert_eq!(
        vec![
            price_groups[0].grouping_value.clone(),
            price_groups[0].secondary_grouping_value.clone(),
        ],
        matrix_price.matrix_config.matrix_values[0].dimension_values
    );
    let now = OffsetDateTime::now_utc();
    let then = now.sub(Duration::from_secs(60 * 60 * 24));
    let costs = client
        .get_customer_costs(
            &customer.id,
            &CustomerCostParams::default()
                .view_mode(CostViewMode::Periodic)
                .timeframe_end(&now)
                .timeframe_start(&then),
        )
        .await
        .unwrap();
    assert_eq!(costs.len(), 1);
}

#[test(tokio::test)]
async fn test_errors() {
    let client = new_client();

    let res = client.get_customer("$NOEXIST$").await;
    assert_error_with_status_code(res, StatusCode::NOT_FOUND);

    let res = client.get_customer_by_external_id("$NOEXIST$").await;
    assert_error_with_status_code(res, StatusCode::NOT_FOUND);
}
