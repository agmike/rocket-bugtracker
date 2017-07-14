#![feature(plugin, custom_attribute, custom_derive)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;
extern crate dotenv;
extern crate r2d2;
extern crate r2d2_diesel;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_codegen;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate lazy_static;
extern crate chrono;
extern crate itertools;

mod schema;
mod models;

mod db;

use std::collections::HashMap;
use std::path::PathBuf;

use diesel::prelude::*;
use rocket::*;
use rocket::http::{Cookie, Cookies, Status};
use rocket::request::{Form, FromForm, Outcome, FromRequest, FlashMessage};
use rocket::Outcome::{Success, Failure};
use rocket::response::{Flash, Redirect, NamedFile};
use rocket_contrib::{JSON, Template};
use serde::Serialize;
use chrono::Duration;
use itertools::Itertools;

use db::{DB, DB_POOL};
use models::*;
use schema::*;


const COOKIE_USER_KEY: &str = &"user";


#[derive(Debug)]
struct NoAuth;

#[derive(Debug)]
struct Auth { pub user: User }

impl<'a, 'r> FromRequest<'a, 'r> for Auth {
    type Error = NoAuth;
    fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        println!("Auth::from_request: cookies: {:?}", request.cookies());
        match request.cookies().find(COOKIE_USER_KEY).and_then(|c| c.value().parse::<i32>().ok()) {
            Some(user_id) => {
                let conn = db::DB_POOL.get().unwrap();
                let user = users::table.filter(users::id.eq(user_id)).first(&*conn).unwrap();
                Success(Auth { user })
            },
            None => Failure((Status::Forbidden, NoAuth)),
        }
    }
}

fn shorten_string(str: &mut String) {
    if let Some((i, _)) = str.char_indices().find(|&(i, _)| i >= 80) {
        str.truncate(i);
        str.push_str("...");
    }
}

#[derive(Debug, Serialize)]
struct IndexContext<'a> {
    page_title: &'a str,
    user: User,
    pub issues: Vec<Issue>,
}

#[get("/")]
fn index(db: DB, auth: Result<Auth, NoAuth>) -> Result<Template, Redirect> {
    let user = match auth {
        Ok(auth) => auth.user,
        _ => return Err(Redirect::to("/login"))
    };

    let issues = issues::table.filter(issues::id.ne(GLOBAL_ISSUE_ID)).load::<Issue>(db.conn()).unwrap();
    let issue_tags = IssueTag::belonging_to(&issues).inner_join(tags::table).load::<(IssueTag, Tag)>(db.conn()).unwrap();
    let issue_tags = issue_tags.grouped_by(&issues);

    let mut issues: Vec<Issue> = issues.into_iter().zip(issue_tags.into_iter())
            .filter(|&(_, ref it_t)| !it_t.iter().any(|&(_, ref t)| t.id == CLOSED_TAG_ID))
        .map(|(issue, _)| issue).collect();

    for issue in &mut issues {
        shorten_string(&mut issue.description);
    }

    Ok(Template::render("index", &IndexContext {
        page_title: "главная",
        user,
        issues,
    }))
}

#[derive(Debug, Default, Serialize)]
struct LoginContext {
    pub failed_attempt: bool,
}

#[get("/login")]
fn login(flash: Option<FlashMessage>, cookies: &Cookies) -> Result<Template, Redirect> {
    if cookies.find(COOKIE_USER_KEY).is_some() {
        return Err(Redirect::to("/"));
    }
    let mut ctx = LoginContext::default();
    if flash.map(|f| f.msg().contains("login_failed")).unwrap_or(false) {
        ctx.failed_attempt = true;
    }
    Ok(Template::render("login", &ctx))
}

#[derive(Debug, FromForm)]
struct LoginForm {
    pub email: String,
    pub password: String,
}

#[post("/login", data = "<login_form>")]
fn login_submit(db: DB, login_form: Form<LoginForm>, cookies: &Cookies) -> Result<Redirect, Flash<Redirect>> {
    let login_data = login_form.get();
    println!("attempting login with: {:?}", login_data);
    match users::table.filter(users::email.eq(&login_data.email)
            .and(users::password.eq(&login_data.password))).first::<User>(db.conn()) {
        Ok(user) => {
            cookies.add(Cookie::build(COOKIE_USER_KEY, user.id.to_string())
                .path("/")
                .finish());
            println!("logging in as user {:?}, setting cookies to {:?}", user, cookies);
            Ok(Redirect::to("/"))
        }
        Err(diesel::result::Error::NotFound) => {
            println!("user not found");
            Err(Flash::error(Redirect::to("/login"), "login_failed"))
        },
        err @ _ => { err.unwrap(); unreachable!() }
    }
}

#[get("/logout")]
fn logout(cookies: &Cookies) -> Redirect {
    cookies.remove(COOKIE_USER_KEY);
    Redirect::to("/")
}

#[derive(Debug, Serialize)]
struct NewIssueContext<'a> {
    page_title: &'static str,
    user: &'a User,
}

#[get("/issues/new")]
fn new_issue(auth: Auth) -> Template {
    let ctx = NewIssueContext {
        page_title: "Новый баг",
        user: &auth.user
    };

    Template::render("new_issue", &ctx)
}

#[derive(Debug, FromForm)]
struct NewIssueForm {
    pub title: String,
    pub description: String,
}

#[post("/issues/new", data = "<new_issue_form>")]
fn new_issue_submit(db: DB, auth: Auth, new_issue_form: Form<NewIssueForm>) -> Redirect {
    let new_issue = new_issue_form.into_inner();

    let issue = diesel::insert(&NewIssue {
        title: new_issue.title,
        description: new_issue.description
    }).into(issues::table).get_result::<Issue>(db.conn()).unwrap();

    let action = NewAction {
        user_id: auth.user.id,
        issue_id: issue.id,
        time: chrono::Local::now().naive_local(),
        comment: None,
        add_tag: None,
        remove_tag: None,
        create_issue: Some(issue.id),
        create_user: None,
    };

    let _ = diesel::insert(&action).into(actions::table).get_result::<Action>(db.conn()).unwrap();

    Redirect::to("/")
}

#[derive(Debug, Serialize)]
struct IssuesContext {
    pub user: User,
    pub issues: Vec<IssuesIssueContext>,
}

#[derive(Debug, Serialize)]
struct IssuesIssueContext {
    pub id: i32,
    pub title: String,
    pub description: String,
    pub closed: bool,
    pub tags_str: String,
}

#[get("/issues")]
fn issues(db: DB, auth: Auth) -> Template {
    let issues = issues::table.filter(issues::id.ne(GLOBAL_ISSUE_ID)).load::<Issue>(db.conn()).unwrap();
    let issue_tags = IssueTag::belonging_to(&issues).inner_join(tags::table).load::<(IssueTag, Tag)>(db.conn()).unwrap();
    let issue_tags = issue_tags.grouped_by(&issues);

    let issues = issues.into_iter().zip(issue_tags.into_iter()).map(|(issue, it_t)| {
        let closed = it_t.iter().any(|&(_, ref t)| t.id == CLOSED_TAG_ID);
        let sep = ", ";
        let tags_str = it_t.iter().map(|&(_, ref t)| &t.name[..]).intersperse(sep).collect::<String>();
        let mut ic = IssuesIssueContext {
            id: issue.id,
            title: issue.title,
            description: issue.description,
            closed, tags_str
        };
        shorten_string(&mut ic.description);
        shorten_string(&mut ic.tags_str);
        ic
    }).collect::<Vec<_>>();

    Template::render("issues", &IssuesContext {
        user: auth.user,
        issues,
    })
}

#[derive(Debug, Serialize)]
struct IssueContext<'a> {
    pub page_title: &'a str,
    pub user: &'a User,
    pub issue: &'a Issue,
    pub tags: &'a [Tag],
    pub history: Vec<IssueHistoryContext>,
}

#[derive(Debug, Serialize)]
struct IssueHistoryContext {
    pub user: String,
    pub data: String,
}

#[get("/issues/<id>")]
fn issue(db: DB, auth: Auth, id: i32) -> Template {
    let issue: Issue = issues::table.filter(issues::id.eq(id)).first(db.conn()).unwrap();
    let issue_tags: Vec<(IssueTag, Tag)> = IssueTag::belonging_to(&issue)
        .inner_join(tags::table)
        .load(db.conn()).unwrap();

    let actions: Vec<Action> = actions::table.filter(actions::issue_id.eq(id)).load(db.conn()).unwrap();

    let ctx = IssueContext {
        page_title: &format!("баг #{}", id),
        user: &auth.user,
        issue: &issue,
        tags: &issue_tags.into_iter().map(|(_, t)| t).collect::<Vec<Tag>>(),
        history: actions.into_iter().map(|a| {
            let user = users::table.filter(users::id.eq(a.user_id)).first::<User>(db.conn()).unwrap().name;
            let data = a.comment
                .or(a.add_tag.map(|tag_id| format!("добавил метку {}", tags::table.filter(tags::id.eq(tag_id)).first::<Tag>(db.conn()).unwrap().name)))
                .or(a.remove_tag.map(|tag_id| format!("убрал метку {}", tags::table.filter(tags::id.eq(tag_id)).first::<Tag>(db.conn()).unwrap().name)))
                .or(a.create_issue.map(|issue_id| format!("добавил баг #{}", issue_id)))
                .or(a.create_user.map(|user_id| format!("добавил пользователя {}", users::table.filter(users::id.eq(user_id)).first::<User>(db.conn()).unwrap().name)))
                .unwrap();
            IssueHistoryContext { user, data }
        }).collect(),
    };

    Template::render("issue", &ctx)
}

#[derive(Debug, FromForm)]
struct NewCommentForm {
    pub comment: String,
}

#[post("/issues/<id>/comments/new", data = "<comment_form>")]
fn issue_new_comment_submit(db: DB, auth: Auth, id: i32, comment_form: Form<NewCommentForm>) -> Redirect {
    let new_comment = comment_form.into_inner();

    let action = NewAction {
        user_id: auth.user.id,
        issue_id: id,
        time: chrono::Local::now().naive_local(),
        comment: Some(new_comment.comment),
        add_tag: None,
        remove_tag: None,
        create_issue: None,
        create_user: None,
    };

    let _ = diesel::insert(&action).into(actions::table).get_result::<Action>(db.conn()).unwrap();

    Redirect::to(&format!("/issues/{}", id))
}

#[derive(Debug, Serialize)]
struct TagsContext<'a> {
    pub page_title: &'a str,
    pub user: &'a User,
    pub tags: &'a [Tag],
}

#[get("/tags")]
fn tags(db: DB, auth: Auth) -> Template {
    let tags: Vec<Tag> = tags::table.load(db.conn()).unwrap();

    let ctx = TagsContext {
        page_title: "метки",
        user: &auth.user,
        tags: &tags,
    };

    Template::render("tags", &ctx)
}

#[derive(Debug, FromForm)]
struct NewTagForm {
    pub name: String,
}

#[post("/tags/new", data = "<tag_form>")]
fn new_tag_submit(db: DB, auth: Auth, tag_form: Form<NewTagForm>) -> Redirect {
    let new_tag = tag_form.into_inner();

    let tag = NewTag {
        name: new_tag.name
    };

    let tag = diesel::insert(&tag).into(tags::table).get_result::<Tag>(db.conn()).unwrap();

    let action = NewAction {
        user_id: auth.user.id,
        issue_id: GLOBAL_ISSUE_ID,
        time: chrono::Local::now().naive_local(),
        comment: None,
        add_tag: Some(tag.id),
        remove_tag: None,
        create_issue: None,
        create_user: None,
    };

    let _ = diesel::insert(&action).into(actions::table).get_result::<Action>(db.conn()).unwrap();

    Redirect::to("/tags")
}

#[get("/issues/<issue_id>/add-tag/<tag_id>")]
fn issue_add_tag(db: DB, auth: Auth, issue_id: i32, tag_id: i32) -> Redirect {
    let issue_tag = issue_tags::table.filter(issue_tags::issue_id.eq(issue_id).and(issue_tags::tag_id.eq(tag_id))).first::<IssueTag>(db.conn());
    if let Err(diesel::result::Error::NotFound) = issue_tag {
        let _ = diesel::insert(&NewIssueTag {
            issue_id, tag_id
        }).into(issue_tags::table).get_result::<IssueTag>(db.conn()).unwrap();
    } else {
        issue_tag.unwrap();
    }

    let action = NewAction {
        user_id: auth.user.id,
        issue_id: issue_id,
        time: chrono::Local::now().naive_local(),
        comment: None,
        add_tag: Some(tag_id),
        remove_tag: None,
        create_issue: None,
        create_user: None,
    };

    let _ = diesel::insert(&action).into(actions::table).get_result::<Action>(db.conn()).unwrap();

    Redirect::to(&format!("/issues/{}", issue_id))
}

#[get("/issues/<issue_id>/remove-tag/<tag_id>")]
fn issue_remove_tag(db: DB, auth: Auth, issue_id: i32, tag_id: i32) -> Redirect {
    diesel::delete(issue_tags::table.filter(issue_tags::issue_id.eq(issue_id).and(issue_tags::tag_id.eq(tag_id)))).execute(db.conn()).unwrap();

    let action = NewAction {
        user_id: auth.user.id,
        issue_id: issue_id,
        time: chrono::Local::now().naive_local(),
        comment: None,
        add_tag: None,
        remove_tag: Some(tag_id),
        create_issue: None,
        create_user: None,
    };

    let _ = diesel::insert(&action).into(actions::table).get_result::<Action>(db.conn()).unwrap();

    Redirect::to(&format!("/issues/{}", issue_id))
}

#[get("/json/tags")]
fn json_tags(db: DB, auth: Auth) -> JSON<Vec<Tag>> {
    let tags: Vec<Tag> = tags::table.load(db.conn()).unwrap();
    println!("json_tags: {:?}", tags);
    JSON(tags)
}

#[derive(Debug, Serialize)]
struct UsersContext<'a> {
    page_title: &'a str,
    user: &'a User,
    users: &'a [User]
}

#[get("/users")]
fn users(db: DB, auth: Auth,) -> Template {
    let users = users::table.filter(users::id.ne(ROOT_USER_ID)).load::<User>(db.conn()).unwrap();

    Template::render("users", &UsersContext {
        page_title: "пользователи",
        user: &auth.user,
        users: &users,
    })
}

#[derive(Debug, Serialize)]
struct NewUserContext<'a> {
    page_title: &'a str,
    user: &'a User,
    page_heading: &'a str,
}

#[get("/users/new")]
fn new_user(db: DB, auth: Auth,) -> Template {
    Template::render("new_user", &NewUserContext {
        page_title: "добавление пользователя",
        user: &auth.user,
        page_heading: "Добавление пользователя"
    })
}

#[derive(Debug, FromForm)]
struct NewUserForm {
    pub name: String,
    pub email: String,
    pub password: String,
    pub password_confirm: String,
}

#[post("/users/new", data = "<new_user_form>")]
fn new_user_submit(db: DB, auth: Auth, new_user_form: Form<NewUserForm>) -> Redirect {
    let new_user = new_user_form.into_inner();

    let user = diesel::insert(&NewUser {
        name: new_user.name,
        email: new_user.email,
        password: new_user.password
    }).into(users::table).get_result::<User>(db.conn()).unwrap();

    let action = NewAction {
        user_id: auth.user.id,
        issue_id: GLOBAL_ISSUE_ID,
        time: chrono::Local::now().naive_local(),
        comment: None,
        add_tag: None,
        remove_tag: None,
        create_issue: None,
        create_user: Some(user.id),
    };

    let _ = diesel::insert(&action).into(actions::table).get_result::<Action>(db.conn()).unwrap();

    Redirect::to("/users")
}

#[get("/static/<file..>")]
fn files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(PathBuf::from("static/").join(file)).ok()
}

fn main() {
    let empty = HashMap::<String, String>::with_capacity(0);
    Template::render("common/header", &empty);
    Template::render("common/footer", &empty);

    rocket::ignite().mount("/", routes![
        index, login, login_submit, logout,
        issues, new_issue, new_issue_submit, issue, issue_new_comment_submit, issue_add_tag, issue_remove_tag,
        tags, new_tag_submit, json_tags,
        users, new_user, new_user_submit,
        files
    ]).launch();
}
