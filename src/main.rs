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
use urlshortener::dal::repository::UrlRepository;
use urlshortener::models;

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
    router.get("/:alias", redirect_to_alias);

    let mut api_router = Router::new();
    api_router.get("/url", hello_world);
    api_router.get("/url/:alias", get_url);
    api_router.post("/url", shorten_url);

    let mut mount = Mount::new();
    mount.mount("/", router);
    mount.mount("/api/", api_router);

    let mut chain = Chain::new(mount);
    chain.link_after(CORS);
    Iron::new(chain).http("localhost:3000").unwrap();
}

fn respond_json(value: String) -> IronResult<Response> {
    let content_type = "application/json".parse::<Mime>().unwrap();
    Ok(Response::with((content_type, iron::status::Ok, value)))
}

fn redirect_to_alias(req: &mut Request) -> IronResult<Response> {
    let alias = req.extensions.get::<Router>().unwrap().find("alias").unwrap();
    let long_url = UrlRepository::new().find_one(alias.to_string()).unwrap().long_url;
    let urlstr: &str = &*long_url;
    let url = Url::parse(urlstr).unwrap();
    Ok(Response::with((status::MovedPermanently, Redirect(url.clone()))))
}

fn hello_world(_: &mut Request) -> IronResult<Response> {
    let point = Point { x: 1, y: Some("saju".to_string()) };
    let serialized = serde_json::to_string(&point).unwrap();
    respond_json(serialized)
}

fn get_url(req: &mut Request) -> IronResult<Response> {
    let alias = req.extensions.get::<Router>().unwrap().find("alias").unwrap_or("");
    let find = UrlRepository::new().find_one(alias.to_string());
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
    let url_repository = UrlRepository::new();
    let created = url_repository.add(url);

    match created {
        Ok(v) =>{
            let serialized = serde_json::to_string(&v).unwrap();
            return respond_json(serialized);
        },
        Err(e) => {
            if e == "Alias already exists." {
                return Ok(Response::with((status::Conflict)))
            }
            else {
                return Ok(Response::with((status::InternalServerError)))
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
