extern crate reqwest;
extern crate serde;
extern crate qrcode;
extern crate image;
extern crate base64;
extern crate uuid;

extern crate bankid_rs;

use qrcode::QrCode;
use image::Luma;
use image::png::PNGEncoder;
use std::io::Write;

use serde::Serialize;
use serde::Deserialize;
use uuid::Uuid;

use reqwest::header;
use std::fs::File;
use std::io::Read;
use std::collections::HashMap;
use std::borrow::Cow;

use tokio::timer::delay;
use std::time::Duration;

use hyper::{Body, Request, Response, Server, Method, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use url::Url;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

// mod aws;

#[derive(Debug, Serialize, Deserialize)]
struct Post {
    id: Option<i32>,
    title: String,
    body: String,
    #[serde(rename = "userId")]
    user_id: i32,
}

static INTERNAL_SERVER_ERROR: &[u8] = b"Internal Server Error";
static NOT_FOUND_ERROR: &[u8] = b"Not found";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadReqResponse {
    pub file_id: String,
    pub upload_url: String
}

/*
async fn svc_upload_req(req: Request<Body>, aws : &aws::Aws) -> Result<Response<Body>> {
    let file_id = Uuid::new_v4().to_hyphenated().to_string();
    let filename = format!("{}/{}", String::from("uploads"), file_id);
    let json = UploadReqResponse{
        upload_url: aws.put_s3_signed_url(&filename),
        file_id: file_id
    };
    let res = match serde_json::to_string(&json) {
        Ok(json) => {
            Response::builder()
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json))
                .unwrap()
        }
        Err(_) => {
            Response::builder()
                .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
                .body(INTERNAL_SERVER_ERROR.into())
                .unwrap()
        }
    };
    Ok(res)
}
*/

async fn svc_not_found(_req: Request<Body>) -> Result<Response<Body>> {
    let res =  Response::builder()
        .status(hyper::StatusCode::NOT_FOUND)
        .body(NOT_FOUND_ERROR.into()).unwrap();
    Ok(res)
}

async fn svc_auth_req(_req: Request<Body>, client: reqwest::Client) -> Result<Response<Body>> {
    let resp = bankid_rs::auth(client.clone(), "https://appapi2.test.bankid.com/rp/v5", None, "46.39.99.238").await?;
    let order_ref = resp.order_ref.clone();
    let auto_start_token = resp.auto_start_token.clone();


    let res =  match auto_start_token {
        Some(token) => {
            let mut qr_uri = String::from("bankid:///?autostarttoken=");
            qr_uri.push_str(&token);
            let qrcode_base68 = bankid_rs::qr_code_png(&qr_uri).await;

            //let collect = poll_collect_until_completed(client.clone(), end_point, &order_ref).await?;
            match qrcode_base68 {
                Some(code) => { 
                    println!("Genereted code {}", code);
                    Response::builder()
                        .header(header::CONTENT_TYPE, "text/html")
                        .body(Body::from(format!("<html><body><img src=\"data:image/png;base64,{}\"><a href=\"collect?orderRef={}\">Collect response</a></body></html>", code, resp.order_ref))).unwrap()
                },
                None => {
                    println!("Error none");
                    Response::builder()
                        .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
                        .body(INTERNAL_SERVER_ERROR.into()).unwrap()
                }
            } 
        },
        None => {
            Response::builder()
                        .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
                        .body(INTERNAL_SERVER_ERROR.into()).unwrap()
        }
    };
    
    Ok(res)
}

async fn svc_sign_req(req: Request<Body>, client: reqwest::Client) -> Result<Response<Body>> {
    
    let (visible, invisible) = match req.uri().query() {
        Some (query) => {
            let mut userVisibleData : Option<String> = None;
            let mut userNonVisibleData : Option<String> = None;

            let query_params = querystring::querify(query);
            for param in query_params {
                match param.0 {
                    "userVisibleData" => userVisibleData = Some(param.1.to_string()),
                    "userNonVisibleData" => userNonVisibleData = Some(param.1.to_string()),
                    _ => ()
                };
            }
            (userVisibleData, userNonVisibleData)
        },
        None => (None, None)
    };

    println!("parsed visible {}", visible.clone().unwrap());
    let resp = bankid_rs::sign(client.clone(), "https://appapi2.test.bankid.com/rp/v5", None, "46.39.99.238", &visible.unwrap(), invisible).await?;
    let order_ref = resp.order_ref.clone();
    let auto_start_token = resp.auto_start_token.clone();

    println!("sign order_ref {}", order_ref);
    let res =  match auto_start_token {
        Some(token) => {
            println!("sign order_ref {} token {}", order_ref, token);

            let mut qr_uri = String::from("bankid:///?autostarttoken=");
            qr_uri.push_str(&token);
            let qrcode_base68 = bankid_rs::qr_code_png(&qr_uri).await;

            //let collect = poll_collect_until_completed(client.clone(), end_point, &order_ref).await?;
            match qrcode_base68 {
                Some(code) => { 
                    println!("Genereted code {}", code);
                    Response::builder()
                        .header(header::CONTENT_TYPE, "text/html")
                        .body(Body::from(format!("<html><body><img src=\"data:image/png;base64,{}\"><a href=\"collect?orderRef={}\">Collect response</a></body></html>", code, resp.order_ref))).unwrap()
                },
                None => {
                    println!("Error none");
                    Response::builder()
                        .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
                        .body(INTERNAL_SERVER_ERROR.into()).unwrap()
                }
            } 
        },
        None => {
            Response::builder()
                        .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
                        .body(INTERNAL_SERVER_ERROR.into()).unwrap()
        }
    };
    
    Ok(res)
}

async fn svc_collect(req: Request<Body>, client: reqwest::Client) -> Result<Response<Body>> {
    
    let order_ref = match req.uri().query() {
        Some (query) => {
            let mut order_ref : Option<String> = None;

            let query_params = querystring::querify(query);
            for param in query_params {
                match param.0 {
                    "orderRef" => order_ref = Some(param.1.to_string()),
                    _ => ()
                };
            }
            order_ref
        },
        None => None
    };

    let collected_response = bankid_rs::collect(client.clone(), "https://appapi2.test.bankid.com/rp/v5", &order_ref.unwrap()).await?;
    
    let res = match serde_json::to_string(&collected_response) {
        Ok(json) => {
            Response::builder()
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json))
                .unwrap()
        }
        Err(_) => {
            Response::builder()
                .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
                .body(INTERNAL_SERVER_ERROR.into())
                .unwrap()
        }
    };

    Ok(res)
}



async fn svc_insignia(req: Request<Body>, client: reqwest::Client) -> Result<Response<Body>> {// , aws: aws::Aws) -> Result<Response<Body>> {
    let res : Result<Response<Body>> = match (req.method(), req.uri().path()) {
        
        (&Method::GET, "/auth") => {
            svc_auth_req(req, client.to_owned()).await
        },
        // (&Method::GET, "/upload") => {
        //     svc_upload_req(req, &aws).await
        // },
        (&Method::GET, "/sign") => {
            svc_sign_req(req, client.to_owned()).await
        },
        (&Method::GET, "/collect") => {
            svc_collect(req, client.to_owned()).await
        },
        _ => {
            svc_not_found(req).await
        } 
    };
    res
}

static NOTFOUND: &[u8] = b"Not Found";

#[tokio::main]
pub async fn main() -> Result<()> {
    pretty_env_logger::init();

    // For every connection, we must make a `Service` to handle all
    // incoming HTTP requests on said connection.

    let end_point = "https://appapi2.test.bankid.com/rp/v5";
    let server_cert_filename =  "cert/test/bankid.crt";
    let client_cert_filename =  "cert/test/FPTestcert2_20150818_102329.pfx";
    let client_cert_password =  "qwerty123";

    // read server certificate
    let mut buf = Vec::new();
    File::open(server_cert_filename)?.read_to_end(&mut buf)?;
    let cert = reqwest::Certificate::from_pem(&buf)?;

    // read client certificate
    let mut buf = Vec::new();
    File::open(client_cert_filename)?
        .read_to_end(&mut buf)?;
    let pkcs12 = reqwest::Identity::from_pkcs12_der(&buf, client_cert_password)?;

    let client = reqwest::Client::builder()
        .identity(pkcs12)
        .add_root_certificate(cert)
//        .gzip(true)
        .timeout(Duration::from_secs(10))
        .build()?;

    // let aws = aws::Aws::new();

    let make_svc = make_service_fn(move |_| {
        // This is the `Service` that will handle the connection.
        // `service_fn` is a helper to convert a function that
        // returns a Response into a `Service`.
        let client = client.clone();
        //let aws = aws.clone();

        async {
            //Ok::<_, Infallible>(service_fn(svc_auth_req))

            Ok::<_, GenericError>(service_fn(move |req| {
                svc_insignia(req, client.to_owned()) // , aws.to_owned())
            }))
        }
    });

    let addr = ([0, 0, 0, 0], 3001).into();   

    let server = Server::bind(&addr).serve(make_svc);

    println!("Listening on http://{}", addr);

    server.await?;

    Ok(())
}

