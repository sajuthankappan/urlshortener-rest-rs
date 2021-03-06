#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]
#![feature(custom_attribute)]

extern crate bodyparser;
extern crate hyper;
extern crate iron;
extern crate mount;
extern crate unicase;
extern crate serde;
extern crate serde_json;
extern crate router;
extern crate urlshortener;

use hyper::mime::{Mime};
use iron::AfterMiddleware;
use iron::headers;
use iron::method::Method::*;
use iron::modifiers::Redirect;
use iron::prelude::*;
use iron::Url;
use iron::status;
use mount::Mount;
use router::Router;
use unicase::UniCase;
use urlshortener::errors;
use urlshortener::models;
use urlshortener::UrlManager;

macro_rules! try_or_500 {
    ($expr:expr) => (match $expr {
        Ok(val) => val,
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)))
        }
    })
}

fn main() {
    let mut router = Router::new();
    router.get("/", redirect_to_home);
    router.get("/ping", pong);
    router.get("/:alias", redirect_to_alias);

    let mut api_router = Router::new();
    api_router.get("/", pong);
    api_router.get("/ping", pong);
    api_router.get("/url", pong);
    api_router.get("/url/ping", pong);
    api_router.get("/url/:alias", get_url);
    api_router.post("/url", shorten_url);

    let mut mount = Mount::new();
    mount.mount("/", router);
    mount.mount("/api/", api_router);

    let mut chain = Chain::new(mount);
    chain.link_after(CORS);
    println!("Urlshortener rest services running at http://localhost:3000");
    Iron::new(chain).http("0.0.0.0:3000").unwrap();
}

fn respond_json(value: String) -> IronResult<Response> {
    let content_type = "application/json".parse::<Mime>().unwrap();
    Ok(Response::with((content_type, iron::status::Ok, value)))
}

fn redirect_to_home(_req: &mut Request) -> IronResult<Response> {
    let homepage_url_str = "http://tsaju.in/urlshortener";
    let homepage_url = Url::parse(homepage_url_str).unwrap();
    Ok(Response::with((status::MovedPermanently, Redirect(homepage_url.clone()))))
}

fn redirect_to_alias(req: &mut Request) -> IronResult<Response> {
    let alias = req.extensions.get::<Router>().unwrap().find("alias").unwrap();
    let long_url = UrlManager::new().find_one(alias.to_string()).unwrap().long_url;
    let url_str: &str = &*long_url;
    let url_result = Url::parse(url_str);
    if let Ok(url) = url_result
    {
        Ok(Response::with((status::MovedPermanently, Redirect(url.clone()))))
    }
    else
    {
        // Try to handle if long url do not start with http / https
        let new_long_url = "http://".to_string() + &long_url;
        let new_long_url_str: &str = &*new_long_url;
        let new_url = Url::parse(new_long_url_str).unwrap();
        Ok(Response::with((status::MovedPermanently, Redirect(new_url.clone()))))
    }
}

fn pong(_: &mut Request) -> IronResult<Response> {
    let pong = Pong { message: Some("pong".to_string()) };
    let serialized = serde_json::to_string(&pong).unwrap();
    respond_json(serialized)
}

fn get_url(req: &mut Request) -> IronResult<Response> {
    let alias = req.extensions.get::<Router>().unwrap().find("alias").unwrap_or("");
    let find = UrlManager::new().find_one(alias.to_string());
    
    match find {
        Some(url) => {
            let serialized = serde_json::to_string(&url).unwrap();
            respond_json(serialized)
        },
        None => {
            Ok(Response::with(status::NotFound))
        }
    }

}

fn shorten_url(req: &mut Request) -> IronResult<Response> {
    //let mut buffer = String::new();
    //let size = req.body.read_to_string(&mut buffer);
    //req.body.read_to_string(&mut buffer).unwrap();
    let body = req.get::<bodyparser::Raw>().unwrap().unwrap();
    //println!("{:?}", body);

    let url: models::Url = try_or_500!(serde_json::from_str(&body));
    let url_manager = UrlManager::new();
    let created = url_manager.add(url);

    match created {
        Ok(v) =>{
            let serialized = serde_json::to_string(&v).unwrap();
            return respond_json(serialized);
        },
        Err(e) => {
            match e {
                errors::UrlError::AliasAlreadyExists => {
                    return Ok(Response::with((status::Conflict)))
                },
                errors::UrlError::OtherError => {
                    return Ok(Response::with((status::InternalServerError)))
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Point {
    x: i32,
//    #[serde(rename="xx")]
    y: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Pong {
    message: Option<String>,
}


struct CORS;

impl AfterMiddleware for CORS {
    fn after(&self, _: &mut Request, mut res: Response) -> IronResult<Response> {
        res.headers.set(headers::AccessControlAllowOrigin::Any);
        res.headers.set(headers::AccessControlAllowHeaders(
                vec![UniCase("accept".to_string()),
                UniCase("content-type".to_string())]));
        res.headers.set(headers::AccessControlAllowMethods(
                vec![Get,Head,Post,Delete,Options,Put,Patch]));
        Ok(res)
    }
}
