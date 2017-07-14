use chrono::NaiveDateTime;

use schema::*;

#[derive(Debug, Queryable, Identifiable, Associations, AsChangeset, Serialize)]
#[has_many(actions)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub password: String,
}

pub const ROOT_USER_ID: i32 = 0;

#[derive(Debug, Insertable)]
#[table_name="users"]
pub struct NewUser {
    pub name: String,
    pub email: String,
    pub password: String
}

#[derive(Debug, Queryable, Identifiable, Associations, AsChangeset, Serialize)]
#[has_many(issue_tags)]
pub struct Issue {
    pub id: i32,
    pub title: String,
    pub description: String,
}

pub const GLOBAL_ISSUE_ID: i32 = 0;

#[derive(Debug, Insertable)]
#[table_name="issues"]
pub struct NewIssue {
    pub title: String,
    pub description: String
}

#[derive(Debug, Queryable, Identifiable, Associations, AsChangeset, Serialize)]
#[has_many(issue_tags)]
pub struct Tag {
    pub id: i32,
    pub name: String,
}

pub const CLOSED_TAG_ID: i32 = 0;

#[derive(Debug, Insertable)]
#[table_name="tags"]
pub struct NewTag {
    pub name: String,
}

#[derive(Debug, Queryable, Identifiable, Associations, AsChangeset, Serialize)]
#[table_name="issue_tags"]
#[belongs_to(Issue)]
#[belongs_to(Tag)]
pub struct IssueTag {
    pub id: i32,
    pub issue_id: i32,
    pub tag_id: i32,
}

#[derive(Debug, Insertable)]
#[table_name="issue_tags"]
pub struct NewIssueTag {
    pub issue_id: i32,
    pub tag_id: i32,
}

#[derive(Debug, Queryable, Identifiable, Associations, AsChangeset, Serialize)]
#[belongs_to(Issue)]
#[belongs_to(User)]
pub struct Action {
    pub id: i32,
    pub user_id: i32,
    pub issue_id: i32,
    pub time: NaiveDateTime,
    pub comment: Option<String>,
    pub add_tag: Option<i32>,
    pub remove_tag: Option<i32>,
    pub create_issue: Option<i32>,
    pub create_user: Option<i32>,
}

#[derive(Debug, Insertable)]
#[table_name="actions"]
pub struct NewAction {
    pub user_id: i32,
    pub issue_id: i32,
    pub time: NaiveDateTime,
    pub comment: Option<String>,
    pub add_tag: Option<i32>,
    pub remove_tag: Option<i32>,
    pub create_issue: Option<i32>,
    pub create_user: Option<i32>,
}
