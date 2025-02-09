use pointercrate_core::{
    permission::{Permission, PermissionsManager},
    pool::PointercratePool,
};
use pointercrate_demonlist::{
    demon::{FullDemon, MinimalDemon, PostDemon},
    player::claim::PlayerClaim,
    record::{FullRecord, RecordStatus, Submission},
    submitter::Submitter,
    LIST_ADMINISTRATOR, LIST_HELPER, LIST_MODERATOR,
};
use pointercrate_user::{AuthenticatedUser, Registration};
use pointercrate_user_pages::account::AccountPageConfig;
use rocket::{
    http::{Header, Status},
    local::asynchronous::{Client, LocalRequest, LocalResponse},
};
use serde::{de::DeserializeOwned, Serialize};
use sqlx::{pool::PoolConnection, PgConnection, Postgres};
use std::{collections::HashMap, net::IpAddr, str::FromStr};

macro_rules! truncate {
    ($conn: expr, $($table: expr),+) => {
        $(
            sqlx::query(concat!("TRUNCATE TABLE ", $table, " CASCADE")).execute(&mut *$conn).await.unwrap();
        )+
    };
}

pub struct TestClient(Client);

impl TestClient {
    fn new(client: Client) -> Self {
        TestClient(client)
    }

    pub fn get(&self, url: impl Into<String>) -> TestRequest {
        TestRequest::new(self.0.get(url.into()))
    }

    pub fn put(&self, url: impl Into<String>) -> TestRequest {
        TestRequest::new(self.0.put(url.into()))
    }

    pub fn post(&self, url: impl Into<String>, body: &impl Serialize) -> TestRequest {
        TestRequest::new(self.0.post(url.into()).json(body))
    }
}

pub struct TestRequest<'c> {
    request: LocalRequest<'c>,
    expected_status: Status,
    expected_headers: HashMap<String, String>,
}

impl<'c> TestRequest<'c> {
    fn new(request: LocalRequest<'c>) -> Self {
        TestRequest {
            request,
            expected_status: Status::Ok,
            expected_headers: HashMap::new(),
        }
        .header("X-Real-Ip", "127.0.0.1")
    }

    pub fn header(mut self, header_name: impl Into<String>, header_value: impl Into<String>) -> Self {
        self.request = self.request.header(Header::new(header_name.into(), header_value.into()));
        self
    }

    pub fn authorize_as(mut self, user: &AuthenticatedUser) -> Self {
        self.header("Authorization", format!("Bearer {}", user.generate_access_token()))
    }

    pub fn expect_status(mut self, status: Status) -> Self {
        self.expected_status = status;
        self
    }

    pub fn expect_header(mut self, header_name: impl Into<String>, header_value: impl Into<String>) -> Self {
        self.expected_headers.insert(header_name.into(), header_value.into());
        self
    }

    pub async fn get_result<Result: DeserializeOwned>(self) -> Result {
        let body_text = self.execute().await.into_string().await.unwrap();
        serde_json::from_str(&body_text).unwrap()
    }

    pub async fn execute(self) -> LocalResponse<'c> {
        let response = self.request.dispatch().await;

        assert_eq!(response.status(), self.expected_status);

        for (name, value) in self.expected_headers {
            let header = response.headers().get_one(&name);

            assert!(header.is_some(), "missing required header value");

            assert_eq!(header.unwrap(), value);
        }

        response
    }
}

pub async fn setup() -> (TestClient, PoolConnection<Postgres>) {
    dotenv::dotenv().unwrap();

    let pool = PointercratePool::init().await;
    let mut connection = pool.connection().await.unwrap();

    // reset test database
    truncate!(
        &mut connection,
        "records",
        "demons",
        "player_claims",
        "players",
        "members",
        "submitters"
    );

    let permissions = PermissionsManager::new(vec![LIST_HELPER, LIST_MODERATOR, LIST_ADMINISTRATOR])
        .assigns(LIST_ADMINISTRATOR, LIST_MODERATOR)
        .implies(LIST_ADMINISTRATOR, LIST_MODERATOR)
        .implies(LIST_MODERATOR, LIST_HELPER);

    let rocket = pointercrate_demonlist_api::setup(rocket::build().manage(pool))
        .manage(permissions)
        .manage(AccountPageConfig::default());

    // generate some data
    Submitter::create_submitter(IpAddr::from_str("127.0.0.1").unwrap(), &mut connection)
        .await
        .unwrap();

    (TestClient::new(Client::tracked(rocket).await.unwrap()), connection)
}

pub async fn system_user_with_perms(perm: Permission, connection: &mut PgConnection) -> AuthenticatedUser {
    let user = AuthenticatedUser::register(
        Registration {
            name: "Patrick".to_string(),
            password: "bad password".to_string(),
        },
        &mut *connection,
    )
    .await
    .unwrap();

    sqlx::query!(
        "UPDATE members SET permissions = $2::INTEGER::BIT(16) WHERE member_id = $1",
        user.inner().id,
        perm.bit() as i16
    )
    .execute(connection)
    .await
    .unwrap();

    user
}

pub async fn add_normal_user(connection: &mut PgConnection) -> AuthenticatedUser {
    AuthenticatedUser::register(
        Registration {
            name: "Patrick".to_string(),
            password: "bad password".to_string(),
        },
        connection,
    )
    .await
    .unwrap()
}

pub async fn add_demon(
    name: impl Into<String>, position: i16, requirement: i16, verifier_id: i32, publisher_id: i32, connection: &mut PgConnection,
) -> i32 {
    sqlx::query!(
        "INSERT INTO demons (name, position, requirement, verifier, publisher) VALUES ($1::TEXT::CITEXT, $2, $3, $4, $5) RETURNING id",
        name.into(),
        position,
        requirement,
        verifier_id,
        publisher_id
    )
    .fetch_one(&mut *connection)
    .await
    .unwrap()
    .id
}

pub async fn put_claim(user_id: i32, player_id: i32, verified: bool, lock_submissions: bool, connection: &mut PgConnection) -> PlayerClaim {
    sqlx::query!(
        "INSERT INTO player_claims (member_id, player_id, verified, lock_submissions) VALUES ($1, $2, $3, $4)",
        user_id,
        player_id,
        verified,
        lock_submissions
    )
    .execute(connection)
    .await
    .unwrap();

    PlayerClaim {
        user_id,
        player_id,
        verified,
        lock_submissions,
    }
}

pub async fn add_simple_record(progress: i16, player: i32, demon: i32, status: RecordStatus, connection: &mut PgConnection) -> i32 {
    let system_sub = Submitter::by_ip(IpAddr::from_str("127.0.0.1").unwrap(), &mut *connection)
        .await
        .unwrap()
        .unwrap();

    sqlx::query!(
        "INSERT INTO records (progress, status_, player, submitter, demon, video) VALUES ($1, $2::text::record_status, $3, $4, $5, NULL) \
         RETURNING id",
        progress,
        status.to_sql(),
        player,
        system_sub.id,
        demon
    )
    .fetch_one(&mut *connection)
    .await
    .unwrap()
    .id
}
