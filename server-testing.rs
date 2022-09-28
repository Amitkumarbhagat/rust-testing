#[derive(Deserialize)]
pub struct Todo {
    pub id: i32,
    pub name: String,
    pub checked: bool,
}

#[derive(Clone)]
pub struct DBAccess {
    pub db_pool: DBPool,
}

const INIT_SQL: &str = "./db.sql";

pub fn create_pool() -> std::result::Result<DBPool, mobc::Error<Error>> {
    let config = Config::from_str("postgres://postgres@127.0.0.1:7878/postgres")?;
    Ok(Pool::builder().build(PgConnectionManager::new(config, NoTls)))
}

impl DBAccess {
    pub fn new(db_pool: DBPool) -> Self {
        Self { db_pool }
    }

    pub async fn init_db(&self) -> Result<()> {
        let init_file = fs::read_to_string(INIT_SQL)?;
        let con = self.get_db_con().await?;
        con.batch_execute(init_file.as_str())
            .await
            .map_err(DBInitError)?;
        Ok(())
    }

    async fn get_db_con(&self) -> Result<DBCon> {
        self.db_pool.get().await.map_err(DBPoolError)
    }

    fn row_to_todo(&self, row: &Row) -> Todo {
        let id: i32 = row.get(0);
        let name: String = row.get(1);
        let checked: bool = row.get(2);
        Todo { id, name, checked }
    }
}

#[async_trait]
impl DBAccessor for DBAccess {
    async fn fetch_todos(&self) -> Result<Vec<Todo>> {
        let con = self.get_db_con().await?;
        let query = "SELECT id, name, checked FROM todo ORDER BY id ASC";
        let q = con.query(query, &[]).await;
        let rows = q.map_err(DBQueryError)?;

        Ok(rows.iter().map(|r| self.row_to_todo(&r)).collect())
    }

    async fn create_todo(&self, name: String) -> Result<Todo> {
        let con = self.get_db_con().await?;
        let query = "INSERT INTO todo (name) VALUES ($1) RETURNING *";
        let row = con.query_one(query, &[&name]).await.map_err(DBQueryError)?;
        Ok(self.row_to_todo(&row))
    }
}

#[derive(Clone)]
pub struct Client {
    client: HyperClient<HttpsConnector<HttpConnector>>,
}

#[derive(Debug, Deserialize)]
pub struct CatFact {
    pub text: String,
}

impl Client {
    pub fn new() -> Self {
        let HTTPs = HttpsConnector::new();
        Self {
            client: HyperClient::builder().build::<_, Body>(https),
        }
    }

    fn get_url(&self) -> String {
        URI.to_owned()
    }
}

#[async_trait]
impl HttpClient for Client {
    async fn get_cat_fact(&self) -> Result<String> {
        let req = Request::builder()
            .method(Method::GET)
            .uri(&format!("{}{}", self.get_url(), "/facts/random"))
            .header("content-type", "application/json")
            .header("accept", "application/json")
            .body(Body::empty())?;
        let res = self.client.request(req).await?;
        if !res.status().is_success() {
            return Err(error::Error::GetCatFactError(res.status()));
        }
        let body_bytes = to_bytes(res.into_body()).await?;
        let json = from_slice::<CatFact>(&body_bytes)?;
        Ok(json.text)
    }
}