use prometheus_client::encoding::{text::encode, EncodeLabelSet};
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::registry::Registry;

use std::sync::Arc;

use tide::{Middleware, Next, Request, Result};

#[async_std::main]
async fn main() -> std::result::Result<(), std::io::Error> {
    tide::log::start();

    let mut registry = Registry::default();
    let http_requests_total = Family::<Labels, Counter>::default();
    registry.register(
        "http_requests_total",
        "Number of HTTP requests",
        http_requests_total.clone(),
    );

    let middleware = MetricsMiddleware {
        http_requests_total,
    };
    let mut app = tide::with_state(State {
        registry: Arc::new(registry),
    });

    app.with(middleware);
    app.at("/").get(|_| async {
        let body = "<html>
            <head><title>TC4400 Oxide Metrics</title></head>
            <body>
            <p><a href='/metrics'>Metrics</a></p>
            </body>
            </html>";
        let response = tide::Response::builder(200)
            .body(body)
            .content_type("text/html")
            .build();
        Ok(response)
    });
    app.at("/metrics")
        .get(|req: tide::Request<State>| async move {
            let mut encoded = String::new();
            encode(&mut encoded, &req.state().registry)?;
            let response = tide::Response::builder(200)
                .body(encoded)
                .content_type("application/openmetrics-text; version=1.0.0; charset=utf-8")
                .build();
            Ok(response)
        });
    app.listen("127.0.0.1:8080").await?;

    Ok(())
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct Labels {
    method: String,
    path: String,
}

#[derive(Clone)]
struct State {
    registry: Arc<Registry>,
}

#[derive(Default)]
struct MetricsMiddleware {
    http_requests_total: Family<Labels, Counter>,
}

#[tide::utils::async_trait]
impl Middleware<State> for MetricsMiddleware {
    async fn handle(&self, request: Request<State>, next: Next<'_, State>) -> Result {
        // let method = match request.method() {
        //     http::Method::Get => Method::Get,
        //     http::Method::Put => Method::Put,
        //     http::Method::Post => Method::Post,
        //     _ => Method::Other,
        // };
        let path = request.url().path().to_string();
        self.http_requests_total
            .get_or_create(&Labels {
                method: request.method().to_string(),
                path,
            })
            .inc();

        let response = next.run(request).await;
        Ok(response)
    }
}
