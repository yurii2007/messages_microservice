use diesel::prelude::*;

use crate::schema::messages;

#[derive(Queryable, Selectable)]
#[diesel(table_name = messages, check_for_backend(diesel::pg::Pg))]
pub struct Message {
    pub id: i32,
    pub username: String,
    pub message: String,
    pub timestamp: i64,
}

#[derive(Insertable)]
#[diesel(table_name = messages)]
pub struct NewMessage {
    pub username: String,
    pub message: String,
}
