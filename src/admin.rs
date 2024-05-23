use rocket::form::Form;
use rocket::http::Status;
use rocket::response::Redirect;
use rocket::{get, post, routes, Route};
use rocket::State;
use sled::Db;
use std::sync::Arc;
use tokio::fs; // Import tokio::fs for async file operations
use rocket::response::content::RawHtml; // Import RawHtml for HTML responses
use crate::load_template; // Import load_template function
use crate::generate_post_html; // Import generate_post_html function
use crate::Post; // Import Post struct
use crate::ADMIN_PATH; // Import ADMIN_PATH constant
use chrono::Utc; // Import Utc for timestamp handling

#[derive(FromForm)]
struct EditForm {
    directory: String,
    title: String,
    message: String,
    password: String,
}

#[derive(FromForm)]
struct DeleteForm {
    directory: String,
    password: String,
}

pub struct AdminState {
    db: Arc<Db>,
    admin_password: String,
}

impl AdminState {
    pub fn new(db: Arc<Db>, admin_password: String) -> Self {
        AdminState { db, admin_password }
    }
}

#[post("/edit", data = "<form>")]
async fn edit_post(form: Form<EditForm>, state: &State<AdminState>) -> Result<Redirect, Status> {
    if form.password != state.admin_password {
        return Err(Status::Unauthorized);
    }

    if let Some(mut post) = state.db.get(&form.directory).ok().flatten().and_then(|v| serde_json::from_slice::<Post>(&v).ok()) {
        post.title = form.title.clone();
        post.message = form.message.clone();
        let value = serde_json::to_vec(&post).map_err(|_| Status::InternalServerError)?;
        state.db.insert(&form.directory, value).map_err(|_| Status::InternalServerError)?;

        let delete_option = if Utc::now().timestamp() - post.timestamp <= 120 {
            format!(
                r#"<div style="text-align: right; margin-top: 20px;">
                    <form action="/delete_post" method="post">
                        <input type="hidden" name="directory" value="{}">
                        <button type="submit" style="background-color: red; color: white; padding: 5px 10px; border: none; border-radius: 3px; cursor: pointer;">Delete</button>
                    </form>
                   </div>"#,
                post.directory
            )
        } else {
            String::new()
        };

        let post_html = generate_post_html(&post.title, &post.message, post.media.as_deref(), &delete_option);
        fs::write(format!("articles/{}/index.html", form.directory), post_html).await.map_err(|_| Status::InternalServerError)?;

        Ok(Redirect::to(format!("/articles/{}/index.html", form.directory)))
    } else {
        Err(Status::NotFound)
    }
}

#[post("/delete", data = "<form>")]
async fn delete_post(form: Form<DeleteForm>, state: &State<AdminState>) -> Result<Redirect, Status> {
    if form.password != state.admin_password {
        return Err(Status::Unauthorized);
    }

    if state.db.get(&form.directory).ok().flatten().and_then(|v| serde_json::from_slice::<Post>(&v).ok()).is_some() {
        state.db.remove(&form.directory).map_err(|_| Status::InternalServerError)?;
        fs::remove_dir_all(format!("articles/{}", form.directory)).await.map_err(|_| Status::InternalServerError)?;
        Ok(Redirect::to("/"))
    } else {
        Err(Status::NotFound)
    }
}

#[get("/")]
async fn admin_panel() -> RawHtml<String> {
    let template = load_template("admin_panel");
    let content = template.replace("{{admin_path}}", ADMIN_PATH);
    RawHtml(content)
}

pub fn routes() -> Vec<Route> {
    routes![admin_panel, edit_post, delete_post]
}
