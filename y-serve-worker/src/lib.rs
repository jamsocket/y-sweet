use worker::{durable_object, event, Env, Request, Response, Result, Router};

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    let router = Router::new();

    let response = router
        .get("/", |_, _| Response::ok("Hello world!"))
        .run(req, env)
        .await?;

    Ok(response)
}

#[durable_object]
pub struct YServe {}

#[durable_object]
impl DurableObject for YServe {
    fn new(state: State, _env: Env) -> Self {
        Self {}
    }

    async fn fetch(&mut self, req: Request) -> Result<Response> {
        let url = req.url()?;
        let (_, path) = url.path().rsplit_once('/').unwrap_or_default();
        let method = req.method();
        match (method, path) {
            _ => Response::error("Document command not found", 404),
        }
    }
}
