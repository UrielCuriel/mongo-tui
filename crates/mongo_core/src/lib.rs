use std::sync::Arc;
pub use mongodb::bson;
use mongodb::{
    bson::{Document, doc},
    Client, options::ClientOptions,
};
use futures::stream::TryStreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub struct MongoCore {
    pub client: Arc<Mutex<Option<Client>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollectionInfo {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DatabaseInfo {
    pub name: String,
    pub collections: Vec<CollectionInfo>,
}

impl MongoCore {
    pub fn new() -> Self {
        Self { client: Arc::new(Mutex::new(None)) }
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
        filter: Option<Document>,
        projection: Option<Document>,
        sort: Option<Document>,
        limit: Option<i64>,
        skip: Option<u64>,
    ) -> anyhow::Result<Vec<Document>> {
        let guard = self.client.lock().await;
        let Some(client) = &*guard else {
            return Ok(vec![]);
        };

        let db = client.database(db_name);
        let collection = db.collection::<Document>(collection_name);

        let mut find = collection.find(filter.unwrap_or_default());
        if let Some(projection) = projection {
            find = find.projection(projection);
        }
        if let Some(sort) = sort {
            find = find.sort(sort);
        }
        if let Some(limit) = limit {
            find = find.limit(limit);
        }
        if let Some(skip) = skip {
            find = find.skip(skip);
        }

        let mut cursor = find.await?;
        let mut docs = Vec::new();

        while let Some(doc) = cursor.try_next().await? {
            docs.push(doc);
        }

        Ok(docs)
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

        let pipeline = vec![
            doc! { "$sample": { "size": 1 } },
        ];
        let mut cursor = collection.aggregate(pipeline).await?;

        if let Some(doc) = cursor.try_next().await? {
             let keys: Vec<String> = doc.keys().map(|k| k.to_string()).collect();
             return Ok(keys);
        }

        Ok(vec![])
    }
}
