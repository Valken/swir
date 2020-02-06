use std::collections::HashMap;
use std::sync::Arc;

use futures::channel::oneshot;
use futures::lock::Mutex;
use futures::stream::StreamExt;
use http::HeaderValue;
use hyper::client::connect::dns::GaiResolver;
use hyper::client::HttpConnector;
use hyper::{header, Body, Client, HeaderMap, Method, Request, Response, StatusCode};
use tokio::sync::mpsc;
use rand::{Rng, SeedableRng};
use rand::rngs::SmallRng;
use base64;

use crate::utils::structs::{BackendStatusCodes, ClientSubscribeRequest, CustomerInterfaceType, Job, MessagingResult, PublishRequest, RestToMessagingContext};
use crate::utils::structs::{MessagingToRestContext, SubscribeRequest};

fn extract_topic_from_headers(headers: &HeaderMap<HeaderValue>) -> String {
    let topic_header = header::HeaderName::from_lowercase(b"topic").unwrap();
    let maybe_topic_header = headers.get(topic_header);
    if let Some(topic) = maybe_topic_header {
        String::from_utf8_lossy(topic.as_bytes()).to_string()
    } else {
        "".to_string()
    }
}

fn find_channel_by_topic<'a>(
    client_topic: &'a String,
    from_client_to_backend_channel_sender: &'a Box<HashMap<String, Box<mpsc::Sender<RestToMessagingContext>>>>,
) -> Option<&'a Box<mpsc::Sender<RestToMessagingContext>>> {
    from_client_to_backend_channel_sender.get(client_topic)
}

fn find_to_client_sender<'a>(client_topic: &'a String,to_client_sender: &'a Box<HashMap<String, Box<mpsc::Sender<MessagingToRestContext>>>>)->Option<&'a Box<mpsc::Sender<MessagingToRestContext>>>
{
    to_client_sender.get(client_topic)
    
}

fn validate_content_type(headers: &HeaderMap<HeaderValue>) -> Option<bool> {
    match headers.get(http::header::CONTENT_TYPE) {
        Some(header) => {
            if header == HeaderValue::from_static("application/json") {
                return Some(true);
            } else {
                return None;
            }
        }
        None => return None,
    }
}

fn set_http_response(backend_status: BackendStatusCodes, response: &mut Response<Body>) {
    match backend_status {
        BackendStatusCodes::Ok(msg) => {
            *response.status_mut() = StatusCode::OK;
            *response.body_mut() = Body::from(msg);
        }
        BackendStatusCodes::Error(msg) => {
            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            *response.body_mut() = Body::from(msg);
        }
        BackendStatusCodes::NoTopic(msg) => {
            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            *response.body_mut() = Body::from(msg);
        }
    }
}

async fn get_whole_body(mut req: Request<Body>) -> Vec<u8> {
    let mut whole_body = Vec::new();
    while let Some(maybe_chunk) = req.body_mut().next().await {
        if let Ok(chunk) = &maybe_chunk {
            whole_body.extend_from_slice(chunk);
        }
    }
    whole_body
}

pub async fn handler(req: Request<Body>, from_client_to_backend_channel_sender: Box<HashMap<String, Box<mpsc::Sender<RestToMessagingContext>>>>,to_client_sender_for_rest:Box<HashMap<String, Box<mpsc::Sender<MessagingToRestContext>>>>) -> Result<Response<Body>, hyper::Error> {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NOT_ACCEPTABLE;

    let headers = req.headers().clone();
    debug!("Headers {:?}", headers);
    debug!("Body {:?}", req.body());

    match (req.method(), req.uri().path()) {
        (&Method::POST, "/publish") => {
            let whole_body = get_whole_body(req).await;
            let wb = whole_body.clone();
            let wb = String::from_utf8_lossy(&wb);
            info!("Publish start {}", wb);
            let client_topic = extract_topic_from_headers(&headers);
            let maybe_channel = find_channel_by_topic(&client_topic, &from_client_to_backend_channel_sender);
            let mut sender = if let Some(channel) = maybe_channel {
                channel.clone()
            } else {
                set_http_response(BackendStatusCodes::NoTopic("No mapping for this topic".to_string()), &mut response);
                return Ok(response);
            };

            let p = PublishRequest {
                payload: whole_body,
                client_topic: client_topic,
            };

            let (local_tx, local_rx): (oneshot::Sender<MessagingResult>, oneshot::Receiver<MessagingResult>) = oneshot::channel();
            let job = RestToMessagingContext {
                job: Job::Publish(p),
                sender: local_tx,
            };

            if let Err(e) = sender.try_send(job) {
                warn!("Channel is dead {:?}", e);
                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                *response.body_mut() = Body::empty();
            }

            debug!("Waiting for broker");
            let response_from_broker: Result<MessagingResult, oneshot::Canceled> = local_rx.await;
            debug!("Got result {:?}", response_from_broker);
            if let Ok(res) = response_from_broker {
                set_http_response(res.status, &mut response);
            } else {
                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                *response.body_mut() = Body::empty();
            }
            info!("Publish end {}", wb);
            return Ok(response);
        }

        (&Method::POST, "/subscribe") => {
            if validate_content_type(&headers).is_none() {
                return Ok(response);
            }

            let whole_body = get_whole_body(req).await;
            let maybe_json = serde_json::from_slice(&whole_body);
            match maybe_json {
                Ok(json) => {
                    let json: ClientSubscribeRequest = json;
                    info!("{:?}", json);
                    let (local_tx, local_rx): (oneshot::Sender<MessagingResult>, oneshot::Receiver<MessagingResult>) = oneshot::channel();
		    let client_topic =  json.client_topic.clone();

		    if let Some(to_client_sender) = find_to_client_sender(&client_topic,&to_client_sender_for_rest){

			let mut small_rng = SmallRng::from_entropy();
			let mut array: [u8; 32]=[0;32];
			small_rng.fill(&mut array);
			let client_id = base64::encode(&array);	
			let mut endpoint = json.endpoint.clone();
			endpoint.client_id = client_id;
			let sb = SubscribeRequest {
                            client_interface_type: CustomerInterfaceType::REST,
                            client_topic: json.client_topic.clone(),
                            endpoint: endpoint,
			    tx:to_client_sender.clone()
			};

			let maybe_channel = find_channel_by_topic(&sb.client_topic, &from_client_to_backend_channel_sender);

			let mut sender = if let Some(channel) = maybe_channel {
                            channel.clone()
			} else {
                            set_http_response(BackendStatusCodes::NoTopic("No channel for this topic".to_string()), &mut response);
                            return Ok(response);
			};

			let job = RestToMessagingContext {
                            job: Job::Subscribe(sb),
                            sender: local_tx,
			};

			if let Err(e) = sender.try_send(job) {
                            warn!("Channel is dead {:?}", e);
                            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                            *response.body_mut() = Body::empty();
			}

			let response_from_broker: Result<MessagingResult, oneshot::Canceled> = local_rx.await;
			debug!("Got result {:?}", response_from_broker);
			if let Ok(res) = response_from_broker {
                            set_http_response(res.status, &mut response);
			} else {
                            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                            *response.body_mut() = Body::empty();
			}
		    }else{
			set_http_response(BackendStatusCodes::NoTopic("No mapping for this topic".to_string()), &mut response);			
		    }
		    
                }
                Err(e) => {
                    warn!("{:?}", e);
                    *response.status_mut() = StatusCode::BAD_REQUEST;
                    *response.body_mut() = Body::from(e.to_string());
                }
            }
            Ok(response)
        }

        // The 404 Not Found route...
        _ => {
            let not_found = Response::default();
            *response.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

async fn send_request(client: Client<HttpConnector<GaiResolver>>, payload: MessagingToRestContext) {
    let uri = payload.uri;
    let uri = uri.parse::<hyper::Uri>().unwrap();

    let p = payload.payload.clone();
    let req = Request::builder()
        .method("POST")
        .uri(uri)
        .header(hyper::header::CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"))
        .body(Body::from(payload.payload))
        .expect("request builder");

    //    let sender = payload.sender.clone();
    //    let err_sender = payload.sender.clone();

    let p = String::from_utf8_lossy(&p);
    info!("Making request for {}", p);
    let resp = client.request(req).await;
    let _res = resp
        .map(move |res| {
            info!("Status POST to the client: {} {} ", res.status(), p);
            //                let mut status = "All good".to_string();
            if res.status() != hyper::StatusCode::OK {
                warn!("Error from the client {}", res.status());
                //                    status = "Invalid response from the client".to_string();
            }
            //                if let Err(e) = sender.send(MessagingResult { status: u32::from(res.status().as_u16()), result: status }) {
            //                    warn!("Problem with an internal communication {:?}", e);
            //                }
            //                res.into_body().concat2()
            res.into_body()
        })
        .map(|_| {})
        .map_err(move |err| {
            eprintln!("Error {}", err);
            //                    if let Err(e) = err_sender.send(MessagingResult { status: 1, result: "Something is wrong".to_string() }) {
            //                        warn!("Problem with an internal communication {:?}", e);
            //                    }
        });
}

pub async fn client_handler(rx: Arc<Mutex<mpsc::Receiver<MessagingToRestContext>>>) {
    let client = hyper::Client::builder().keep_alive(true).build_http();
    info!("Client done");
    let mut rx = rx.lock().await;
    while let Some(payload) = rx.next().await {
        let client = client.clone();
        tokio::spawn(async move { send_request(client, payload).await });
    }
}
