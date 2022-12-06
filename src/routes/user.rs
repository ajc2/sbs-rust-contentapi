use crate::api_data::*;
use crate::context::*;
use crate::forms;
use crate::conversion;
use crate::config;
use crate::api::*;
use super::*;

use std::collections::HashMap;
use anyhow::anyhow;
use rocket::http::{CookieJar, Cookie};
use rocket::form::Form;
use rocket::response::Redirect;
use rocket_dyn_templates::Template;

macro_rules! userhome_base {
    ($context:ident, { $($uf:ident : $uv:expr),*$(,)* }) => {
        {
            let mut userpage : Option<Content> = None;

            if let Some(ref user) = $context.current_user {
                let mut request = FullRequest::new();
                add_value!(request, "uid", user.id);
                let mut user_request = build_request!(
                    RequestType::content, 
                    String::from("*"), //ok do we really need it ALL?
                    String::from("!userpage(@uid)")
                ); 
                user_request.name = Some(String::from("userpage"));
                request.requests.push(user_request);

                let result = post_request(&$context, &request).await?;

                let mut userpage_raw = conversion::cast_result_safe::<Content>(&result, "userpage")?;
                userpage = userpage_raw.pop(); //Doesn't matter if it's none
            }

            basic_template!("userhome", $context, {
                userprivate : crate::api::get_user_private_safe(&$context).await,
                userpage : userpage,
                $($uf: $uv,)*
            })
        }
    };
}
use serde_json::json;
pub(crate) use userhome_base;

pub async fn post_userbio(context: &Context, form: forms::UserBio<'_>) -> Result<Content, anyhow::Error>
{
    if let Some(ref user) = context.current_user {
        let mut request = FullRequest::new();
        add_value!(request, "type", "userpages"); //Need the parent

        let mut parent_request = build_request!(
            RequestType::content, 
            String::from("id,parentId,literalType"), 
            String::from("!userpage(@uid)")
        ); 
        parent_request.name = Some(String::from("parent"));
        request.requests.push(parent_request);

        let result = post_request(context, &request).await?;

        let mut parents_raw = conversion::cast_result_required::<Content>(&result, "parent")?;

        match parents_raw.pop() {
            Some(parent) => {
                let mut content = Content::default();
                content.text = Some(String::from(form.text));
                content.id = Some(form.id);
                content.parentId = parent.id;
                content.contentType = Some(ContentType::USERPAGE);
                content.name = Some(format!("{}'s userpage", user.username));
                content.values = Some(make_values! {
                    "markup": "bbcode"
                });
                post_content(context, &content).await.map_err(|e| e.into())
            }
            None => {
                Err(anyhow!("Couldn't find the userpage parent! This is a programming error!"))
            }
        }
    }
    else {
        Err(anyhow!("Not logged in!"))
    }
}

#[get("/login")]
pub async fn login_get(context: Context) -> Result<Template, RouteError> {
    Ok(basic_template!("login", context, {}))
}

#[get("/userhome")]
pub async fn userhome_get(context: Context) -> Result<Template, RouteError> {
    Ok(userhome_base!(context, {}))
}

#[post("/login", data = "<login>")]
pub async fn login_post(context: Context, login: Form<forms::Login<'_>>, jar: &CookieJar<'_>) -> Result<MultiResponse, RouteError> {
    let new_login = conversion::convert_login(&context, &login);
    match post_login(&context, &new_login).await
    {
        Ok(result) => {
            login!(jar, context, result, new_login.expireSeconds);
            Ok(MultiResponse::Redirect(my_redirect!(context.config, "/userhome")))
        },
        Err(error) => {
            Ok(MultiResponse::Template(basic_template!("login", context, {errors: vec![error.to_string()]})))
        } 
    }
}

//Alternate post endpoint for sending the recovery endpoint. On success, go to the /recover page, which
//will let you finalize setting a new password
#[post("/login?recover", data = "<recover>")]
pub async fn loginrecover_post(context: Context, recover: Form<forms::LoginRecover<'_>>) -> Result<MultiResponse, RouteError> {
    let mut errors = Vec::new();
    handle_email!(post_email_recover(&context, recover.email).await, errors);
    let template = if errors.len() == 0 { "recover" } else { "login" };
    //Error goes back to login template, but success goes to special reset page
    Ok(MultiResponse::Template(basic_template!(template, context, {
        emailresult : String::from(recover.email),  //This is 'email' because it's just SENDING the recovery email, not the recover form
        recovererrors: errors
    })))
}

#[get("/recover")] //A plain page render, if you accidentally get here. THe page will still work, but you have to add crap
pub async fn recover_get(context: Context) -> Result<Template, RouteError> {
    Ok(basic_template!("recover", context, { }))
}

//Dedicated recover submit page. On succcess, login and go to userhome. On failure, show recover page again
#[post("/recover", data = "<sensitive>")]
pub async fn recover_usersensitive_post(context: Context, sensitive: Form<forms::UserSensitive<'_>>, jar: &CookieJar<'_>) -> Result<MultiResponse, RouteError> {
    match post_usersensitive(&context, &sensitive).await {
        Ok(token) => {
            login!(jar, context, token);
            Ok(MultiResponse::Redirect(my_redirect!(context.config, "/userhome")))
        },
        Err(error) => {
            //This NEEDS to be the same as the post render from /login?recover!
            Ok(MultiResponse::Template(basic_template!("recover", context, {
                emailresult: String::from(sensitive.currentEmail),
                recovererrors: vec![error.to_string()]
            })))
        }
    }
}

//The userhome version of updating the sensitive info. This one actually has the ability to change your email
#[post("/userhome?sensitive", data = "<sensitive>")]
pub async fn usersensitive_post(context: Context, sensitive: Form<forms::UserSensitive<'_>>) -> Result<MultiResponse, RouteError> {
    let mut errors = Vec::new();
    match post_usersensitive(&context, &sensitive).await {
        Ok(_token) => {} //Don't need the token
        Err(error) => { errors.push(error.to_string()) }
    };
    Ok(MultiResponse::Template(userhome_base!(context, {sensitiveerrors:errors})))
}


#[post("/userhome", data= "<update>")]
pub async fn userhome_update_post(mut context: Context, update: Form<forms::UserUpdate<'_>>) -> Result<Template, RouteError>
{
    let mut errors = Vec::new();
    //If the user is there, get a copy of it so we can modify and post it
    if let Some(mut current_user) = context.current_user.clone() {
        //Modify
        current_user.username = String::from(update.username);
        current_user.avatar = String::from(update.avatar);
        //Either update the context user or set an error
        match post_userupdate(&context, &current_user).await { 
            Ok(new_user) => context.current_user = Some(new_user),
            Err(error) => errors.push(error.to_string())
        }
    }
    else {
        errors.push(String::from("Couldn't pull user data, are you still logged in?"));
    }
    Ok(userhome_base!(context, {updateerrors:errors}))
}

//Don't need the heavy lifting of an entire context just for logout 
#[get("/logout")]
pub fn logout_get(config: &rocket::State<config::Config>, jar: &CookieJar<'_>) -> Redirect {
    jar.remove(Cookie::named(config.token_cookie_key.clone()));
    my_redirect!(config, "/")
}

#[get("/user/<username>")]
pub async fn user_get(context: Context, username: String) -> Result<Template, RouteError>
{
    //Go get the user and their userpage
    let mut request = FullRequest::new();
    add_value!(request, "username", username);

    request.requests.push(build_request!(
        RequestType::user, 
        String::from("*"), 
        String::from("username = @username")
    )); 

    request.requests.push(build_request!(
        RequestType::content, 
        String::from("*"), //ok do we really need it ALL?
        String::from("!userpage(@user.id)")
    )); 

    let result = post_request(&context, &request).await?;

    //Now try to parse two things out of it
    let users_raw = conversion::cast_result_required::<User>(&result, "user")?;
    let content_raw = conversion::cast_result_required::<Content>(&result, "content")?;

    Ok(basic_template!("user", context, {
        pageuser: users_raw.get(0),
        pageuserbio: content_raw.get(0)
    }))
}