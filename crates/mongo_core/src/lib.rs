use futures::stream::TryStreamExt;
pub use mongodb::bson;
use mongodb::{
    bson::{doc, Document},
    options::ClientOptions,
    Client,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub struct MongoCore {
    pub client: Arc<Mutex<Option<Client>>>,
}

impl Default for MongoCore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollectionInfo {
    pub name: String,
}

#[derive(Debug, Clone, Default)]
pub struct FindOptions {
    pub filter: Option<Document>,
    pub projection: Option<Document>,
    pub sort: Option<Document>,
    pub limit: Option<i64>,
    pub skip: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DatabaseInfo {
    pub name: String,
    pub collections: Vec<CollectionInfo>,
}

impl MongoCore {
    pub fn new() -> Self {
        Self {
            client: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn connect(&self, uri: &str) -> anyhow::Result<()> {
        let client_options = ClientOptions::parse(uri).await?;
        let client = Client::with_options(client_options)?;
        let mut guard = self.client.lock().await;
        *guard = Some(client);
        Ok(())
    }

    pub async fn list_databases(&self) -> anyhow::Result<Vec<DatabaseInfo>> {
        let guard = self.client.lock().await;
        let Some(client) = &*guard else {
            return Ok(vec![]);
        };

        let db_names = client.list_database_names().await?;
        let mut databases = Vec::new();

        for db_name in db_names {
            let db = client.database(&db_name);
            let collection_names = db.list_collection_names().await?;
            let collections = collection_names
                .into_iter()
                .map(|name| CollectionInfo { name })
                .collect();
            databases.push(DatabaseInfo {
                name: db_name,
                collections,
            });
        }
        Ok(databases)
    }

    pub async fn find_documents(
        &self,
        db_name: &str,
        collection_name: &str,
        options: FindOptions,
    ) -> anyhow::Result<Vec<Document>> {
        let guard = self.client.lock().await;
        let Some(client) = &*guard else {
            return Ok(vec![]);
        };

        let db = client.database(db_name);
        let collection = db.collection::<Document>(collection_name);

        let mut find = collection.find(options.filter.unwrap_or_default());
        if let Some(projection) = options.projection {
            find = find.projection(projection);
        }
        if let Some(sort) = options.sort {
            find = find.sort(sort);
        }
        if let Some(limit) = options.limit {
            find = find.limit(limit);
        }
        if let Some(skip) = options.skip {
            find = find.skip(skip);
        }

        let mut cursor = find.await?;
        let mut docs = Vec::new();

        while let Some(doc) = cursor.try_next().await? {
            docs.push(doc);
        }

        Ok(docs)
    }

    pub async fn count_documents(
        &self,
        db_name: &str,
        collection_name: &str,
        filter: Option<Document>,
    ) -> anyhow::Result<u64> {
        let guard = self.client.lock().await;
        let Some(client) = &*guard else {
            return Ok(0);
        };

        let db = client.database(db_name);
        let collection = db.collection::<Document>(collection_name);
        let count = collection
            .count_documents(filter.unwrap_or_default())
            .await?;
        Ok(count)
    }

    pub async fn get_collection_schema(
        &self,
        db_name: &str,
        collection_name: &str,
    ) -> anyhow::Result<Vec<String>> {
        let guard = self.client.lock().await;
        let Some(client) = &*guard else {
            return Ok(vec![]);
        };

        let db = client.database(db_name);
        let collection = db.collection::<Document>(collection_name);

        let pipeline = vec![doc! { "$sample": { "size": 1 } }];
        let mut cursor = collection.aggregate(pipeline).await?;

        if let Some(doc) = cursor.try_next().await? {
            let keys: Vec<String> = doc.keys().map(|k| k.to_string()).collect();
            return Ok(keys);
        }

        Ok(vec![])
    }
}
