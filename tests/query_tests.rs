use cryptonamo::{traits::DecryptedRecord, Cryptonamo, EncryptedTable, Plaintext};
use serial_test::serial;
use std::{collections::HashMap, future::Future};

#[derive(Debug, PartialEq, Cryptonamo)]
#[cryptonamo(partition_key = "email")]
#[cryptonamo(sort_key_prefix = "user")]
pub struct User {
    #[cryptonamo(query = "exact", compound = "email#name")]
    #[cryptonamo(query = "exact")]
    pub email: String,

    #[cryptonamo(query = "prefix", compound = "email#name")]
    #[cryptonamo(query = "prefix")]
    pub name: String,
}

impl User {
    fn new(email: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            email: email.into(),
        }
    }
}

impl DecryptedRecord for User {
    fn from_attributes(attributes: HashMap<String, Plaintext>) -> Self {
        Self {
            email: attributes.get("email").unwrap().try_into().unwrap(),
            name: attributes.get("name").unwrap().try_into().unwrap(),
        }
    }
}

async fn run_test<F: Future<Output = ()>>(f: impl FnOnce(EncryptedTable) -> F) {
    let config = aws_config::from_env()
        .endpoint_url("http://localhost:8000")
        .load()
        .await;

    let client = aws_sdk_dynamodb::Client::new(&config);

    let table = EncryptedTable::init(client, "users")
        .await
        .expect("Failed to init table");

    table
        .put(&User::new("dan@coderdan.co", "Dan Draper"))
        .await
        .expect("Failed to insert Dan");

    table
        .put(&User::new("jane@smith.org", "Jane Smith"))
        .await
        .expect("Failed to insert Jane");

    table
        .put(&User::new("daniel@example.com", "Daniel Johnson"))
        .await
        .expect("Failed to insert Daniel");

    f(table).await;
}

#[tokio::test]
#[serial]
async fn test_query_single_exact() {
    run_test(|table| async move {
        let res: Vec<User> = table
            .query()
            .eq("email", "dan@coderdan.co")
            .send()
            .await
            .expect("Failed to query");

        assert_eq!(res, vec![User::new("dan@coderdan.co", "Dan Draper")]);
    })
    .await;

    run_test(|table| async move {
        let res: Vec<User> = table
            .query()
            .starts_with("name", "Dan")
            .send()
            .await
            .expect("Failed to query");

        assert_eq!(
            res,
            vec![
                User::new("dan@coderdan.co", "Dan Draper"),
                User::new("daniel@example.com", "Daniel Johnson")
            ]
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_query_compound() {
    run_test(|table| async move {
        let res: Vec<User> = table
            .query()
            .starts_with("name", "Dan")
            .eq("email", "dan@coderdan.co")
            .send()
            .await
            .expect("Failed to query");

        assert_eq!(res, vec![User::new("dan@coderdan.co", "Dan Draper")]);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_get_by_partition_key() {
    run_test(|table| async move {
        let res: Option<User> = table.get("dan@coderdan.co").await.expect("Failed to send");
        assert_eq!(res, Some(User::new("dan@coderdan.co", "Dan Draper")));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_delete() {
    run_test(|table| async move {
        table
            .delete::<User>("dan@coderdan.co")
            .await
            .expect("Failed to send");

        let res = table
            .get::<User>("dan@coderdan.co")
            .await
            .expect("Failed to send");
        assert_eq!(res, None);

        let res = table
            .query::<User>()
            .starts_with("name", "Dan")
            .send()
            .await
            .expect("Failed to send");
        assert_eq!(res, vec![User::new("daniel@example.com", "Daniel Johnson")]);

        let res = table
            .query::<User>()
            .eq("email", "dan@coderdan.co")
            .send()
            .await
            .expect("Failed to send");
        assert_eq!(res, vec![]);

        let res = table
            .query::<User>()
            .eq("email", "dan@coderdan.co")
            .starts_with("name", "Dan")
            .send()
            .await
            .expect("Failed to send");
        assert_eq!(res, vec![])
    })
    .await;
}