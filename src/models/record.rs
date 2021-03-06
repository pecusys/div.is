//use sqlx::{Sqlite, FromRow, Sqlite::*};
use sqlx::{Sqlite, FromRow, sqlite::*};
use chrono::{DateTime, Utc};
use crate::db::Db;
use std::collections::HashMap;
use std::rc::Rc;
use super::{
    Time, Model, Status, Permission,
    entry::EntryType,
    user::User, item::Item,
    link::{UserRecordLink, RecordItemLink},
};

// TODO: Think out "tagging" functionality for records --
// want this to be "default", but also want it to be implemented
// in a more dynamic way than simply adding a tag, taglink table --
// handle through items associated with record?

#[derive(Default, FromRow, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Record {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,
    pub uid: i32,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default = "Status::active")]
    pub status: String,
    #[serde(default = "Permission::private")]
    pub permission: String,
    #[serde(default = "Time::now")]
    pub created_at: i32,
}

// NOTE: pass user ID into all user header requests, so it can be passed ot Record::new()
// TODO: make sure uid exists (user exists) before inserting into db, etc.
impl Record {

    pub fn new(uid: i32, name: String) -> Record {
        Self { 
            id: None, uid, name, 
            description: None,
            status: Status::active(),
            permission: Permission::private(),
            created_at: Time::now(),
        }
    }

    pub async fn from_id(db: &Db, id: i32) -> sqlx::Result<Self> {
        let record = sqlx::query_as::<_, Self>("SELECT * FROM Records WHERE id=?;")  
            .bind(id)
            .fetch_one(&db.pool).await?;
        Ok(record)
    }

    pub async fn from_uid(db: &Db, uid: i32) -> sqlx::Result<Vec<Self>> {
        let records = sqlx::query_as::<_, Record>( 
            "SELECT * FROM Records WHERE uid= ?;")
            .bind(uid)
            .fetch_all(&db.pool).await?;
        for record in &records {
            println!("record: {}", &record.id.unwrap());
            println!("{}", &record.to_string());
        }
        Ok(records)
    }

    pub async fn insert(self, db: &Db) 
    -> sqlx::Result<Self> {
        sqlx::query("INSERT INTO Records 
        (uid, name, status, private, created_at) 
        VALUES ($1, $2, $3, $4, $5);")  
            .bind(&self.uid)
            .bind(&self.name)
            .bind(&self.status)
            .bind(&self.permission)
            .bind(Time::now())
            .execute(&db.pool).await?;
        // TODO: Create userRecordLink by retrieveing rid after inserting new record
        //UserRecordLink::create(db, self.uid, self.id.unwrap()).await?;
        Ok(self)
    }

    pub async fn get_items(self, db: &Db) -> sqlx::Result<Vec<Item>> {
        let items: Vec<Item> = sqlx::query_as::<Sqlite, Item>("
            SELECT * FROM Items INNER JOIN RecordItemLinks 
            ON RecordItemLinks.iid=Items.id WHERE RecordItemLinks.iid=?;")
            .bind(&self.id)
            .fetch_all(&db.pool).await?;
        Ok(items)
    }

    pub async fn add_user(self, db: &Db, uid: i32) -> sqlx::Result<Self> {
        UserRecordLink::create(db, uid, self.id.unwrap()).await?;
        Ok(self)
    }

    pub async fn get_users(self, db: &Db) -> sqlx::Result<Vec<User>> {
        let users: Vec<User> = sqlx::query_as::<Sqlite, User>("
            SELECT * FROM Users INNER JOIN UserRecordLinks
            ON UserRecordLinks.uid=Users.id WHERE UserRecordLinks.rid=?;")
            .bind(&self.id)
            .fetch_all(&db.pool).await?;
        Ok(users)
    }

    // TODO: Get records where user has UserRecordLink association,
    // but record's uid != user's uid
    pub async fn associated_with_user(db: &Db, user: &User) -> sqlx::Result<Vec<Self>> {
        let records: Vec<Self> = sqlx::query_as::<Sqlite, Self>("
            SELECT * FROM Records INNER JOIN UserRecordLinks
            ON UserRecordLinks.uid=Users.id 
            WHERE UserRecordLinks.uid=?
              AND Records.uid!=?;")
            .bind(user.id)
            .fetch_all(&db.pool).await?;
        Ok(records)
    }


    pub async fn create_entry_type(&self, uid: i32, name: String) -> EntryType {
        EntryType {
            id: None, uid: uid, name: name,
            permission: Permission::private(),
            status: Status::active(),
            created_at: Time::now(),
        }
   }

    pub async fn insert_with_items(
        db: &Db,
        uid: i32, 
        name: String, 
        items: Option<HashMap<i32, String>>
    ) -> sqlx::Result<Record> {
        let record = Record::new(uid, name)
            .insert(&db).await?;
        match items {
            Some(items) => {
                for iid in items.keys() {
                    let itm = Item::from_id(&db, iid.to_owned()).await?;
                    let pri = items.get(iid).unwrap();
                    RecordItemLink::create(db, record.id.unwrap(),
                        itm.id.unwrap(), pri.to_owned()).await?;
                }
            },
            None => {}
        }
        Ok(record)
    }

    pub async fn create_link_to_item(self, 
        db: &Db, 
        item_id: i32, 
        priority: String,
    ) -> sqlx::Result<Self> {
        RecordItemLink::create(&db, self.id.unwrap(), item_id, priority).await?;
        Ok(self)
    }

    pub async fn clear_archived(self) -> () { () }

}

pub struct RecordBuilder {
    pub id: Option<i32>,
    pub name: Option<String>,
    pub items: Option<Vec<Rc<Item>>>,
}

impl RecordBuilder {
    pub fn with_name(mut self, name: String) -> RecordBuilder {
        self.name = Some(name); self
    }
    //pub fn build(self) -> Record {
        //Record::default()
    //}
}

impl Model for Record {}

// $07/25/20$  not impl in sql
//
pub enum Relation {
    Master(i32, i32), //not sure if i want "following?"
    Slave(i32, i32),
    Synchronize(i32, i32),
    NoRelationship,
}
impl Relation {
    pub fn none() -> String { "none".to_string() }
}

#[derive(Default, FromRow, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct RecordRelation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,
    pub rid1: i32,
    pub rid2: i32,
    #[serde(default = "Relation::none")]
    pub rel: String,
    #[serde(default = "Time::now")]
    pub created_at: i32,
    #[serde(default = "Time::now")]
    pub updated_at: i32,
}

impl RecordRelation {

}
