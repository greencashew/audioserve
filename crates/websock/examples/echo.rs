#[macro_use]
extern crate log;
extern crate websock as ws;
use futures::future;
use hyper::server::Server;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response};
use std::convert::Infallible;

type GenericError = Box<dyn std::error::Error + Send + Sync>;

/// Our server HTTP handler to initiate HTTP upgrades.
async fn server_upgrade(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    debug!("We got these headers: {:?}", req.headers());

    Ok(ws::spawn_websocket(req, |m| {
        debug!("Got message {:?}", m);
        let counter: u64 = {
            let mut c = m.context_ref().write().unwrap();
            *c = *c + 1;
            *c
        };

        Box::new(future::ok(Some(ws::Message::text(
            format!("{}: {}", counter, m.to_str().unwrap()),
            m.context(),
        ))))
    }))
}
#[tokio::main]
async fn main() -> Result<(), GenericError> {
    pretty_env_logger::init();
    let addr = ([127, 0, 0, 1], 5000).into();
    let service = make_service_fn(|_| async { Ok::<_, Infallible>(service_fn(server_upgrade)) });
    let server = Server::bind(&addr).serve(service);
    info!("Serving on {}", addr);
    server.await?;

    Ok(())
}
